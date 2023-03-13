use crate::fly::semantics::Model;
use crate::fly::syntax::Term::NAryOp;
use crate::term::{term_to_cnf_clauses, Cnf};
use crate::{
    fly::syntax::*,
    inference::basics::FOModule,
    // fly::{semantics::*, syntax::*},
    // term::subst::*,
    verify::SolverConf,
};
use std::ops::Deref;
use std::rc::Rc;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Frame {
    terms: Vec<Term>,
}

#[allow(dead_code)]
impl Frame {
    pub fn new() {
        Frame {
            terms: Vec::from([Term::Literal(true)]),
        };
    }

    pub fn strengthen(&mut self, term: Term) {
        self.terms.push(term);
    }
}

#[derive(Debug, Clone)]
pub enum TermOrModel {
    Model(Model),
    Term(Term),
}

#[allow(dead_code)]
pub struct BackwardsReachableState {
    pub id: usize,
    pub term_or_model: TermOrModel,
    pub num_steps_to_bad: usize,
    pub known_absent_until_frame: usize,
}

#[allow(dead_code)]
pub struct UPDR {
    pub solver_conf: Rc<SolverConf>,
    frames: Vec<Frame>,
    backwards_reachable_states: Vec<BackwardsReachableState>,
    currently_blocking_id: Option<usize>,
}

#[allow(dead_code)]
impl UPDR {
    pub fn new(solver_conf: Rc<SolverConf>) -> UPDR {
        UPDR {
            solver_conf,
            frames: vec![],
            backwards_reachable_states: vec![],
            currently_blocking_id: None,
        }
    }

    pub fn find_state_to_block(&mut self, module: &FOModule) -> Option<usize> {
        println!("start");
        loop {
            println!("loop");
            // Search for a known state.
            let bstate_min = self.backwards_reachable_states.iter_mut().min_by(|b1, b2| {
                (b1.known_absent_until_frame, b1.num_steps_to_bad)
                    .cmp(&(b2.known_absent_until_frame, b2.num_steps_to_bad))
            });
            if bstate_min.is_none()
                || bstate_min.as_ref().unwrap().known_absent_until_frame == self.frames.len() - 1
            {
                println!("break");
                break;
            }
            let mut found_state = bstate_min.unwrap();
            match &found_state.term_or_model {
                TermOrModel::Term(t) => {
                    println!("m: {}", t);
                    if module
                        .trans_cex(&self.solver_conf, &self.frames.last().unwrap().terms, &t)
                        .is_some()
                    {
                        return Some(found_state.id);
                    }
                }
                TermOrModel::Model(m) => {
                    println!("m: {}", m.to_term());
                    if m.eval(
                        &NAryOp(NOp::And, self.frames.last().unwrap().terms.clone()),
                        None,
                    ) != 0
                    {
                        return Some(found_state.id);
                    }
                }
            }
            // The state does not appear in this frame.
            found_state.known_absent_until_frame += 1;
        }
        println!("broke");

        // Search for a new state.
        let last_frame = self.frames.last().unwrap();
        println!("last_frame.terms {}", &last_frame.terms[0]);
        let counter_example = module.trans_safe_cex(&self.solver_conf, &last_frame.terms);
        if module.safeties.len() == 0 || counter_example.is_none() {
            println!("None");
            // Nothing to block.
            return None;
        }
        println!(
            "counter_example: {}",
            &counter_example.as_ref().unwrap().to_term()
        );
        let new_state = BackwardsReachableState {
            id: self.backwards_reachable_states.len(),
            term_or_model: TermOrModel::Model(counter_example.unwrap()),
            num_steps_to_bad: 0,
            // Was not found in the last frame, only in this one.
            known_absent_until_frame: self.frames.len() - 2,
        };
        self.backwards_reachable_states.push(new_state);
        Some(self.backwards_reachable_states.len())
    }

    fn establish_safety(&mut self, module: &FOModule) {
        while let Some(state_index) = self.find_state_to_block(module) {
            self.currently_blocking_id = Some(state_index.clone());
            let bstate = &self.backwards_reachable_states[state_index];
            let mut trace: Vec<TermOrModel> = vec![];
            self.block(
                &bstate.term_or_model.clone(),
                &bstate.known_absent_until_frame + 1,
                &mut trace,
                module,
            );
        }
    }

