// Copyright 2022 VMware, Inc.
// SPDX-License-Identifier: BSD-2-Clause

use std::collections::{HashMap, HashSet};

use crate::{
    fly::{
        semantics::{Interpretation, Model, Universe},
        syntax::{Signature, Sort, Term},
    },
    smtlib::{
        proc::{SatResp, SmtProc, SolverCmd, SolverError},
        sexp::{app, atom_i, atom_s, sexp_l, Atom, Sexp},
    },
    solver::sexp,
};

/// A [`Solver`] requires a Backend, which specifies how to start a particular
/// solver process and how to parse its models (this is not part of SMT-LIB and
/// thus there are meaningful differences in the format of how solvers respond
/// to `(get-model)`).
pub trait Backend {
    fn get_cmd(&self) -> SolverCmd;

    fn parse(
        &self,
        sig: &Signature,
        n_states: usize,
        indicators: &HashSet<String>,
        model: &Sexp,
    ) -> FOModel;
}

/// An FOModel ("first-order model") gives a cardinality to each universe and an
/// interpretation to each symbol (including primed versions). This is intended
/// to be an easy-to-construct representation of a trace involving multiple
/// states, parseable from an SMT solver's counter example to a single query.
pub struct FOModel {
    pub universe: HashMap<String, usize>,
    pub interp: HashMap<String, Interpretation>,
}

/// A Solver provides an interface to a running SMT solver, allowing interaction
/// with it using [`fly::syntax::Term`]'s.
///
/// The Backend makes it possible to parse and return models in the compact
/// representation of `semantics::Model`.
pub struct Solver<B> {
    proc: SmtProc,
    signature: Signature,
    n_states: usize,
    asserts: Vec<Term>,
    indicators: HashSet<String>,
    backend: B,
}

impl<B: Backend> Solver<B> {
    /// Start a Solver for a particular signature and backend.
    ///
    /// The `tee` argument causes the SMT2 output sent to the solver to also be
    /// sent to a file, for debugging purposes.
    pub fn new(
        signature: &Signature,
        n_states: usize,
        backend: B,
        tee: Option<String>,
    ) -> Result<Self, SolverError> {
        let signature = signature.clone();
        let mut proc = SmtProc::new(backend.get_cmd())?;
        if let Some(p) = tee {
            proc.tee(p)?;
        }
        Self::send_signature(&mut proc, &signature, n_states);
        Ok(Self {
            proc,
            signature,
            n_states,
            asserts: vec![],
            indicators: HashSet::new(),
            backend,
        })
    }

    /// Emit encoding of signature, using `n_states` to determine how many times
    /// to emit each mutable symbol.
    fn send_signature(proc: &mut SmtProc, sig: &Signature, n_states: usize) {
        for sort in &sig.sorts {
            proc.send(&app("declare-sort", [sexp::sort(sort), atom_i(0)]));
        }
        for r in &sig.relations {
            // immutable symbols are always declared once
            if !r.mutable {
                proc.send(&app(
                    "declare-fun",
                    [
                        atom_s(&r.name),
                        sexp_l(r.args.iter().map(sexp::sort)),
                        sexp::sort(&r.typ),
                    ],
                ));
            }
            // mutable symbols are declared according to n_states (or not at all
            // if n_states=0)
            if r.mutable {
                for n_primes in 0..n_states {
                    let name = &r.name;
                    proc.send(&app(
                        "declare-fun",
                        [
                            atom_s(format!("{name}{}", "'".repeat(n_primes))),
                            sexp_l(r.args.iter().map(sexp::sort)),
                            sexp::sort(&r.typ),
                        ],
                    ));
                }
            }
        }
    }

    pub fn assert(&mut self, t: &Term) {
        self.proc.send(&app("assert", [sexp::term(t)]));
        self.asserts.push(t.clone())
    }

    pub fn comment_with<F>(&mut self, comment: F)
    where
        F: FnOnce() -> String,
    {
        self.proc.comment_with(comment)
    }

    /// Get an indicator variable uniquely determined by `name`.
    pub fn get_indicator(&mut self, name: &str) -> Term {
        let ind = format!("@ind@{name}");
        self.indicators.insert(ind.clone());
        Term::Id(ind)
    }

