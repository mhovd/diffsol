use crate::{callable::{ConstantOp, LinearOp, NonLinearOp}, matrix::Matrix, ode_solver::{OdeSolverProblem, OdeSolverSolution, Vector}};

pub fn robertson<M: Matrix + 'static>() -> (OdeSolverProblem<impl NonLinearOp<M = M, V = M::V, T = M::T>, impl LinearOp<M = M, V = M::V, T = M::T> , impl ConstantOp<M = M, V = M::V, T = M::T>>, OdeSolverSolution<M::V>) {
    let p = M::V::from_vec(vec![0.04.into(), 1.0e4.into(), 3.0e7.into()]);
    let mut problem = OdeSolverProblem::new_ode_with_mass(
        | x: &M::V, p: &M::V, _t: M::T, y: &mut M::V | {
            y[0] = -p[0] * x[0] + p[1] * x[1] * x[2];
            y[1] = p[0] * x[0] - p[1] * x[1] * x[2] - p[2] * x[1] * x[1];
            y[2] = M::T::from(1.0) - x[0] - x[1] - x[2];
        },
        | x: &M::V, p: &M::V, _t: M::T, v: &M::V, y: &mut M::V | {
            y[0] = -p[0] * v[0] + p[1] * v[1] * x[2] + p[1] * x[1] * v[2];
            y[1] = p[0] * v[0] - p[1] * v[1] * x[2] - p[1] * x[1] * v[2]  - M::T::from(2.0) * p[2] * v[1];
            y[2] = M::T::from(1.0) - v[0] - v[1] - v[2];
        },
        | x: &M::V, _p: &M::V, _t: M::T, y: &mut M::V | {
            y[0] = x[0];
            y[1] = x[1];
            y[2] = 0.0.into();
        },
        | _p: &M::V, _t: M::T | {
            M::V::from_vec(vec![1.0.into(), 0.0.into(), 0.0.into()])
        },
        p.clone(),
    );
    problem.rtol = M::T::from(1.0e-4);
    problem.atol = M::V::from_vec(vec![1.0e-8.into(), 1.0e-6.into(), 1.0e-6.into()]);
    let mut soln = OdeSolverSolution::default();
    soln.push(M::V::from_vec(vec![1.0.into(), 0.0.into(), 0.0.into()]), 0.0.into());
    soln.push(M::V::from_vec(vec![9.8517e-01.into(), 3.3864e-05.into(), 1.4794e-02.into()]), 0.4.into());
    soln.push(M::V::from_vec(vec![9.0553e-01.into(), 2.2406e-05.into(), 9.4452e-02.into()]), 4.0.into());
    soln.push(M::V::from_vec(vec![7.1579e-01.into(), 9.1838e-06.into(), 2.8420e-01.into()]), 40.0.into());
    soln.push(M::V::from_vec(vec![4.5044e-01.into(), 3.2218e-06.into(), 5.4956e-01.into()]), 400.0.into());
    soln.push(M::V::from_vec(vec![1.8320e-01.into(), 8.9444e-07.into(), 8.1680e-01.into()]), 4000.0.into());
    soln.push(M::V::from_vec(vec![3.8992e-02.into(), 1.6221e-07.into(), 9.6101e-01.into()]), 40000.0.into());
    soln.push(M::V::from_vec(vec![4.9369e-03.into(), 1.9842e-08.into(), 9.9506e-01.into()]), 400000.0.into());
    soln.push(M::V::from_vec(vec![5.1674e-04.into(), 2.0684e-09.into(), 9.9948e-01.into()]), 4000000.0.into());
    soln.push(M::V::from_vec(vec![5.2009e-05.into(), 2.0805e-10.into(), 9.9995e-01.into()]), 4.0000e+07.into());
    soln.push(M::V::from_vec(vec![5.2012e-06.into(), 2.0805e-11.into(), 9.9999e-01.into()]), 4.0000e+08.into());
    soln.push(M::V::from_vec(vec![5.1850e-07.into(), 2.0740e-12.into(), 1.0000e+00.into()]), 4.0000e+09.into());
    soln.push(M::V::from_vec(vec![4.8641e-08.into(), 1.9456e-13.into(), 1.0000e+00.into()]), 4.0000e+10.into());
    (problem, soln)
}

// Output from Sundials IDA serial example problem for Robertson kinetics:
//
//idaRoberts_dns: Robertson kinetics DAE serial example problem for IDA
//         Three equation chemical kinetics problem.
//
//Linear solver: DENSE, with user-supplied Jacobian.
//Tolerance parameters:  rtol = 0.0001   atol = 1e-08 1e-06 1e-06 
//Initial conditions y0 = (1 0 0)
//Constraints and id not used.
//
//-----------------------------------------------------------------------
//  t             y1           y2           y3      | nst  k      h
//-----------------------------------------------------------------------
//2.6402e-01   9.8997e-01   3.4706e-05   1.0000e-02 |  27  2   4.4012e-02
//    rootsfound[] =   0   1
//4.0000e-01   9.8517e-01   3.3864e-05   1.4794e-02 |  29  3   8.8024e-02
//4.0000e+00   9.0553e-01   2.2406e-05   9.4452e-02 |  43  4   6.3377e-01
//4.0000e+01   7.1579e-01   9.1838e-06   2.8420e-01 |  68  4   3.1932e+00
//4.0000e+02   4.5044e-01   3.2218e-06   5.4956e-01 |  95  4   3.3201e+01
//4.0000e+03   1.8320e-01   8.9444e-07   8.1680e-01 | 126  3   3.1458e+02
//4.0000e+04   3.8992e-02   1.6221e-07   9.6101e-01 | 161  5   2.5058e+03
//4.0000e+05   4.9369e-03   1.9842e-08   9.9506e-01 | 202  3   2.6371e+04
//4.0000e+06   5.1674e-04   2.0684e-09   9.9948e-01 | 250  3   1.7187e+05
//2.0788e+07   1.0000e-04   4.0004e-10   9.9990e-01 | 280  5   1.0513e+06
//    rootsfound[] =  -1   0
//4.0000e+07   5.2009e-05   2.0805e-10   9.9995e-01 | 293  4   2.3655e+06
//4.0000e+08   5.2012e-06   2.0805e-11   9.9999e-01 | 325  4   2.6808e+07
//4.0000e+09   5.1850e-07   2.0740e-12   1.0000e+00 | 348  3   7.4305e+08
//4.0000e+10   4.8641e-08   1.9456e-13   1.0000e+00 | 362  2   7.5480e+09
//
//Final Statistics:
//Current time                 = 41226212070.53522
//Steps                        = 362
//Error test fails             = 15
//NLS step fails               = 0
//Initial step size            = 2.164955286048077e-05
//Last step size               = 7548045540.281308
//Current step size            = 7548045540.281308
//Last method order            = 2
//Current method order         = 2
//Residual fn evals            = 537
//IC linesearch backtrack ops  = 0
//NLS iters                    = 537
//NLS fails                    = 5
//NLS iters per step           = 1.483425414364641
//LS setups                    = 60
//Jac fn evals                 = 60
//LS residual fn evals         = 0
//Prec setup evals             = 0
//Prec solves                  = 0
//LS iters                     = 0
//LS fails                     = 0
//Jac-times setups             = 0
//Jac-times evals              = 0
//LS iters per NLS iter        = 0
//Jac evals per NLS iter       = 0.111731843575419
//Prec evals per NLS iter      = 0
//Root fn evals                = 404
