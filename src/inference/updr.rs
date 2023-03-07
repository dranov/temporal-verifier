use std::rc::Rc;
use crate::{
    fly::syntax::*,
    // fly::{semantics::*, syntax::*},
    // term::subst::*,
    verify::SolverConf,
    inference::basics::FOModule,
};

#[allow(dead_code)]
struct Frame {
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
struct BackwardsReachableState {
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
struct UPDR {
    solver_conf: Rc<SolverConf>,
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

    pub fn search(&self, module: FOModule) {
        let mut frames: Vec<Frame> = Vec::new();
        let mut backwards_reachable_states: Vec<BackwardsReachableState> = Vec::new();
        loop {
            self.establish_safety(&module, &mut backwards_reachable_states);
            self.simplify(&module, &mut frames);
            inductive_frame: Option<Frame> = self.get_inductive_frame(&module, &frames);
            if inductive_frame.is_some() {
                break inductive_frame
            }
            frames.push(self.new_frame());
        }
    }
}