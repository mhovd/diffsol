pub mod bdf;
pub mod builder;
pub mod equations;
pub mod method;
pub mod problem;
pub mod sdirk;
pub mod test_models;

#[cfg(feature = "diffsl")]
pub mod diffsl;

#[cfg(feature = "sundials")]
pub mod sundials;

#[cfg(test)]
mod tests {
    use self::problem::OdeSolverSolution;

    use super::test_models::{
        exponential_decay::exponential_decay_problem,
        exponential_decay_with_algebraic::exponential_decay_with_algebraic_problem,
        robertson::robertson, robertson_ode::robertson_ode,
    };
    use super::*;
    use crate::linear_solver::nalgebra::lu::LU;
    use crate::matrix::Matrix;
    use crate::nonlinear_solver::newton::NewtonNonlinearSolver;
    use crate::op::filter::FilterCallable;
    use crate::op::ode_rhs::OdeRhs;
    use crate::op::Op;
    use crate::scalar::scale;
    use crate::LU;
    use crate::{NonLinearSolver, OdeEquations, OdeSolverMethod, OdeSolverProblem, OdeSolverState};
    use crate::{Sdirk, Tableau, Vector};
    use num_traits::One;
    use num_traits::Zero;
    use tests::bdf::Bdf;
    use tests::test_models::dydt_y2::dydt_y2_problem;
    use tests::test_models::gaussian_decay::gaussian_decay_problem;

    fn test_ode_solver<M, Eqn>(
        method: &mut impl OdeSolverMethod<Eqn>,
        mut root_solver: impl NonLinearSolver<FilterCallable<OdeRhs<Eqn>>>,
        problem: OdeSolverProblem<Eqn>,
        solution: OdeSolverSolution<M::V>,
        override_tol: Option<M::T>,
    ) where
        M: Matrix + 'static,
        Eqn: OdeEquations<M = M, T = M::T, V = M::V>,
    {
        let state = OdeSolverState::new_consistent(&problem, &mut root_solver).unwrap();
        method.set_problem(state, &problem);
        for point in solution.solution_points.iter() {
            while method.state().unwrap().t < point.t {
                method.step().unwrap();
            }

            let soln = method.interpolate(point.t).unwrap();

            if let Some(override_tol) = override_tol {
                soln.assert_eq_st(&point.state, override_tol);
            } else {
                let tol = {
                    let problem = method.problem().unwrap();
                    point.state.abs() * scale(problem.rtol) + problem.atol.as_ref()
                };
                soln.assert_eq(&point.state, &(tol * scale(Eqn::T::from(10.0))));
            }
        }
    }

    type Mcpu = nalgebra::DMatrix<f64>;

    #[test]
    fn test_tr_bdf2_nalgebra_exponential_decay() {
        let tableau = Tableau::<Mcpu>::implicit_euler();
        let mut s = Sdirk::new(tableau, LU::default());
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = exponential_decay_problem::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 4
        number_of_steps: 4
        number_of_error_test_failures: 0
        number_of_nonlinear_solver_iterations: 0
        number_of_nonlinear_solver_fails: 0
        initial_step_size: 0.1919383103666485
        final_step_size: 0.2869585664044871
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 18
        number_of_jac_mul_evals: 2
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 1
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_exponential_decay() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = exponential_decay_problem::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 16
        number_of_steps: 19
        number_of_error_test_failures: 8
        number_of_nonlinear_solver_iterations: 54
        number_of_nonlinear_solver_fails: 0
        initial_step_size: 0.011892071150027213
        final_step_size: 0.23215911532645564
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 56
        number_of_jac_mul_evals: 2
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 1
        "###);
    }

    #[cfg(feature = "sundials")]
    #[test]
    fn test_sundials_exponential_decay() {
        let mut s = crate::SundialsIda::default();
        let rs = NewtonNonlinearSolver::new(crate::SundialsLinearSolver::new_dense());
        let (problem, soln) = exponential_decay_problem::<crate::SundialsMatrix>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 16
        number_of_steps: 24
        number_of_error_test_failures: 3
        number_of_nonlinear_solver_iterations: 39
        number_of_nonlinear_solver_fails: 0
        initial_step_size: 0.001
        final_step_size: 0.256
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 39
        number_of_jac_mul_evals: 32
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 16
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_exponential_decay_algebraic() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = exponential_decay_with_algebraic_problem::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 18
        number_of_steps: 21
        number_of_error_test_failures: 7
        number_of_nonlinear_solver_iterations: 58
        number_of_nonlinear_solver_fails: 2
        initial_step_size: 0.004450050658086208
        final_step_size: 0.20974041151932246
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 62
        number_of_jac_mul_evals: 7
        number_of_mass_evals: 64
        number_of_mass_matrix_evals: 2
        number_of_jacobian_matrix_evals: 2
        "###);
    }

