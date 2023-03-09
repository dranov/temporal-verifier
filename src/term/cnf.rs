// Copyright 2022-2023 VMware, Inc.
// SPDX-License-Identifier: BSD-2-Clause

//! Convert terms to CNF.
//!
//! See the documentation for [`Cnf`] for details.

// turned out to not be needed (yet)
#![allow(dead_code)]

use crate::fly::{
    printer, syntax,
    syntax::{NOp, Term, UOp},
};

use crate::fly::syntax::Term::{App, Id, Quantified};
use NOp::And;
use Term::{BinOp, Literal, NAryOp, UnaryOp};
use UOp::Always;

/// Conjunctive normal form terms.
///
/// These terms have the shape (p1 & ... & pn) or (always (p1 & ... & pn)),
/// where the terms are not conjunctions.
///
/// Creates a single conjunction so that the root is always NAryOp(And, _) or
/// UnaryOp(Always, NAryOp(And, _)).
///
/// Does not recurse into forall and exists and normalize there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cnf(pub Term);

fn not_and(t: &Term) {
    assert!(
        !matches!(t, NAryOp(And, _)),
        "contains a conjunction {}",
        printer::term(t)
    );
}

fn flat_disjunction(t: &Term) {
    match t {
        NAryOp(And, terms) => {
            for t in terms {
                not_and(t);
            }
        }
        _ => panic!("{} is not a conjunction", printer::term(t)),
    }
}

/// The well-formedness predicate for [`Cnf`].
fn cnf(t: &Term) {
    match t {
        NAryOp(_, _) => flat_disjunction(t),
        UnaryOp(Always, t) => flat_disjunction(t),
        _ => panic!("{} is not an always or conjunction", printer::term(t)),
    }
}

/// If t is an always, get its body. Collapses consecutive always.
fn get_always(t: &Term) -> Option<Term> {
    match t {
        UnaryOp(Always, t) => Some(get_always(t).unwrap_or_else(|| *t.clone())),
        _ => None,
    }
}

fn conjuncts(t: Term) -> Vec<Term> {
    match t {
        NAryOp(And, ts) => ts.into_iter().flat_map(conjuncts).collect(),
        _ => vec![t],
    }
}

fn cartesian_product(v: &[Vec<Term>]) -> Vec<Vec<Term>> {
    if v.is_empty() {
        return vec![vec![]];
    }

    let mut result: Vec<Vec<Term>> = vec![];

    for i in &v[0] {
        for rest in cartesian_product(&v[1..]) {
            let mut prod = vec![i.clone()];
            prod.extend(rest);
            result.push(prod);
        }
    }

    result
}

fn body_to_clauses(t: Term, is_negated: bool) -> Vec<Term> {
    match t {
        Literal(b) => vec![Term::Literal(b ^ !is_negated)],
        UnaryOp(UOp::Not, t) => body_to_clauses(*t.clone(), !is_negated),
        UnaryOp(.., _) => panic!("got UnaryOp other than Not!"),
        BinOp(syntax::BinOp::NotEquals, lhs, rhs) => {
            if is_negated {
                vec![BinOp(syntax::BinOp::Equals, lhs.clone(), rhs.clone())]
            } else {
                vec![BinOp(syntax::BinOp::NotEquals, lhs.clone(), rhs.clone())]
            }
        }
        BinOp(syntax::BinOp::Equals, lhs, rhs) => {
            if !is_negated {
                vec![BinOp(syntax::BinOp::Equals, lhs.clone(), rhs.clone())]
            } else {
                vec![BinOp(syntax::BinOp::NotEquals, lhs.clone(), rhs.clone())]
            }
        }
        BinOp(syntax::BinOp::Implies, lhs, rhs) => body_to_clauses(
            NAryOp(NOp::Or, vec![Term::negate(*lhs.clone()), *rhs.clone()]),
            is_negated,
        ),
        BinOp(syntax::BinOp::Iff, lhs, rhs) => body_to_clauses(
            NAryOp(
                NOp::And,
                vec![
                    NAryOp(NOp::Or, vec![Term::negate(*lhs.clone()), *rhs.clone()]),
                    NAryOp(NOp::Or, vec![Term::negate(*rhs.clone()), *lhs.clone()]),
                ],
            ),
            is_negated,
        ),
        NAryOp(NOp::And, terms) => {
            if is_negated {
                body_to_clauses(
                    NAryOp(
                        NOp::Or,
                        terms.into_iter().map(|t: Term| Term::negate(t)).collect(),
                    ),
                    !is_negated,
                )
            } else {
                let mut res: Vec<Term> = Vec::new();
                for t in terms {
                    res.extend(body_to_clauses(t, is_negated));
                }
                res
            }
        }
        NAryOp(NOp::Or, terms) => {
            if is_negated {
                body_to_clauses(
                    NAryOp(
                        NOp::And,
                        terms.into_iter().map(|t| Term::negate(t)).collect(),
                    ),
                    !is_negated,
                )
            } else {
                let sub_formulas: Vec<Vec<Term>> = terms
                    .into_iter()
                    .map(|t| body_to_clauses(t, false))
                    .collect();
                let product = cartesian_product(&sub_formulas);
                product
                    .into_iter()
                    .map(|ts: Vec<Term>| NAryOp(NOp::Or, ts))
                    .collect()
            }
        }
        Id(_) | App(_, _) => {
            if is_negated {
                vec![Term::negate(t)]
            } else {
                vec![t]
            }
        }
        _ => panic!("got illegal operator"),
    }
}

