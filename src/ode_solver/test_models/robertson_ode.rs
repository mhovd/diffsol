use std::rc::Rc;

use crate::{op::{ConstantOp, LinearOp, NonLinearOp}, matrix::Matrix, ode_solver::{OdeSolverProblem, OdeSolverSolution, Vector}};

pub fn robertson_ode<M: Matrix + 'static>() -> (OdeSolverProblem<impl NonLinearOp<M = M, V = M::V, T = M::T>, impl LinearOp<M = M, V = M::V, T = M::T> , impl ConstantOp<M = M, V = M::V, T = M::T>>, OdeSolverSolution<M::V>) {
    let p = M::V::from_vec(vec![0.04.into(), 1.0e4.into(), 3.0e7.into()]);
    let mut problem = OdeSolverProblem::new_ode(
 //     dy1/dt = -.04*y1 + 1.e4*y2*y3
 //*    dy2/dt = .04*y1 - 1.e4*y2*y3 - 3.e7*(y2)^2
 //*    dy3/dt = 3.e7*(y2)^2

        | x: &M::V, p: &M::V, _t: M::T, y: &mut M::V | {
            y[0] = -p[0] * x[0] + p[1] * x[1] * x[2];
            y[1] = p[0] * x[0] - p[1] * x[1] * x[2] - p[2] * x[1] * x[1];
            y[2] = p[2] * x[1] * x[1];
        },
        | x: &M::V, p: &M::V, _t: M::T, v: &M::V, y: &mut M::V | {
            y[0] = -p[0] * v[0] + p[1] * v[1] * x[2] + p[1] * x[1] * v[2];
            y[1] = p[0] * v[0] - p[1] * v[1] * x[2] - p[1] * x[1] * v[2]  - M::T::from(2.0) * p[2] * x[1] * v[1];
            y[2] = M::T::from(2.0) * p[2] * x[1] * v[1];
        },
        | _p: &M::V, _t: M::T | {
            M::V::from_vec(vec![1.0.into(), 0.0.into(), 0.0.into()])
        },
        p.clone(),
    );
    problem.rtol = M::T::from(1.0e-4);
    problem.atol = Rc::new(M::V::from_vec(vec![1.0e-8.into(), 1.0e-6.into(), 1.0e-6.into()]));
    let mut soln = OdeSolverSolution::default();
    soln.push(M::V::from_vec(vec![1.0.into(), 0.0.into(), 0.0.into()]), 0.0.into());
    soln.push(M::V::from_vec(vec![9.851641e-01.into(), 3.386242e-05.into(), 1.480205e-02.into()]), 0.4.into());
    soln.push(M::V::from_vec(vec![9.055097e-01.into(), 2.240338e-05.into(), 9.446793e-02.into()]), 4.0.into());
    soln.push(M::V::from_vec(vec![7.158017e-01.into(), 9.185037e-06.into(), 2.841892e-01.into()]), 40.0.into());
    soln.push(M::V::from_vec(vec![4.505360e-01.into(), 3.223271e-06.into(), 5.494608e-01.into()]), 400.0.into());
    soln.push(M::V::from_vec(vec![1.832299e-01.into(), 8.944378e-07.into(), 8.167692e-01.into()]), 4000.0.into());
    soln.push(M::V::from_vec(vec![3.898902e-02.into(), 1.622006e-07.into(), 9.610108e-01.into()]), 40000.0.into());
    soln.push(M::V::from_vec(vec![4.936383e-03.into(), 1.984224e-08.into(), 9.950636e-01.into()]), 400000.0.into());
    soln.push(M::V::from_vec(vec![5.168093e-04.into(), 2.068293e-09.into(), 9.994832e-01.into()]), 4000000.0.into());
    soln.push(M::V::from_vec(vec![5.202440e-05.into(), 2.081083e-10.into(), 9.999480e-01.into()]), 4.0000e+07.into());
    soln.push(M::V::from_vec(vec![5.201061e-06.into(), 2.080435e-11.into(), 9.999948e-01.into()]), 4.0000e+08.into());
    soln.push(M::V::from_vec(vec![5.258603e-07.into(), 2.103442e-12.into(), 9.999995e-01.into()]), 4.0000e+09.into());
    soln.push(M::V::from_vec(vec![6.934511e-08.into(), 2.773804e-13.into(), 9.999999e-01.into()]), 4.0000e+10.into());
    (problem, soln)
}

