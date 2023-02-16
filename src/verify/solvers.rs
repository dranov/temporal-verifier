use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    fly::syntax::Signature,
    solver::{backends::GenericBackend, Solver},
};
pub struct SolverConf {
    backend: GenericBackend,
    tee: Option<PathBuf>,
}

impl SolverConf {
    pub fn new(backend: GenericBackend, tee: Option<PathBuf>) -> Self {
        Self { backend, tee }
    }

    pub fn solver(&self, sig: &Signature, n_states: usize) -> Solver<GenericBackend> {
        // TODO: failures to start the solver should be bubbled up to user nicely
        Solver::new(sig, n_states, self.backend.clone(), self.tee.as_deref())
            .expect("could not start solver")
    }
}

pub struct SolverManager {
    conf: SolverConf,
    sig: Signature,
    n_states: usize,

    /// The currently running solver (eventually, a pool of available solvers).
    solver: Solver<GenericBackend>,
    query_times: Vec<Duration>,
}

impl SolverManager {
    pub fn new(conf: SolverConf, sig: &Signature, n_states: usize) -> SolverManager {
        let solver = Solver::new(sig, n_states, conf.backend.clone(), conf.tee.as_deref())
            .expect("could not launch solver");
        SolverManager {
            conf,
            sig: sig.clone(),
            n_states,
            solver,
            query_times: vec![],
        }
    }

    fn restart_solver(&mut self) {
        let solver = Solver::new(
            &self.sig,
            self.n_states,
            self.conf.backend.clone(),
            self.conf.tee.as_deref(),
        )
        .expect("could not launch solver");
        self.solver = solver;
    }

    pub fn with_solver<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Solver<GenericBackend>) -> R,
    {
        let start = Instant::now();
        let r = f(&mut self.solver);
        let t = start.elapsed();
        self.query_times.push(t);
        self.restart_solver();
        r
    }

    pub fn print_stats(&self) {
        let total: Duration = self.query_times.iter().sum();
        println!(
            "{} queries {:.1}s SMT solver time",
            self.query_times.len(),
            total.as_secs_f64()
        );
    }
}