    #[test]
    fn test_implicit_euler_nalgebra_robertson() {
        let tableau = Tableau::<Mcpu>::implicit_euler();
        let mut s = Sdirk::new(tableau, LU::default());
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 243
        number_of_steps: 182
        number_of_error_test_failures: 16
        number_of_nonlinear_solver_iterations: 0
        number_of_nonlinear_solver_fails: 45
        initial_step_size: 0.017125887701465843
        final_step_size: 54006237997.52221
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 3387
        number_of_jac_mul_evals: 130
        number_of_mass_evals: 3390
        number_of_mass_matrix_evals: 2
        number_of_jacobian_matrix_evals: 43
        "###);
    }

    #[test]
    fn test_esdirk34_nalgebra_robertson() {
        let tableau = Tableau::<Mcpu>::esdirk34();
        let mut s = Sdirk::new(tableau, LU::default());
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 243
        number_of_steps: 182
        number_of_error_test_failures: 16
        number_of_nonlinear_solver_iterations: 0
        number_of_nonlinear_solver_fails: 45
        initial_step_size: 0.017125887701465843
        final_step_size: 54006237997.52221
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 3387
        number_of_jac_mul_evals: 130
        number_of_mass_evals: 3390
        number_of_mass_matrix_evals: 2
        number_of_jacobian_matrix_evals: 43
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_robertson() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 106
        number_of_steps: 345
        number_of_error_test_failures: 5
        number_of_nonlinear_solver_iterations: 983
        number_of_nonlinear_solver_fails: 22
        initial_step_size: 0.0000045643545698038086
        final_step_size: 5435491162.573224
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 986
        number_of_jac_mul_evals: 55
        number_of_mass_evals: 989
        number_of_mass_matrix_evals: 2
        number_of_jacobian_matrix_evals: 18
        "###);
    }

    #[cfg(feature = "sundials")]
    #[test]
    fn test_sundials_robertson() {
        let mut s = crate::SundialsIda::default();
        let rs = NewtonNonlinearSolver::new(crate::SundialsLinearSolver::new_dense());
        let (problem, soln) = robertson::<crate::SundialsMatrix>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 59
        number_of_steps: 355
        number_of_error_test_failures: 15
        number_of_nonlinear_solver_iterations: 506
        number_of_nonlinear_solver_fails: 5
        initial_step_size: 0.001
        final_step_size: 11535117835.253025
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 507
        number_of_jac_mul_evals: 178
        number_of_mass_evals: 686
        number_of_mass_matrix_evals: 60
        number_of_jacobian_matrix_evals: 59
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_robertson_colored() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson::<Mcpu>(true);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 106
        number_of_steps: 345
        number_of_error_test_failures: 5
        number_of_nonlinear_solver_iterations: 983
        number_of_nonlinear_solver_fails: 22
        initial_step_size: 0.0000045643545698038086
        final_step_size: 5435491162.573224
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 986
        number_of_jac_mul_evals: 58
        number_of_mass_evals: 988
        number_of_mass_matrix_evals: 2
        number_of_jacobian_matrix_evals: 18
        "###);
    }

    #[test]
    fn test_tr_bdf2_nalgebra_robertson_ode() {
        let tableau = Tableau::<Mcpu>::tr_bdf2();
        let mut s = Sdirk::new(tableau, LU::default());
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson_ode::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 283
        number_of_steps: 270
        number_of_error_test_failures: 1
        number_of_nonlinear_solver_iterations: 0
        number_of_nonlinear_solver_fails: 12
        initial_step_size: 0.0010137172178872197
        final_step_size: 43056078583.7812
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 2703
        number_of_jac_mul_evals: 39
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 13
        "###);
    }

    #[test]
    fn test_sdirk4_nalgebra_robertson_ode() {
        let tableau = Tableau::<Mcpu>::tr_bdf2();
        let mut s = Sdirk::new(tableau, LU::default());
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson_ode::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 166
        number_of_steps: 116
        number_of_error_test_failures: 2
        number_of_nonlinear_solver_iterations: 0
        number_of_nonlinear_solver_fails: 48
        initial_step_size: 0.015979018288085487
        final_step_size: 38942585345.694695
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 2637
        number_of_jac_mul_evals: 135
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 45
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_robertson_ode() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = robertson_ode::<Mcpu>(false);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 106
        number_of_steps: 346
        number_of_error_test_failures: 7
        number_of_nonlinear_solver_iterations: 982
        number_of_nonlinear_solver_fails: 20
        initial_step_size: 0.0000038381494276795106
        final_step_size: 5227948052.846298
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 984
        number_of_jac_mul_evals: 57
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 19
        "###);
    }

