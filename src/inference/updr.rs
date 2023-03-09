use std::rc::Rc;
use crate::{
    fly::syntax::*,
    // fly::{semantics::*, syntax::*},
    // term::subst::*,
    verify::SolverConf,
    inference::basics::FOModule,
};
use crate::term::{Cnf, term_to_cnf_clauses};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Frame {
    terms: Vec<Term>,
}

#[allow(dead_code)]
impl Frame {
    pub fn new() {
        Frame { terms: Vec::from([Term::Literal(true)]) };
    }

    pub fn strengthen(&mut self, term: Term) {
        self.terms.push(term);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BackwardsReachableState {
    pub id: usize,
    pub term: Term,
    pub num_steps_to_bad: usize,
    pub known_absent_until_frame: usize,
}

#[allow(dead_code)]
impl BackwardsReachableState {
    pub fn new(id: usize, term: Term, num_steps_to_bad: usize) {
        BackwardsReachableState {
            id,
            term,
            num_steps_to_bad,
            known_absent_until_frame: 0,
        };
    }
}

#[allow(dead_code)]
pub struct UPDR {
    pub solver_conf: Rc<SolverConf>,
    // frames: Vec<Frame>,
    // backwards_reachable_states: Vec<BackwardsReachableState>,
    // currently_blocking: BackwardsReachableState,
    // safeties: Vec<Term>,
}

#[allow(dead_code)]
impl UPDR {
    pub fn new(solver_conf: Rc<SolverConf>) {
        UPDR { solver_conf };
    }

    pub fn find_state_to_block(
        module: &FOModule,
        backwards_reachable_states: &Vec<BackwardsReachableState>,
    ) -> Option<BackwardsReachableState> {
        loop {

        }
    }

    pub fn search(&self, m: &Module) -> Option<Frame> {
        let mut module = FOModule::new(m, false);
        let mut backwards_reachable_states: Vec<BackwardsReachableState> = Vec::new();
        for safety in &module.safeties {
            for clause in term_to_cnf_clauses(safety) {
                backwards_reachable_states.push(
                    BackwardsReachableState {
                        id: backwards_reachable_states.len(),
                        term: clause,
                        num_steps_to_bad: 0,
                        known_absent_until_frame: 0,
                    }
                )
            }
        }
        let mut frames: Vec<Frame> = vec![Frame{
            terms: module.inits.clone(),
        }];
        println!("{}", &module.safeties[0]);
        println!("{}", backwards_reachable_states[0].term);
        println!("{}", frames[0].terms[0]);
        Some(frames[0].clone())
        // loop {
        //     self.establish_safety(&module, &mut backwards_reachable_states);
        //     self.simplify(&module, &mut frames);
        //     let inductive_frame: Option<Frame> = self.get_inductive_frame(&module, &frames);
        //     if inductive_frame.is_some() {
        //         break inductive_frame;
        //     }
        //     frames.push(self.new_frame());
        // }
    }
}