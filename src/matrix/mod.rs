use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};
use std::fmt::{Debug, Display};
use num_traits::{One, Zero};

use crate::{IndexType, Scalar, Vector};
use anyhow::Result;

mod dense_serial;

pub trait MatrixCommon: Sized + Debug + Display
{
    type V: Vector<T = Self::T>;
    type T: Scalar;
    fn diagonal(&self) -> Self::V;
    fn nrows(&self) -> IndexType;
    fn ncols(&self) -> IndexType;
}

impl <'a, M> MatrixCommon for &'a M where M: MatrixCommon {
    type T = M::T;
    type V = M::V;
    fn diagonal(&self) -> Self::V {
        M::diagonal(self)
    }
    fn ncols(&self) -> IndexType {
        M::ncols(self)
    }
    fn nrows(&self) -> IndexType {
        M::nrows(self)
    }
}

impl <'a, M> MatrixCommon for &'a mut M where M: MatrixCommon {
    type T = M::T;
    type V = M::V;
    fn diagonal(&self) -> Self::V {
        M::diagonal(self)
    }
    fn ncols(&self) -> IndexType {
        M::ncols(self)
    }
    fn nrows(&self) -> IndexType {
        M::nrows(self)
    }
}

pub trait MatrixOpsByValue<Rhs = Self, Output = Self>: MatrixCommon 
    + Add<Rhs, Output = Output>
    + Sub<Rhs, Output = Output> 
{}

impl <M, Rhs, Output> MatrixOpsByValue<Rhs, Output> for M where M: MatrixCommon 
    + Add<Rhs, Output = Output>
    + Sub<Rhs, Output = Output> 
{}

pub trait MatrixMutOpsByValue<Rhs = Self>: MatrixCommon 
    + AddAssign<Rhs>
    + SubAssign<Rhs> 
{}

impl <M, Rhs> MatrixMutOpsByValue<Rhs> for M where M: MatrixCommon 
    + AddAssign<Rhs>
    + SubAssign<Rhs> 
{}

pub trait MatrixMutOps<View>: 
    MatrixMutOpsByValue<Self> 
    + for<'a> MatrixMutOpsByValue<&'a Self>
    + MatrixMutOpsByValue<View> 
    + for<'a> MatrixMutOpsByValue<&'a View>
    + MulAssign<Self::T>
    + DivAssign<Self::T>
{}

impl <M, View> MatrixMutOps<View> for M 
where 
    M: MatrixMutOpsByValue<Self> 
    + for<'a> MatrixMutOpsByValue<&'a Self> 
    + MatrixMutOpsByValue<View> 
    + for<'a> MatrixMutOpsByValue<&'a View>
    + MulAssign<Self::T>
    + DivAssign<Self::T>
{}

pub trait MatrixOps<View>: 
    MatrixOpsByValue<Self> 
    + for<'a> MatrixOpsByValue<&'a Self>
    + MatrixOpsByValue<View> 
    + for<'a> MatrixOpsByValue<&'a View>
    + Mul<Self::T, Output = Self>
    + Div<Self::T, Output = Self>
{}

impl <M, View> MatrixOps<View> for M 
where 
    M: MatrixOpsByValue<Self> 
    + for<'a> MatrixOpsByValue<&'a Self> 
    + MatrixOpsByValue<View> 
    + for<'a> MatrixOpsByValue<&'a View>
    + Mul<Self::T, Output = Self>
    + Div<Self::T, Output = Self>
{}

/// A trait allowing for references to implement matrix operations
/// Usage:
/// ```
/// use crate::{Matrix, Vector};
/// fn mat_vec_mul<M: Matrix, V: Vector>(m: &M, v: &V) -> V 
/// where
///     for <'a> &'a M: MatrixRef<M>,
///{
///     m.mat_mul(v)
/// }
/// ```
pub trait MatrixRef<M: MatrixCommon>:
    MatrixOpsByValue<M, M>
    + for<'a> MatrixOpsByValue<M, M> 
    + Mul<M::T, Output = M>
    + Div<M::T, Output = M>
{}

