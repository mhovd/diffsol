use crate::{
    matrix::{DenseMatrix, MatrixRef},
    ode_solver::equations::OdeEquations,
    scalar::scale,
    IndexType, Matrix, OdeSolverProblem, Vector, VectorRef,
};
use num_traits::{One, Zero};
use std::{
    cell::RefCell,
    ops::{Deref, SubAssign},
    rc::Rc,
};

use super::{NonLinearOp, Op};

// callable to solve for F(y) = M (y) - f(phi + c * y) = 0
pub struct SdirkCallable<Eqn: OdeEquations> {
    eqn: Rc<Eqn>,
    c: RefCell<Eqn::T>,
    phi: RefCell<Eqn::V>,
    jac: RefCell<Eqn::M>,
    rhs_jac: RefCell<Eqn::M>,
    mass_jac: RefCell<Eqn::M>,
    rhs_jacobian_is_stale: RefCell<bool>,
    jacobian_is_stale: RefCell<bool>,
    mass_jacobian_is_stale: RefCell<bool>,
    number_of_jac_evals: RefCell<usize>,
}

impl<Eqn: OdeEquations> BdfCallable<Eqn> {
    pub fn new(ode_problem: &OdeSolverProblem<Eqn>) -> Self {
        let eqn = ode_problem.eqn.clone();
        let n = ode_problem.eqn.nstates();
        let c = RefCell::new(Eqn::T::zero());
        let phi = RefCell::new(<Eqn::V as Vector>::zeros(n));
        let rhs_jac = RefCell::new(Eqn::M::zeros(n, n));
        let jac = RefCell::new(Eqn::M::zeros(n, n));
        let mass_jac = RefCell::new(Eqn::M::zeros(n, n));
        let rhs_jacobian_is_stale = RefCell::new(true);
        let jacobian_is_stale = RefCell::new(true);
        let mass_jacobian_is_stale = RefCell::new(true);
        let number_of_jac_evals = RefCell::new(0);

        Self {
            eqn,
            phi,
            c,
            jac,
            rhs_jac,
            mass_jac,
            rhs_jacobian_is_stale,
            jacobian_is_stale,
            mass_jacobian_is_stale,
            number_of_jac_evals,
        }
    }

    pub fn number_of_jac_evals(&self) -> usize {
        *self.number_of_jac_evals.borrow()
    }
    pub fn set_c(&self, h: Eqn::T, alpha: Eqn::T)
    where
        for<'b> &'b Eqn::M: MatrixRef<Eqn::M>,
    {
        self.c.replace(h * alpha);
        if !*self.rhs_jacobian_is_stale.borrow() && !*self.mass_jacobian_is_stale.borrow() {
            let rhs_jac_ref = self.rhs_jac.borrow();
            let rhs_jac = rhs_jac_ref.deref();
            let mass_jac_ref = self.mass_jac.borrow();
            let mass_jac = mass_jac_ref.deref();
            let c = *self.c.borrow().deref();
            self.jac.replace(mass_jac - rhs_jac * c);
        } else {
            self.jacobian_is_stale.replace(true);
        }
    }
    pub fn set_phi(
        &self,
        phi: Eqn::V,
    ) {
        self.phi.replace(phi);
    }
    pub fn set_rhs_jacobian_is_stale(&self) {
        self.rhs_jacobian_is_stale.replace(true);
        self.jacobian_is_stale.replace(true);
    }
}

impl<Eqn: OdeEquations> Op for BdfCallable<Eqn> {
    type V = Eqn::V;
    type T = Eqn::T;
    type M = Eqn::M;
    fn nstates(&self) -> usize {
        self.eqn.nstates()
    }
    fn nout(&self) -> usize {
        self.eqn.nstates()
    }
    fn nparams(&self) -> usize {
        self.eqn.nparams()
    }
}