/* -----------------------------------------------------------------
 * Programmer(s): Scott D. Cohen, Alan C. Hindmarsh and
 *                Radu Serban @ LLNL
 * -----------------------------------------------------------------
 * SUNDIALS Copyright Start
 * Copyright (c) 2002-2023, Lawrence Livermore National Security
 * and Southern Methodist University.
 * All rights reserved.
 *
 * See the top-level LICENSE and NOTICE files for details.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 * SUNDIALS Copyright End
 * -----------------------------------------------------------------
 * Example problem:
 *
 * The following is a simple example problem, with the coding
 * needed for its solution by CVODE. The problem is from
 * chemical kinetics, and consists of the following three rate
 * equations:
 *    dy1/dt = -.04*y1 + 1.e4*y2*y3
 *    dy2/dt = .04*y1 - 1.e4*y2*y3 - 3.e7*(y2)^2
 *    dy3/dt = 3.e7*(y2)^2
 * on the interval from t = 0.0 to t = 4.e10, with initial
 * conditions: y1 = 1.0, y2 = y3 = 0. The problem is stiff.
 * While integrating the system, we also use the rootfinding
 * feature to find the points at which y1 = 1e-4 or at which
 * y3 = 0.01. This program solves the problem with the BDF method,
 * Newton iteration with the dense linear solver, and a
 * user-supplied Jacobian routine.
 * It uses a scalar relative tolerance and a vector absolute
 * tolerance. Output is printed in decades from t = .4 to t = 4.e10.
 * Run statistics (optional outputs) are printed at the end.
 * -----------------------------------------------------------------*/

 // 3-species kinetics problem

// At t = 2.6391e-01      y =  9.899653e-01    3.470564e-05    1.000000e-02
// rootsfound[] =   0   1
// At t = 4.0000e-01      y =  9.851641e-01    3.386242e-05    1.480205e-02
// At t = 4.0000e+00      y =  9.055097e-01    2.240338e-05    9.446793e-02
// At t = 4.0000e+01      y =  7.158017e-01    9.185037e-06    2.841892e-01
// At t = 4.0000e+02      y =  4.505360e-01    3.223271e-06    5.494608e-01
// At t = 4.0000e+03      y =  1.832299e-01    8.944378e-07    8.167692e-01
// At t = 4.0000e+04      y =  3.898902e-02    1.622006e-07    9.610108e-01
// At t = 4.0000e+05      y =  4.936383e-03    1.984224e-08    9.950636e-01
// At t = 4.0000e+06      y =  5.168093e-04    2.068293e-09    9.994832e-01
// At t = 2.0790e+07      y =  1.000000e-04    4.000397e-10    9.999000e-01
// rootsfound[] =  -1   0
// At t = 4.0000e+07      y =  5.202440e-05    2.081083e-10    9.999480e-01
// At t = 4.0000e+08      y =  5.201061e-06    2.080435e-11    9.999948e-01
// At t = 4.0000e+09      y =  5.258603e-07    2.103442e-12    9.999995e-01
// At t = 4.0000e+10      y =  6.934511e-08    2.773804e-13    9.999999e-01
// 
// Final Statistics:
// Current time                 = 41154661313.59952
// Steps                        = 542
// Error test fails             = 22
// NLS step fails               = 0
// Initial step size            = 8.236259832589498e-14
// Last step size               = 4747036977.219155
// Current step size            = 4747036977.219155
// Last method order            = 4
// Current method order         = 4
// Stab. lim. order reductions  = 0
// RHS fn evals                 = 754
// NLS iters                    = 751
// NLS fails                    = 3
// NLS iters per step           = 1.385608856088561
// LS setups                    = 107
// Jac fn evals                 = 11
// LS RHS fn evals              = 0
// Prec setup evals             = 0
// Prec solves                  = 0
// LS iters                     = 0
// LS fails                     = 0
// Jac-times setups             = 0
// Jac-times evals              = 0
// LS iters per NLS iter        = 0
// Jac evals per NLS iter       = 0.01464713715046605
// Prec evals per NLS iter      = 0
// Root fn evals                = 570