impl <RefT, M: MatrixCommon> MatrixRef<M> for RefT where
    RefT: MatrixOpsByValue<M, M>
    + for<'a> MatrixOpsByValue<&'a M, M>
    + Mul<M::T, Output = M>
    + Div<M::T, Output = M>
{}


/// A mutable view of a dense matrix [Matrix]
pub trait MatrixViewMut<'a>: 
    MatrixMutOps<Self::View>
{
    type Owned: Matrix<V = Self::V>;
    type View: MatrixView<'a, V = Self::V, Owned = Self::Owned, T = Self::T>;
    fn gemm_oo(&mut self, alpha: Self::T, a: &Self::Owned, b: &Self::Owned, beta: Self::T);
    fn gemm_vo(&mut self, alpha: Self::T, a: &Self::View, b: &Self::Owned, beta: Self::T);
}

/// A view of a dense matrix [Matrix]
pub trait MatrixView<'a>: 
    MatrixRef<Self::Owned>
    + Clone 
{
    type Owned: Matrix<V = Self::V>;
}

/// A dense matrix. The assumption is that the underlying matrix is stored in column-major order, so functions for taking columns views are efficient
pub trait Matrix: 
    for <'a> MatrixOps<Self::View<'a>>
    + for <'a> MatrixMutOps<Self::View<'a>>
    + Index<(IndexType, IndexType), Output = Self::T> 
    + IndexMut<(IndexType, IndexType), Output = Self::T> 
    + Clone 
{
    /// A view of this matrix type
    type View<'a>: MatrixView<'a, Owned = Self, T = Self::T> where Self: 'a;
    
    /// A mutable view of this matrix type
    type ViewMut<'a>: MatrixViewMut<'a, Owned = Self, T = Self::T, View = Self::View<'a>> where Self: 'a;
    
    /// Create a new matrix of shape `nrows` x `ncols` filled with zeros
    fn zeros(nrows: IndexType, ncols: IndexType) -> Self;
    
    /// Create a new diagonal matrix from a [Vector] holding the diagonal elements
    fn from_diagonal(v: &Self::V) -> Self;
    
    /// Create a new matrix from a vector of triplets (i, j, value) where i and j are the row and column indices of the value
    fn try_from_triplets(nrows: IndexType, ncols: IndexType, triplets: Vec<(IndexType, IndexType, Self::T)>) -> Result<Self>;
    
    /// Get a matrix view of the columns starting at `start` and ending at `start + ncols`
    fn columns(&self, start: IndexType, ncols: IndexType) -> Self::View<'_>;
    
    /// Get a vector view of the column `i`
    fn column(&self, i: IndexType) -> <Self::V as Vector>::View<'_>;

    /// Get a mutable matrix view of the columns starting at `start` and ending at `start + ncols`
    fn columns_mut(&mut self, start: IndexType, nrows: IndexType) -> Self::ViewMut<'_>;
    
    /// Get a mutable vector view of the column `i`
    fn column_mut(&mut self, i: IndexType) -> <Self::V as Vector>::ViewMut<'_>;
    
    /// Perform a matrix-matrix multiplication `self = alpha * a * b + beta * self`, where `alpha` and `beta` are scalars, and `a` and `b` are matrices
    fn gemm(&mut self, alpha: Self::T, a: &Self, b: &Self, beta: Self::T);
    
    /// Extract the diagonal of the matrix as an owned vector
    fn diagonal(&self) -> Self::V;
    
    /// Perform a matrix-matrix multiplication `result = self * x`.
    fn mat_mul(&self, x: &Self) -> Self {
        let mut y = Self::zeros(self.nrows(), x.ncols());
        y.gemm(Self::T::one(), self, x, Self::T::zero());
        y
    }
    
    /// Perform a matrix-vector multiplication `y = self * x + beta * y`.
    fn gemv(&self, alpha: Self::T, x: &Self::V, beta: Self::T, y: &mut Self::V);
}