    /// The `assumptions` map should map indicator variables to whether they
    /// should be assumed true or false.
    pub fn check_sat(
        &mut self,
        assumptions: HashMap<Term, bool>,
    ) -> Result<SatResp<()>, SolverError> {
        if cfg!(debug_assertions) {
            for assumption in assumptions.keys() {
                assert!(
                    if let Term::Id(s) = assumption {
                        self.indicators.contains(s)
                    } else {
                        false
                    },
                    "assumption {assumption} is not an indicator variable"
                );
            }
        }
        if assumptions.is_empty() {
            let sat = self.proc.check_sat()?;
            Ok(sat)
        } else {
            let assumptions = assumptions
                .into_iter()
                .map(|(ind, set_true)| {
                    if set_true {
                        sexp::term(&ind)
                    } else {
                        sexp::negated_term(&ind)
                    }
                })
                .collect::<Vec<_>>();
            let sat = self.proc.check_sat_assuming(&assumptions)?;
            Ok(sat)
        }
    }

    /// After a sat response to check_sat or check_sat_assuming, produce a trace
    /// of models, one per state. Each model interprets all of the symbols in
    /// the signature.
    pub fn get_model(&mut self) -> Vec<Model> {
        let model = self
            .proc
            .send_with_reply(&app("get-model", []))
            .expect("could not get model");
        let fo_model = self
            .backend
            .parse(&self.signature, self.n_states, &self.indicators, &model);
        fo_model.into_trace(&self.signature, self.n_states)
    }

    pub fn get_minimal_model(&mut self) -> Vec<Model> {
        todo!()
    }

    /// Returns an unsat core as a set of indicator variables (a subset of the
    /// assumptions passed to `check_sat`).
    pub fn get_unsat_core(&mut self) -> HashMap<Term, bool> {
        let indicators = self
            .proc
            .get_unsat_assumptions()
            .expect("could not get unsat assumptions");
        let mut assumptions = HashMap::new();
        for t in indicators {
            // TODO: this is very ugly, replace with Sexp destructor methods
            // (even at the expense of precise error reporting)
            match t {
                Sexp::Atom(Atom::S(s)) => {
                    assumptions.insert(Term::Id(s), true);
                }
                Sexp::List(ss) => {
                    // should be (not i)
                    assert!(
                        ss.len() == 2 && (ss[0] == atom_s("not") || ss[0] == atom_s("!")),
                        "malformed unsat assumption {} in solver response",
                        &Sexp::List(ss),
                    );
                    if let Sexp::Atom(Atom::S(s)) = ss[1].clone() {
                        assumptions.insert(Term::Id(s), false);
                    } else {
                        panic!("non-string unsat assumption {} in solver response", &ss[1]);
                    }
                }
                Sexp::Comment(_) => continue,
                _ => panic!("non-string unsat assumption {} in solver response", &t),
            }
        }
        assumptions
    }

    pub fn get_minimal_unsat_core(&mut self) -> Vec<Term> {
        todo!()
    }

    pub fn push(&mut self) {
        self.proc.send(&app("push", []));
    }

    pub fn pop(&mut self) {
        self.proc.send(&app("pop", []));
    }
}

impl FOModel {
    fn into_trace(self, signature: &Signature, n_states: usize) -> Vec<Model> {
        let universe: Universe = signature
            .sorts
            .iter()
            .map(|s| {
                if let Sort::Id(s) = s {
                    self.universe[s]
                } else {
                    panic!("unexpected sort in signature")
                }
            })
            .collect();
        let mut states: Vec<Model> = vec![];
        for n in 0..n_states {
            let interp = signature
                .relations
                .iter()
                .map(|r| {
                    let relation = format!("{r}{primes}", r = &r.name, primes = "'".repeat(n));
                    self.interp[&relation].clone()
                })
                .collect::<Vec<_>>();
            let model = Model {
                signature: signature.clone(),
                universe: universe.clone(),
                interp,
            };
            states.push(model);
        }
        states
    }
}
