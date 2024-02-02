pub mod lu;
pub mod gmres;

#[cfg(test)]
pub mod tests {
    use std::rc::Rc;

    use crate::{nonlinear_solver::tests::SquareClosure, Callable, Matrix, Solver, SolverProblem, Vector, LU};
    use num_traits::{One, Zero};

    // 0 = J * x - 8
    fn square<M: Matrix>(x: &M::V, _p: &M::V, y: &mut M::V, jac: &M) {
        jac.gemv(M::T::one(), x, M::T::zero(), y); // y = J * x
        y.add_scalar_mut(M::T::from(-8.0));
    }

    // J = J * dx
    fn square_jacobian<M: Matrix>(_x: &M::V, _p: &M::V, v: &M::V, y: &mut M::V, jac: &M) {
        jac.gemv(M::T::one(), v, M::T::zero(), y); // y = J * v
    }


    pub fn test_linear_solver<M: Matrix, S: Solver<SquareClosure<M>>>(mut solver: S) {
        let op = Rc::new(SquareClosure::<M>::new(
            square,
            square_jacobian,
            M::from_diagonal(&M::V::from_vec(vec![2.0.into(), 2.0.into()])), 
            2,
        ));
        let p = M::V::zeros(0);
        let problem = Rc::new(SolverProblem::new(op, p));
        let state = <M::V as Vector>::zeros(problem.f.nstates());
        solver.set_problem(&state, problem);
        let b = M::V::from_vec(vec![2.0.into(), 4.0.into()]);
        let x = solver.solve(&b).unwrap();
        let expect = M::V::from_vec(vec![(5.0).into(), 6.0.into()]);
        x.assert_eq(&expect, 1e-6.into());
    }
    
    #[test]
    fn test_lu() {
        type T = f64;
        type M = nalgebra::DMatrix<T>;
        type S = LU<T>;
        test_linear_solver::<M, S>(S::default());
    }
}