/// Convert a quantified term to separate clauses forming a cnf term
fn term_to_cnf_clauses(t: Term) -> Vec<Term> {
    return match t {
        Quantified {
            quantifier: syntax::Quantifier::Forall,
            body,
            binders,
        } => body_to_clauses(*body, false)
            .into_iter()
            .map(|b| Quantified {
                quantifier: syntax::Quantifier::Forall,
                body: Box::new(b),
                binders: binders.clone(),
            })
            .collect(),
        _ => body_to_clauses(t, false),
    };
}

impl Cnf {
    pub fn new(t: Term) -> Self {
        let t = if let Some(body) = get_always(&t) {
            UnaryOp(Always, Box::new(body))
        } else {
            NAryOp(And, conjuncts(t))
        };
        cnf(&t);
        Self(t)
    }
}

#[cfg(test)]
mod tests {
    use crate::fly::parser::parse_term;
    use crate::fly::syntax::{BinOp, NOp, Term};
    use crate::term::cnf::{body_to_clauses, term_to_cnf_clauses};
    use std::collections::HashSet;

    use super::{cnf, Cnf};

    #[test]
    fn test_already_cnf() {
        cnf(&parse_term("p & q & r & (a | b)").unwrap());
        cnf(&parse_term("always p & q & r & (a | b)").unwrap());
    }

    #[test]
    fn test_cnf_and() {
        let t = Term::NAryOp(
            NOp::And,
            vec![parse_term("a").unwrap(), parse_term("b & c").unwrap()],
        );
        let cnf = Cnf::new(t.clone());
        // make sure this test is non-trivial
        assert_ne!(t, cnf.0);
        assert_eq!(cnf.0, parse_term("a & b & c").unwrap());
    }

    #[test]
    fn test_cnf_always() {
        let t = parse_term("always (always (always p & q))").unwrap();
        let cnf = Cnf::new(t.clone());
        assert_ne!(t, cnf.0);
        assert_eq!(cnf.0, parse_term("always p & q").unwrap());
    }

    #[test]
    fn test_cnf_single() {
        let t = parse_term("p | q").unwrap();
        let cnf = Cnf::new(t.clone());
        assert_eq!(cnf.0, Term::NAryOp(NOp::And, vec![t]));
    }

    #[test]
    fn test_cnf_eq() {
        let t = parse_term("p = q").unwrap();
        let cnf = Cnf::new(t.clone());
        assert_eq!(
            cnf.0,
            Term::NAryOp(NOp::And, vec![parse_term("p = q").unwrap()])
        );
    }

    #[test]
    fn test_body_to_clauses() {
        let t = parse_term("(a | (b & c)) | (e & (f = g))").unwrap();
        let terms: HashSet<_> = body_to_clauses(t, false).into_iter().collect();
        let expected: HashSet<_> = vec![
            parse_term("a | b | e").unwrap(),
            parse_term("a | c | e").unwrap(),
            parse_term("a | b | (f = g)").unwrap(),
            parse_term("a | c | (f = g)").unwrap(),
        ]
        .into_iter()
        .collect();
        assert_eq!(terms, expected);
    }

    #[test]
    fn test_term_to_clauses() {
        let t = parse_term("forall a:t, b:t. (a & b)").unwrap();
        let terms: HashSet<_> = term_to_cnf_clauses(t).into_iter().collect();
        let expected: HashSet<_> = vec![
            parse_term("forall a:t, b:t. a").unwrap(),
            parse_term("forall a:t, b:t. b").unwrap(),
        ]
        .into_iter()
        .collect();
        assert_eq!(terms, expected);
    }
}
