use crate::{matrix::MatrixRef, ode_solver::OdeSolverProblem, IndexType, Matrix, Vector, VectorRef};
use num_traits::{One, Zero};
use std::{cell::RefCell, ops::{Deref, SubAssign}, rc::Rc};

use super::{Callable, Jacobian};

// callable to solve for F(y) = M (y' + psi) - c * f(y) = 0 
pub struct BdfCallable<M: Matrix, CRhs: Callable<V = M::V, T = M::T>, CMass: Callable<V = M::V, T = M::T>> 
{
    rhs: Rc<CRhs>,
    mass: Rc<CMass>,
    psi_neg_y0: RefCell<CRhs::V>,
    c: RefCell<CRhs::T>,
    rhs_jac: RefCell<M>,
    jac: RefCell<M>,
    mass_jac: RefCell<M>,
    rhs_jacobian_is_stale: RefCell<bool>,
    jacobian_is_stale: RefCell<bool>,
    mass_jacobian_is_stale: RefCell<bool>,
}

impl<M: Matrix, CRhs: Callable<V = M::V, T = M::T>, CMass: Callable<V = M::V, T = M::T>> BdfCallable<M, CRhs, CMass> 
{
    pub fn new<CInit: Callable<V = M::V, T = M::T>>(ode_problem: Rc<OdeSolverProblem<CRhs, CMass, CInit>>) -> Self {
        let n = ode_problem.problem.f.nstates();
        let c = RefCell::new(CRhs::T::zero());
        let psi_neg_y0 = RefCell::new(<CRhs::V as Vector>::zeros(n));
        let rhs_jac = RefCell::new(<M as Matrix>::zeros(n, n));
        let jac = RefCell::new(<M as Matrix>::zeros(n, n));
        let mass_jac = RefCell::new(<M as Matrix>::zeros(n, n));
        let rhs_jacobian_is_stale = RefCell::new(true);
        let jacobian_is_stale = RefCell::new(true);
        let mass_jacobian_is_stale = RefCell::new(true);
        let rhs = ode_problem.problem.f.clone();
        let mass = ode_problem.mass.clone();

        Self { rhs, mass, psi_neg_y0, c, rhs_jac, jac, mass_jac, rhs_jacobian_is_stale, jacobian_is_stale, mass_jacobian_is_stale }
    }
    pub fn set_c(&self, h: CRhs::T, alpha: &[CRhs::T], order: IndexType) {
        self.c.replace(h * alpha[order]);
        self.jacobian_is_stale.replace(true);
    }
    pub fn set_psi_and_y0(&self, diff: &M, gamma: &[CRhs::T], alpha: &[CRhs::T], order: usize, y0: &CRhs::V) {
        // update psi term as defined in second equation on page 9 of [1]
        let mut new_psi_neg_y0 = diff.column(0) * gamma[0];
        for (i, &gamma_i) in gamma.iter().enumerate().take(order + 1).skip(1) {
            new_psi_neg_y0 += diff.column(i) * gamma_i
        }
        new_psi_neg_y0 *= alpha[order];

        // now negate y0
        new_psi_neg_y0.sub_assign(y0);
        self.psi_neg_y0.replace(new_psi_neg_y0);
    }
    pub fn set_rhs_jacobian_is_stale(&self) {
        self.rhs_jacobian_is_stale.replace(true);
        self.jacobian_is_stale.replace(true);
    }
}


// callable to solve for F(y) = M (y' + psi) - f(y) = 0 
impl<M: Matrix, CRhs: Callable<V = M::V, T = M::T>, CMass: Callable<V = M::V, T = M::T>, CInit: Callable<V = M::V, T = M::T>> Callable for BdfCallable<M, CRhs, CMass> 
where 
    for <'b> &'b CRhs::V: VectorRef<CRhs::V>,
{
    type T = CRhs::T;
    type V = CRhs::V;

    // F(y) = M (y - y0 + psi) - c * f(y) = 0
    fn call(&self, x: &CRhs::V, p: &CRhs::V, y: &mut CRhs::V) {
        self.rhs.call(x, p, y);
        let psi_neg_y0_ref = self.psi_neg_y0.borrow();
        let psi_neg_y0 = psi_neg_y0_ref.deref();
        let c = *self.c.borrow().deref();
        let tmp = x - psi_neg_y0;
        self.mass.gemv(&tmp, p, CRhs::T::one(), -c, y);
}
    fn nstates(&self) -> usize {
        self.rhs.nstates()
    }
    fn nparams(&self) -> usize {
        self.rhs.nparams()
    }
    fn nout(&self) -> usize {
        self.rhs.nout()
    }
    fn jacobian_action(&self, x: &CRhs::V, p: &CRhs::V, v: &CRhs::V, y: &mut CRhs::V) {
        let c = *self.c.borrow().deref();
        self.rhs.jacobian_action(x, p, v, y);
        self.mass.gemv(v, p,  CRhs::T::one(), -c, y);
    }
    
}

impl<CRhs: Jacobian, CMass: Jacobian<M = CRhs::M, V = CRhs::V, T = CRhs::T> + Callable<V = CRhs::V, T = CRhs::T>, CInit: Callable<V = CRhs::V, T = CRhs::T>> Jacobian for BdfCallable<CRhs::M, CRhs, CMass> 
where 
    for <'b> &'b CRhs::V: VectorRef<CRhs::V>,
    for <'b> &'b CRhs::M: MatrixRef<CRhs::M>,
{
    type M = CRhs::M;
    fn jacobian(&self, x: &CRhs::V, p: &CRhs::V) -> CRhs::M {
        if *self.mass_jacobian_is_stale.borrow() {
            self.mass_jac.replace(self.mass.jacobian(x, p));
        }
        if *self.rhs_jacobian_is_stale.borrow() {
            self.rhs_jac.replace(self.rhs.jacobian(x, p));
            self.rhs_jacobian_is_stale.replace(false);
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
        self.jac.borrow().clone()
    }
}