    fn block(
        &mut self,
        term_or_model: &TermOrModel,
        frame_index: usize,
        trace: &mut Vec<TermOrModel>,
        module: &FOModule,
    ) {
        let as_term: Term = match term_or_model {
            TermOrModel::Term(t) => t.clone(),
            TermOrModel::Model(m) => m.to_term(),
        };
        if frame_index == 0
            || (frame_index == 1
                && module
                    .trans_safe_cex(&self.solver_conf, &vec![as_term])
                    .is_some())
        {
            panic!("abstract cex");
        }
        loop {
            if let Some((predecessor, curr)) =
                self.get_predecessor(term_or_model, frame_index - 1, module)
            {
                let src = &self.backwards_reachable_states[self.currently_blocking_id.unwrap()];
                let steps_from_cex =
                    src.known_absent_until_frame + 1 - frame_index + src.num_steps_to_bad;
                let bstate = BackwardsReachableState {
                    id: self.backwards_reachable_states.len(),
                    term_or_model: TermOrModel::Model(predecessor),
                    known_absent_until_frame: steps_from_cex,
                    num_steps_to_bad: 0,
                };
                self.backwards_reachable_states.push(bstate)
            } else {
            }
        }
    }

    fn get_predecessor(
        &mut self,
        term_or_model: &TermOrModel,
        frame_index: usize,
        module: &FOModule,
    ) -> Option<(Model, Model)> {
        let as_term: Term = match term_or_model {
            TermOrModel::Term(t) => t.clone(),
            TermOrModel::Model(m) => m.to_term(),
        };
        let prev_frame = &self.frames[frame_index];
        // if let Some((prev, curr)) =
        module.trans_cex(&self.solver_conf, &prev_frame.terms, &as_term)
        //     {
        //         println!("{} {}", &prev.to_term(), &curr.to_term());
        //         panic!();
        //     } else {
        //         println!("{}",  self.solver_conf.as_ref().backend.get_unsat_core());
        //         panic!();
        //     }
        //     None
    }

    pub fn search(&mut self, m: &Module) -> Option<Frame> {
        let mut module = FOModule::new(m, false);
        self.backwards_reachable_states = Vec::new();
        for safety in &module.safeties {
            for clause in term_to_cnf_clauses(safety) {
                self.backwards_reachable_states
                    .push(BackwardsReachableState {
                        id: self.backwards_reachable_states.len(),
                        term_or_model: TermOrModel::Term(Term::negate_and_simplify(clause)),
                        num_steps_to_bad: 0,
                        known_absent_until_frame: 0,
                    })
            }
        }
        self.frames = vec![Frame {
            terms: module
                .inits
                .clone()
                .into_iter()
                .map(|t| -> Vec<Term> {
                    match t {
                        NAryOp(NOp::And, terms) => terms,
                        _ => panic!("got malformed inits"),
                    }
                })
                .flatten()
                .collect(),
        }];
        // println!("{}", &module.safeties[0]);
        // println!("{}", backwards_reachable_states[0].term);
        // println!("{}", frames[0].terms[0]);
        // Some(frames[0].clone())
        loop {
            self.establish_safety(&module);
            self.print_frames();
            self.simplify(&module);
            // let inductive_frame: Option<Frame> = self.get_inductive_frame(&module);
            // if inductive_frame.is_some() {
            //     break inductive_frame;
            // }
            self.add_frame_and_push(&module);
            self.print_frames();
        }
    }

    fn simplify(&mut self, module: &FOModule) {
        for frame in self.frames.iter_mut() {
            let mut terms: Vec<Term> = vec![];
            for term in &frame.terms {
                let f_minus_t: Vec<Term> = frame
                    .terms
                    .clone()
                    .into_iter()
                    .filter(|t| t != term)
                    .collect();
                if !module.implies(&self.solver_conf, &f_minus_t, term) {
                    terms.push(term.clone())
                }
            }
            frame.terms = terms;
        }
    }

    fn add_frame_and_push(&mut self, module: &FOModule) {
        self.frames.push(Frame { terms: vec![] });
        for i in 0..(self.frames.len() - 1) {
            let prev_terms = self.frames[i].terms.clone();
            for term in prev_terms.iter() {
                if self.frames[i + 1].terms.contains(term) {
                    continue;
                }
                if module
                    .trans_cex(&self.solver_conf, &prev_terms, term)
                    .is_none()
                {
                    self.frames[i + 1].terms.push(term.clone());
                }
            }
        }
    }

    fn print_frames(&self) {
        println!("all frames:");
        for frame in self.frames.iter() {
            print!("[");
            for term in frame.terms.iter() {
                print!("{}, ", term);
            }
            println!("]");
        }
        println!("all BRS:");
        for state in self.backwards_reachable_states.iter() {
            print!(
                "term:{} ",
                match state.term_or_model.clone() {
                    TermOrModel::Term(t) => t,
                    TermOrModel::Model(m) => m.to_term(),
                }
            );
            println!(
                "known_absent_until_frame: {}, num_steps_to_bad : {}",
                state.known_absent_until_frame, state.num_steps_to_bad
            );
        }
    }
}
