use crate::{callable::{ConstantOp, LinearOp, NonLinearOp}, ode_solver::{OdeSolverProblem, OdeSolverSolution}, Matrix, Vector};
use std::ops::MulAssign;
use num_traits::Zero;
use nalgebra::ComplexField;

// exponential decay problem with algebraic constraint
// dy/dt = -ay
// 0 = z - y
// remove warning about unused mut
#[allow(unused_mut)]
fn exponential_decay_with_algebraic<M: Matrix>(x: &M::V, p: &M::V, _t: M::T, mut y: &mut M::V) 
{
    y.copy_from(x);
    y.mul_assign(-p[0]);
    let nstates = y.len();
    y[nstates - 1] = x[nstates - 1] - x[nstates - 2];
}

// Jv = [[-av, 0], [-1, 1]]v = [-av, -v[0] + v[1]]
#[allow(unused_mut)]
fn exponential_decay_with_algebraic_jacobian<M: Matrix>(_x: &M::V, p: &M::V, _t: M::T, v: &M::V, mut y: &mut M::V) {
    y.copy_from(v);
    y.mul_assign(-p[0]);
    let nstates = y.len();
    y[nstates - 1] = v[nstates - 1] - v[nstates - 2];
}

fn exponential_decay_with_algebraic_mass<M: Matrix>(x: &M::V, _p: &M::V, _t: M::T, y: &mut M::V) {
    y.copy_from(x);
    let nstates = y.len();
    y[nstates - 1] = M::T::zero();
}

fn exponential_decay_with_algebraic_init<M: Matrix>(_p: &M::V, _t: M::T) -> M::V {
    M::V::from_vec(vec![1.0.into(), 1.0.into(), 0.0.into()])
}

pub fn exponential_decay_with_algebraic_problem<M: Matrix + 'static>() -> (OdeSolverProblem<impl NonLinearOp<M = M, V = M::V, T = M::T>, impl LinearOp<M = M, V = M::V, T = M::T> , impl ConstantOp<M = M, V = M::V, T = M::T>>, OdeSolverSolution<M::V>) {
    let p = M::V::from_vec(vec![0.1.into()]);
    let problem = OdeSolverProblem::new_ode_with_mass(
        exponential_decay_with_algebraic::<M>,
        exponential_decay_with_algebraic_jacobian::<M>,
        exponential_decay_with_algebraic_mass::<M>,
        exponential_decay_with_algebraic_init::<M>,
        p.clone(),
    );
    let mut soln = OdeSolverSolution::default();
    for i in 0..10 {
        let t = M::T::from(i as f64 / 10.0);
        let y0 = M::V::from_vec(vec![1.0.into(), 1.0.into(), 1.0.into()]);
        let y: M::V = y0 * M::T::exp(-p[0] * t);
        soln.push(y, t);
    }
    (problem, soln)
}