    #[test]
    fn test_tr_bdf2_nalgebra_dydt_y2() {
        let tableau = Tableau::<Mcpu>::tr_bdf2();
        let mut s = Sdirk::new(tableau, LU::default());
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = dydt_y2_problem::<Mcpu>(false, 10);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 219
        number_of_steps: 217
        number_of_error_test_failures: 0
        number_of_nonlinear_solver_iterations: 0
        number_of_nonlinear_solver_fails: 2
        initial_step_size: 0.000027127822860263058
        final_step_size: 0.8542727221697889
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 2352
        number_of_jac_mul_evals: 30
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 3
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_dydt_y2() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = dydt_y2_problem::<Mcpu>(false, 10);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 65
        number_of_steps: 205
        number_of_error_test_failures: 12
        number_of_nonlinear_solver_iterations: 591
        number_of_nonlinear_solver_fails: 5
        initial_step_size: 0.0000019982428436469115
        final_step_size: 1.0775215291906979
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 593
        number_of_jac_mul_evals: 60
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 6
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_dydt_y2_colored() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = dydt_y2_problem::<Mcpu>(true, 10);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 65
        number_of_steps: 205
        number_of_error_test_failures: 12
        number_of_nonlinear_solver_iterations: 591
        number_of_nonlinear_solver_fails: 5
        initial_step_size: 0.0000019982428436469115
        final_step_size: 1.0775215291906979
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 593
        number_of_jac_mul_evals: 16
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 6
        "###);
    }

    #[test]
    fn test_bdf_nalgebra_gaussian_decay() {
        let mut s = Bdf::default();
        let rs = NewtonNonlinearSolver::new(LU::default());
        let (problem, soln) = gaussian_decay_problem::<Mcpu>(false, 10);
        test_ode_solver(&mut s, rs, problem.clone(), soln, None);
        insta::assert_yaml_snapshot!(s.get_statistics(), @r###"
        ---
        number_of_linear_solver_setups: 16
        number_of_steps: 59
        number_of_error_test_failures: 4
        number_of_nonlinear_solver_iterations: 159
        number_of_nonlinear_solver_fails: 0
        initial_step_size: 0.0025148668593658707
        final_step_size: 0.19573299396674515
        "###);
        insta::assert_yaml_snapshot!(problem.eqn.as_ref().get_statistics(), @r###"
        ---
        number_of_rhs_evals: 161
        number_of_jac_mul_evals: 10
        number_of_mass_evals: 0
        number_of_mass_matrix_evals: 0
        number_of_jacobian_matrix_evals: 1
        "###);
    }

    pub struct TestEqn<M> {
        _m: std::marker::PhantomData<M>,
    }
    impl<M: Matrix> Op for TestEqn<M> {
        type M = M;
        type T = M::T;
        type V = M::V;

        fn nout(&self) -> usize {
            1
        }

        fn nstates(&self) -> usize {
            1
        }

        fn nparams(&self) -> usize {
            1
        }
    }
    impl<M: Matrix> OdeEquations for TestEqn<M> {
        fn set_params(&mut self, _p: Self::V) {}

        fn rhs_inplace(&self, _t: Self::T, _y: &Self::V, rhs_y: &mut Self::V) {
            rhs_y[0] = M::T::zero();
        }

        fn rhs_jac_inplace(&self, _t: Self::T, _x: &Self::V, _v: &Self::V, y: &mut Self::V) {
            y[0] = M::T::zero();
        }

        fn init(&self, _t: Self::T) -> Self::V {
            M::V::from_element(1, M::T::zero())
        }
    }

    pub fn test_interpolate<M: Matrix, Method: OdeSolverMethod<TestEqn<M>>>(mut s: Method) {
        let problem = OdeSolverProblem::new(
            TestEqn {
                _m: std::marker::PhantomData,
            },
            M::T::from(1e-6),
            M::V::from_element(1, M::T::from(1e-6)),
            M::T::zero(),
            M::T::one(),
        );
        let state = OdeSolverState::new(&problem);
        s.set_problem(state.clone(), &problem);
        let t0 = M::T::zero();
        let t1 = M::T::one();
        s.interpolate(t0)
            .unwrap()
            .assert_eq_st(&state.y, M::T::from(1e-9));
        assert!(s.interpolate(t1).is_err());
        s.step().unwrap();
        assert!(s.interpolate(s.state().unwrap().t).is_ok());
        assert!(s.interpolate(s.state().unwrap().t + t1).is_err());
    }

    pub fn test_no_set_problem<M: Matrix, Method: OdeSolverMethod<TestEqn<M>>>(mut s: Method) {
        assert!(s.state().is_none());
        assert!(s.problem().is_none());
        assert!(s.take_state().is_none());
        assert!(s.step().is_err());
        assert!(s.interpolate(M::T::one()).is_err());
    }

    pub fn test_take_state<M: Matrix, Method: OdeSolverMethod<TestEqn<M>>>(mut s: Method) {
        let problem = OdeSolverProblem::new(
            TestEqn {
                _m: std::marker::PhantomData,
            },
            M::T::from(1e-6),
            M::V::from_element(1, M::T::from(1e-6)),
            M::T::zero(),
            M::T::one(),
        );
        let state = OdeSolverState::new(&problem);
        s.set_problem(state.clone(), &problem);
        let state2 = s.take_state().unwrap();
        state2.y.assert_eq_st(&state.y, M::T::from(1e-9));
        assert!(s.take_state().is_none());
        assert!(s.state().is_none());
        assert!(s.step().is_err());
        assert!(s.interpolate(M::T::one()).is_err());
    }
}
