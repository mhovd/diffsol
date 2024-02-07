// unit is a callable that returns returns the input vector

use crate::{Matrix, Vector};
use num_traits::One;

use super::{Jacobian, LinearOp};


pub struct UnitCallable<M: Matrix> {
    n: usize,
    _phantom: std::marker::PhantomData<M>,
}

impl<M: Matrix> Default for UnitCallable<M> {
    fn default() -> Self {
        Self::new(1)
    }
}

impl<M: Matrix> UnitCallable<M> {
    pub fn new(n: usize) -> Self {
        Self { n, _phantom: std::marker::PhantomData}
    }
}

impl<M: Matrix> super::Op for UnitCallable<M> {
    type T = M::T;
    type V = M::V;
    fn nstates(&self) -> usize {
        self.n
    }
    fn nout(&self) -> usize {
        self.n
    }
    fn nparams(&self) -> usize {
        0
    }
}

impl<M: Matrix> LinearOp for UnitCallable<M> 
{
    fn call_inplace(&self, x: &M::V, _p: &M::V, y: &mut M::V) {
        y.copy_from(x)
    }
}

// implement Jacobian
impl<M: Matrix> Jacobian for UnitCallable<M> 
{
    type M = M;
    fn jacobian(&self, _x: &Self::V, _p: &Self::V) -> Self::M {
        let mut jac = M::V::zeros(self.n);
        jac.add_scalar_mut(M::T::one());
        M::from_diagonal(&jac)
    }
}
