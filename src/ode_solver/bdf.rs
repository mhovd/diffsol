use std::cmp::{max, min};

use crate::{Scalar, Vector, IndexType, Callable, Matrix, Solver, callable::ode::BdfCallable};

use super::{OdeSolverState, OdeSolverMethod};


pub struct Bdf<'a, T: Scalar, V: Vector<T>, M: Matrix<T, V>> {
    nonlinear_solver: Option<Box<dyn Solver<'a, T, V>>>,
    bdf_callable: Option<BdfCallable<'a, T, V, M>>,
    state: Option<&'a OdeSolverState<T, V>>,
    order: u32,
    n_equal_steps: u32,
    diff: M,
    u: M,
    r: M,
    ru: M,
    alpha: Vec<T>,
    gamma: Vec<T>,
    error_const: Vec<T>,
}

// implement OdeSolverMethod for Bdf

impl<'a, T: Scalar, V: Vector<T>, M: Matrix<T, V>> Bdf<'a, T, V, M> {
    const MAX_ORDER: IndexType = 5;
    const NEWTON_MAXITER: IndexType = 4;
    const MIN_FACTOR: T = T::from(0.2);
    const MAX_FACTOR: T = T::from(10.0);
    
    pub fn new(rtol: T, atol: &'a V, max_iter: IndexType, nonlinear_solver: impl Solver<'a, T, V>) -> Self {
        let n = atol.len();
        Self { 
            nonlinear_solver: None, 
            bdf_callable: None, 
            state: None, 
            order: 1, 
            n_equal_steps: 0, 
            diff: M::zeros(Self::MAX_ORDER, n), 
            gamma: vec![T::from(1.0); Self::MAX_ORDER + 1], 
            alpha: vec![T::from(1.0); Self::MAX_ORDER + 1], 
            error_const: vec![T::from(1.0); Self::MAX_ORDER + 1], 
            u: M::zeros(Self::MAX_ORDER + 1, Self::MAX_ORDER + 1),
            r: M::zeros(Self::MAX_ORDER + 1, Self::MAX_ORDER + 1),
            ru: M::zeros(Self::MAX_ORDER + 1, Self::MAX_ORDER + 1),
        }
    }
    fn _predict(&self) {
        // predict forward to new step (eq 2 in [1])
        for i in 1..=self.order {
            self.y += self.diff.row(i);
        }
        self.scale_y = self.atol + self.state.rtol * self.y.abs();
    }

    fn _compute_r(order: u32, factor: T) -> M {
        //computes the R matrix with entries
        //given by the first equation on page 8 of [1]
        //
        //This is used to update the differences matrix when step size h is varied
        //according to factor = h_{n+1} / h_n
        //
        //Note that the U matrix also defined in the same section can be also be
        //found using factor = 1, which corresponds to R with a constant step size
        let mut r = M::zeros(order + 1, order + 1);
        for i in 1..=order {
            for j in 1..=order {
                r[(i, j)] = r[(i-1, j)] * (i - 1 - factor * j) / i;
            }
        }
        r
    }

    fn _update_step_size(&self, factor: T) {
        //If step size h is changed then also need to update the terms in
        //the first equation of page 9 of [1]:
        //
        //- constant c = h / (1-kappa) gamma_k term
        //- lu factorisation of (M - c * J) used in newton iteration (same equation)
        //- psi term

        self.h *= factor;
        self.n_equal_steps = 0;

        // update D using equations in section 3.2 of [1]
        self.r = self._compute_R(self.order, factor);
        M::gemm(T::one(), &self.r, &self.u, T::zero(), &mut self.ru);
        M::gemm(T::one(), self.diff.rows(0, self.order), &self.ru, T::zero(), &mut self.diff_tmp);
        std::mem::swap(&mut self.diff, &mut self.diff_tmp);

        // update y0 (D has changed)
        self._predict();

        // update psi and c (h, D, y0 has changed)
        self.bdf_callable.set_psi_and_y0(&self.diff, &self.gamma, &self.alpha, self.order, &self.y);
        self.bdf_callable.set_c(self.h, &self.alpha, self.order);
    }

    
    fn _update_differences(&mut self, d: &V) {
        //update of difference equations can be done efficiently
        //by reusing d and D.
        //
        //From first equation on page 4 of [1]:
        //d = y_n - y^0_n = D^{k + 1} y_n
        //
        //Standard backwards difference gives
        //D^{j + 1} y_n = D^{j} y_n - D^{j} y_{n - 1}
        //
        //Combining these gives the following algorithm
        let order = self.order;
        self.diff.row_mut(order + 2) = d - self.diff.row(order + 1);
        self.diff.row_mut(order + 1) = d;
        for i in (0..=order).rev() {
            self.diff.row_mut(i) += self.diff.row(i + 1);
        }
    }
}