// callable to solve for F(y) = M (y' + psi) - f(y) = 0
impl<Eqn: OdeEquations> NonLinearOp for BdfCallable<Eqn>
where
    for<'b> &'b Eqn::V: VectorRef<Eqn::V>,
    for<'b> &'b Eqn::M: MatrixRef<Eqn::M>,
{
    // F(y) = M (y) - f(phi + c * y) = 0
    fn call_inplace(&self, x: &Eqn::V, t: Eqn::T, y: &mut Eqn::V) {
        let phi_ref = self.psi_neg_y0.borrow();
        let phi = phi_ref.deref();
        let c = *self.c.borrow().deref();
        let tmp = phi + c * x;
        self.eqn.rhs_inplace(t, &tmp, y);
        self.eqn.mass_inplace(t, &x, &mut tmp);
        // y = tmp  - y
        y.axpy(Eqn::T::one(), &tmp, -Eqn::T::one());
    }
    // (M - c * f'(y)) v
    fn jac_mul_inplace(&self, x: &Eqn::V, t: Eqn::T, v: &Eqn::V, y: &mut Eqn::V) {
        self.eqn.mass_inplace(t, v, y);
        let tmp = self.eqn.jac_mul(t, x, v);
        // y = - c * tmp  + y
        let c = *self.c.borrow().deref();
        y.axpy(-c, &tmp, Eqn::T::one());
    }

    fn jacobian(&self, x: &Eqn::V, t: Eqn::T) -> Eqn::M {
        if *self.mass_jacobian_is_stale.borrow() {
            self.mass_jac.replace(self.eqn.mass_matrix(t));
            self.mass_jacobian_is_stale.replace(false);
            self.jacobian_is_stale.replace(true);
        }
        if *self.rhs_jacobian_is_stale.borrow() {
            self.rhs_jac.replace(self.eqn.jacobian_matrix(x, t));
            self.rhs_jacobian_is_stale.replace(false);
            self.jacobian_is_stale.replace(true);
        }
        if *self.jacobian_is_stale.borrow() {
            let rhs_jac_ref = self.rhs_jac.borrow();
            let rhs_jac = rhs_jac_ref.deref();
            let mass_jac_ref = self.mass_jac.borrow();
            let mass_jac = mass_jac_ref.deref();
            let c = *self.c.borrow().deref();
            self.jac.replace(mass_jac - rhs_jac * c);
            self.jacobian_is_stale.replace(false);
        }
        let number_of_jac_evals = *self.number_of_jac_evals.borrow() + 1;
        self.number_of_jac_evals.replace(number_of_jac_evals);
        self.jac.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::ode_solver::test_models::exponential_decay::exponential_decay_problem;
    use crate::op::NonLinearOp;
    use crate::vector::Vector;

    use super::BdfCallable;
    type Mcpu = nalgebra::DMatrix<f64>;
    type Vcpu = nalgebra::DVector<f64>;

    #[test]
    fn test_sdirk_callable() {
        let (problem, _soln) = exponential_decay_problem::<Mcpu>(false);
        let mut sdirk_callable = SdirkCallable::new(&problem);
        let c = 0.1;
        let phi = Vcpu::from_vec(vec![1.1, 1.2]);
        sdirk_callable.set_c(c);
        sdirk_callable.set_phi(phi);
        // check that the function is correct
        let y = Vcpu::from_vec(vec![1.0, 1.0]);
        let t = 0.0;
        let mut y_out = Vcpu::from_vec(vec![0.0, 0.0]);

        // F(y) = M y -  f(phi + c * y)
        // M = |1 0|
        //     |0 1|
        // y = |1|
        //     |1|
        // f(y) = |-0.1 * y|
        //        |-0.1 * y|
        // i.e. f(phi + c * y) = |-0.1 * (1.1 + 0.1 * 1)| = |-0.21|
        //                       |-0.1 * (1.2 + 0.1 * 1)| = |-0.22|
        //  i.e. F(y) = |1 0| |1| - |-0.21| =  |0.79|
        //              |0 1| |2|   |-0.22|    |1.78|
        sdirk_callable.call_inplace(&y, t, &mut y_out);
        let y_out_expect = Vcpu::from_vec(vec![0.79, 1.78]);
        y_out.assert_eq(&y_out_expect, 1e-10);

        let v = Vcpu::from_vec(vec![1.0, 1.0]);
        // f'(y)v = |-0.1|
        //          |-0.1|
        // Mv - c * f'(y) v = |1 0| |1| - 0.1 * |-0.1| = |1.01|
        //                    |0 1| |1|         |-0.1|   |1.01|
        bdf_callable.jac_mul_inplace(&y, t, &v, &mut y_out);
        let y_out_expect = Vcpu::from_vec(vec![1.01, 1.01]);
        y_out.assert_eq(&y_out_expect, 1e-10);

        // J = M - c * f'(y) = |1 0| - 0.1 * |-0.1 0| = |1.01 0|
        //                     |0 1|         |0 -0.1|   |0 1.01|
        let jac = bdf_callable.jacobian(&y, t);
        assert_eq!(jac[(0, 0)], 1.01);
        assert_eq!(jac[(0, 1)], 0.0);
        assert_eq!(jac[(1, 0)], 0.0);
        assert_eq!(jac[(1, 1)], 1.01);
    }
}
