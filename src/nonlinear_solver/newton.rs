use std::rc::Rc;

use crate::{callable::{linearise::LinearisedOp, NonLinearOp}, solver::NonLinearSolver, vector::Vector, IterativeSolver, Scalar, Solver, SolverProblem, LU};
use anyhow::{anyhow, Result};
use nalgebra::{DMatrix, DVector};
use std::ops::SubAssign;

use super::{Convergence, ConvergenceStatus};

pub struct NewtonNonlinearSolver<C: NonLinearOp> 
{
    convergence: Option<Convergence<C>>,
    linear_solver: Box<dyn Solver<LinearisedOp<C>>>,
    problem: Option<SolverProblem<C>>,
    max_iter: usize,
    niter: usize,
}

impl <T: Scalar, C: NonLinearOp<M = DMatrix<T>, V = DVector<T>, T = T>> Default for NewtonNonlinearSolver<C> 
{
    fn default() -> Self {
        let linear_solver = Box::<LU<T>>::default();
        Self {
            problem: None,
            convergence: None,
            linear_solver,
            max_iter: 100,
            niter: 0,
        }
    }
}


impl <C: NonLinearOp> NewtonNonlinearSolver<C> 
{
    pub fn new<S: Solver<LinearisedOp<C>> + 'static>(linear_solver: S) -> Self {
        let linear_solver = Box::new(linear_solver);
        Self {
            problem: None,
            convergence: None,
            linear_solver,
            max_iter: 100,
            niter: 0,
        }
    }
}

impl<C: NonLinearOp> IterativeSolver<C> for NewtonNonlinearSolver<C> 
{
    fn set_max_iter(&mut self, max_iter: usize) {
        self.max_iter = max_iter;
    }
    fn max_iter(&self) -> usize {
        self.max_iter
    }
    fn niter(&self) -> usize {
        self.niter 
    }
}

impl<C: NonLinearOp> NonLinearSolver<C> for NewtonNonlinearSolver<C> 
{}

impl<C: NonLinearOp> Solver<C> for NewtonNonlinearSolver<C> {
    fn set_problem(&mut self, problem: SolverProblem<C>) {
        self.clear_problem();
        self.problem = Some(problem);
        let problem = self.problem.as_ref().unwrap();
        self.convergence = Some(Convergence::new(
            &problem, self.max_iter
        ));
    }

    fn problem(&self) -> Option<&SolverProblem<C>> {
        self.problem.as_ref()
    }
    fn problem_mut(&mut self) -> Option<&mut SolverProblem<C>> {
        self.problem.as_mut()
    }

    fn clear_problem(&mut self) {
        self.problem = None;
        self.linear_solver.clear_problem();
    }
    fn solve_in_place(&mut self, xn: & mut C::V) -> Result<()> {
        if self.convergence.is_none() || self.problem.is_none() {
            return Err(anyhow!("NewtonNonlinearSolver::solve() called before set_problem"));
        }
        if xn.len() != self.problem.as_ref().unwrap().f.nstates() {
            return Err(anyhow!("NewtonNonlinearSolver::solve() called with state of wrong size, expected {}, got {}", self.problem.as_ref().unwrap().f.nstates(), xn.len()));
        }
        let convergence = self.convergence.as_mut().unwrap();
        let problem = self.problem.as_ref().unwrap();
        let x0 = xn.clone();
        convergence.reset(&x0);
        let mut tmp = x0.clone();
        if self.linear_solver.problem().is_none() {
            self.linear_solver.set_problem(problem.linearise(&x0));
        };
        self.niter = 0;
        loop {
            self.niter += 1;
            problem.f.call_inplace(xn, &problem.p, problem.t, &mut tmp);
            //tmp = f_at_n

            self.linear_solver.solve_in_place(&mut tmp)?;
            //tmp = -delta_n

            xn.sub_assign(&tmp);
            // xn = xn + delta_n

            let res = convergence.check_new_iteration(&mut tmp);
            match res  {
                ConvergenceStatus::Continue => continue,
                ConvergenceStatus::Converged => return Ok(()),
                ConvergenceStatus::Diverged => break,
                ConvergenceStatus::MaximumIterations => break,
            }
        }
        Err(anyhow!("Newton iteration did not converge"))
    }

    
}