impl<'a, T: Scalar, V: Vector<T>, M: Matrix<T, V>> OdeSolverMethod<'a, T, V> for Bdf<'a, T, V, M> {
    fn interpolate(&self, t: T) -> V {
        //interpolate solution at time values t* where t-h < t* < t
        //
        //definition of the interpolating polynomial can be found on page 7 of [1]
        let mut time_factor = T::from(1.0);
        let order_summation = self.diff.row(0).clone();
        for i in 0..self.order {
            time_factor *= (t - (self.state.t - self.state.h * i)) / (self.state.h * (1 + i));
            order_summation += self.diff.row(i + 1) * time_factor;
        }
        order_summation
    }


    fn set_state(&mut self, state: &'a OdeSolverState<T, V>) {
        self.state = Some(state);

        let nstates = state.rhs.nstates();
        self.order = 1u32; 
        self.psi = T::from(1.0);
        self.c = T::from(1.0);
        self.n_equal_steps = 0;
        self.diff = M::zeros(Self::MAX_ORDER + 1, nstates);
        self.diff.set_col(0, state.y);
        
        // kappa values for difference orders, taken from Table 1 of [1]
        let mut kappa = vec![0, -0.1850, -1 / 9, -0.0823, -0.0415, 0];
        self.alpha = vec![0];
        self.gamma = vec![0];
        self.error_const = vec![0];
        let mut gamma = 0;
        for i in 1..=Self::MAX_ORDER {
            self.gamma.push(self.gamma[i-1] + 1 / (i + 1).into());
            self.alpha.push(1.0 / ((1 - kappa[i]) * gamma));
            self.error_const.push(kappa[i] * gamma + 1 / (i + 1).into());
        }

        // update initial step size based on function
        let mut scale = state.y.abs();
        scale *= state.rtol;
        scale += &state.atol;

        let mut f0 = V::zeros(nstates);
        state.rhs.call(&state.y, &state.p, &mut f0);

        let mut y1 = state.y.clone();
        y1.axpy(state.h, &f0);

        let mut df = V::zeros(nstates);
        state.rhs.call(&y1, &state.p, &mut df);
        
        // store f1 in diff[1] for use in step size control
        self.diff.set_col(1, df * state.h);

        df.axpy(T::from(-1.0), &f0);
        df.component_div_assign(&scale);
        let d2 = df.norm();
        let mut new_h = state.h * d2.pow(-1 / (self.order + 1).into());
        if new_h > T::from(100.0) * state.h {
            new_h = T::from(100.0) * state.h;
        }
        state.h = new_h;

        // setup linear solver for first step
        let c = state.h * self.alpha[self.order];
        self.bdf_callable = Some(BdfCallable::new(&state.rhs, &state.mass));
        let callable = self.bdf_callable.as_ref().unwrap();
        self.nonlinear_solver.set_callable(callable, &state.p);
        
        // setup U
        self.u = self._compute_R(self.order, 1.0);
    }

    fn is_state_set(&self) -> bool {
        self.state.is_some()
    }

    fn clear_state(&mut self) {
        self.state = None;
        self.callable = None;
    }

    fn step(&mut self, t: T) -> T {
        // we will try and use the old jacobian unless convergence of newton iteration
        // fails
        // tells callable to update rhs jacobian if the jacobian is requested (by nonlinear solver)
        self.bdf_callable.set_rhs_jacobian_is_stale();
        // initialise step size and try to make the step,
        // iterate, reducing step size until error is in bounds
        let step_accepted = false;
        let n_iter = -1;
        let mut d = V::zeros(0);
        let mut safety = T::from(0.0);
        let mut error_norm = T::from(0.0);
        let mut scale_y = V::zeros(0);

        // loop until step is accepted
        while !step_accepted {
            // solve BDF equation using y0 as starting point
            match self.nonlinear_solver.solve(&self.state.y) {
                Ok(y) => {
                    // test error is within tolerance
                    scale_y = y.abs() * self.state.rtol;
                    scale_y.add_scalar_assign(&self.state.atol);

                    // combine eq 3, 4 and 6 from [1] to obtain error
                    // Note that error = C_k * h^{k+1} y^{k+1}
                    // and d = D^{k+1} y_{n+1} \approx h^{k+1} y^{k+1}
                    d = y - self.state.y;
                    let error = self.state.error_const[self.state.order] * d;
                    error_norm = (error / scale_y).norm();
                    safety = 0.9 * (2 * self.newton_stats.maxiter + 1) / (2 * self.newton_stats.maxiter + self.newton_stats.niter);
                    
                    if error_norm <= 1.0 {
                        // step is accepted
                        step_accepted = true;
                    } else {
                        // step is rejected
                        // calculate optimal step size factor as per eq 2.46 of [2]
                        // and reduce step size and try again
                        let newton_stats = self.nonlinear_solver.get_statistics();
                        let factor = max( 
                            Self::MIN_FACTOR, safety * error_norm ** (-1 / (self.order + 1))
                        );
                        self._update_step_size(factor);
                        step_accepted = false; 
                        continue
                    }
                }
                Err(e) => {
                    // newton iteration did not converge, but jacobian has already been
                    // evaluated so reduce step size by 0.3 (as per [1]) and try again
                    self._update_step_size(0.3);
                    step_accepted = false;
                    continue
                }
            };
        }

        // take the accepted step
        self.state.t += self.state.h;
        self.state.y += d;
        
        self._update_differences(&d);

        // a change in order is only done after running at order k for k + 1 steps
        // (see page 83 of [2])
        self.n_equal_steps += 1;
        
        if self.n_equal_steps < self.order + 1 {
            self._predict();
            self.bdf_callable.set_psi_and_y0(&self.diff, &self.gamma, &self.alpha, self.order, &self.y);
        } else {
            let order = self.order;
            // similar to the optimal step size factor we calculated above for the current
            // order k, we need to calculate the optimal step size factors for orders
            // k-1 and k+1. To do this, we note that the error = C_k * D^{k+1} y_n
            let error_m_norm = if order > 1 {
                let mut error_m = self.diff.row(order) * self.error_const[order];
                error_m.component_div_assign(scale_y);
                error_m.norm()
            } else {
                T::INFINITY
            };
            let error_p_norm = if order < Self::MAX_ORDER {
                let mut error_p = self.diff.row(order) * self.error_const[order + 2];
                error_p.component_div_assign(scale_y);
                error_p.norm()
            } else {
                T::INFINITY
            };

            let error_norms = vec!([error_m_norm, error_norm, error_p_norm]);
            let factors = error_norms.into_iter().enumerate().map(|(i, error_norm)| {
                error_norm ** (-1 / (i + order))
            }).collect::<Vec<_>>();

            // now we have the three factors for orders k-1, k and k+1, pick the maximum in
            // order to maximise the resultant step size
            let max_index = factors.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap().0;
            order += max_index - 1;

            let factor = min(Self::MAX_FACTOR, safety * factors[max_index]);
            self._update_step_size(factor);
        }
        self.state.t
    }

    
}