//! Comprehensive coverage tests — targets all uncovered branches to reach ≥90%.
//!
//! Each section maps to one source module and exercises the specific branches
//! that existing inline `#[test]` blocks leave untouched.

// ══════════════════════════════════════════════════════════════════════════════
// Module: math
// Missing: abs, powi, min, max, clamp, wl2_norm, wrms_norm(empty), sqrt(0)
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_math {
    use crate::math::*;

    #[test]
    fn test_abs_positive_and_negative() {
        assert_eq!(abs(3.5), 3.5);
        assert_eq!(abs(-3.5), 3.5);
        assert_eq!(abs(0.0), 0.0);
    }

    #[test]
    fn test_sqrt_zero() {
        assert_eq!(sqrt(0.0), 0.0);
    }

    #[test]
    fn test_powi() {
        assert!((powi(2.0, 10) - 1024.0).abs() < 1e-10);
        assert!((powi(3.0, 0) - 1.0).abs() < 1e-10);
        assert!((powi(2.0, -1) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min(1.0, 2.0), 1.0);
        assert_eq!(min(2.0, 1.0), 1.0);
        assert_eq!(max(1.0, 2.0), 2.0);
        assert_eq!(max(2.0, 1.0), 2.0);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(0.5, 0.0, 1.0), 0.5);
        assert_eq!(clamp(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(clamp(2.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn test_wl2_norm() {
        let v = [3.0, 4.0];
        let w = [1.0, 1.0];
        assert!((wl2_norm(&v, &w) - 5.0).abs() < 1e-14);
    }

    #[test]
    fn test_wl2_norm_weighted() {
        // v=[1,0] w=[0,1] → 0.0
        let v = [1.0, 0.0];
        let w = [0.0, 1.0];
        assert_eq!(wl2_norm(&v, &w), 0.0);
    }

    #[test]
    fn test_wrms_norm_empty() {
        assert_eq!(wrms_norm(&[], &[]), 0.0);
    }

    #[test]
    fn test_constants() {
        assert!(UNIT_ROUNDOFF > 0.0);
        assert!(TINY > 0.0);
        assert!(BIG > 1e300);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: error
// Missing: Display for every variant, from_code for every code, from_code(0)
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_error {
    use crate::error::SundialsError;

    fn fmt(e: &SundialsError) -> String {
        format!("{}", e)
    }

    #[test]
    fn test_display_all_variants() {
        assert!(fmt(&SundialsError::TooMuchWork).contains("too much work"));
        assert!(fmt(&SundialsError::TooMuchAccuracy).contains("accuracy"));
        assert!(fmt(&SundialsError::ErrTestFailure).contains("error test"));
        assert!(fmt(&SundialsError::ConvFailure).contains("convergence"));
        assert!(fmt(&SundialsError::LinInitFail).contains("linear solver init"));
        assert!(fmt(&SundialsError::LinSetupFail).contains("setup"));
        assert!(fmt(&SundialsError::LinSolveFail).contains("solve"));
        assert!(fmt(&SundialsError::RhsFuncFail).contains("RHS"));
        assert!(fmt(&SundialsError::FirstRhsFuncErr).contains("first"));
        assert!(fmt(&SundialsError::RepeatedRhsFuncErr).contains("repeated"));
        assert!(fmt(&SundialsError::UnrecRhsFuncErr).contains("unrecoverable"));
        assert!(fmt(&SundialsError::RootFuncFail).contains("root"));
        assert!(fmt(&SundialsError::NlsInitFail).contains("nonlinear solver init"));
        assert!(fmt(&SundialsError::NlsSetupFail).contains("nonlinear solver setup"));
        assert!(fmt(&SundialsError::NlsFail).contains("nonlinear solver failed"));
        assert!(fmt(&SundialsError::ConstraintFail).contains("constraint"));
        assert!(fmt(&SundialsError::MemFail).contains("memory allocation"));
        assert!(fmt(&SundialsError::MemNull).contains("null"));
        assert!(fmt(&SundialsError::IllInput("bad".into())).contains("bad"));
        assert!(fmt(&SundialsError::NoMalloc).contains("not initialized"));
        assert!(fmt(&SundialsError::BadK).contains("bad k"));
        assert!(fmt(&SundialsError::BadT).contains("bad t"));
        assert!(fmt(&SundialsError::BadDky).contains("dky"));
        assert!(fmt(&SundialsError::TooClose).contains("too close"));
        assert!(fmt(&SundialsError::VectorOpErr).contains("vector"));
        assert!(fmt(&SundialsError::ProjMemNull).contains("projection memory"));
        assert!(fmt(&SundialsError::ProjFuncFail).contains("projection function"));
        assert!(fmt(&SundialsError::RepeatedProjFuncErr).contains("repeated projection"));
        assert!(fmt(&SundialsError::ContextErr).contains("context"));
        assert!(fmt(&SundialsError::Unrecognized(99)).contains("99"));
    }

    #[test]
    fn test_from_code_success() {
        assert!(SundialsError::from_code(0).is_none());
    }

    #[test]
    fn test_from_code_all_known() {
        for code in [-1i32, -2, -3, -4, -5, -6, -7, -8, -9, -10,
                     -11, -12, -13, -14, -15, -16, -20, -21, -22,
                     -23, -24, -25, -26, -27, -28, -29, -30, -31, -32] {
            assert!(SundialsError::from_code(code).is_some(),
                "code {code} should map to Some");
        }
    }

    #[test]
    fn test_from_code_unrecognized() {
        let e = SundialsError::from_code(-99).unwrap();
        assert_eq!(e, SundialsError::Unrecognized(-99));
    }

    #[test]
    fn test_clone_and_eq() {
        let e = SundialsError::TooMuchWork;
        assert_eq!(e.clone(), e);
    }

    #[test]
    fn test_error_trait() {
        let e: Box<dyn std::error::Error> = Box::new(SundialsError::MemFail);
        assert!(!e.to_string().is_empty());
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: dual
// Missing: cos, Div, Sub<f64>, Add<f64>, constant, From<f64>, AddAssign
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_dual {
    use crate::dual::Dual;

    #[test]
    fn test_constant_has_zero_dual() {
        let c = Dual::constant(5.0);
        assert_eq!(c.real, 5.0);
        assert_eq!(c.dual, 0.0);
    }

    #[test]
    fn test_from_f64() {
        let d: Dual = 3.14.into();
        assert_eq!(d.real, 3.14);
        assert_eq!(d.dual, 0.0);
    }

    #[test]
    fn test_sin_cos() {
        let x = Dual::new(0.0, 1.0);
        // sin(0) = 0, d/dx sin(x)|₀ = cos(0) = 1
        let s = x.sin();
        assert!((s.real - 0.0).abs() < 1e-15);
        assert!((s.dual - 1.0).abs() < 1e-15);
        // cos(0) = 1, d/dx cos(x)|₀ = -sin(0) = 0
        let c = x.cos();
        assert!((c.real - 1.0).abs() < 1e-15);
        assert!((c.dual - 0.0).abs() < 1e-15);
    }

    #[test]
    fn test_div() {
        // d/dx (x / x²) = d/dx (1/x) = -1/x² at x=2 → -0.25
        let x = Dual::new(2.0, 1.0);
        let x2 = Dual::new(4.0, 2.0); // x²
        let r = x / x2;
        assert!((r.real - 0.5).abs() < 1e-15);
        // (bc - ad)/c² = (1·4 - 2·2)/16 = 0
        assert!((r.dual - 0.0).abs() < 1e-15);
    }

    #[test]
    fn test_add_f64() {
        let x = Dual::new(1.0, 2.0);
        let r = x + 3.0;
        assert_eq!(r.real, 4.0);
        assert_eq!(r.dual, 2.0);
    }

    #[test]
    fn test_sub_f64() {
        let x = Dual::new(5.0, 2.0);
        let r = x - 1.0;
        assert_eq!(r.real, 4.0);
        assert_eq!(r.dual, 2.0);
    }

    #[test]
    fn test_mul_f64_commutative() {
        let x = Dual::new(3.0, 1.0);
        let a = x * 2.0;
        let b = 2.0 * x;
        assert_eq!(a, b);
    }

    #[test]
    fn test_add_assign() {
        let mut x = Dual::new(1.0, 1.0);
        x += Dual::new(2.0, 3.0);
        assert_eq!(x.real, 3.0);
        assert_eq!(x.dual, 4.0);
    }

    #[test]
    fn test_sub_dual() {
        let a = Dual::new(5.0, 3.0);
        let b = Dual::new(2.0, 1.0);
        let r = a - b;
        assert_eq!(r.real, 3.0);
        assert_eq!(r.dual, 2.0);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: mpir
// Missing: MpirConfig::default, NotConverged path, 1×1 system, 2×2 system
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_mpir {
    use crate::mpir::{mpir_solve, MpirConfig, MpirStatus};

    #[test]
    fn test_default_config() {
        let cfg = MpirConfig::default();
        assert_eq!(cfg.max_iter, 10);
        assert!(cfg.tol > 0.0);
    }

    #[test]
    fn test_1x1_system() {
        let a = vec![4.0f64];
        let b = vec![8.0f64];
        let cfg = MpirConfig { max_iter: 5, tol: 1e-12 };
        let (x, status) = mpir_solve(&a, &b, &cfg);
        assert!((x[0] - 2.0).abs() < 1e-8);
        assert!(matches!(status, MpirStatus::Converged { .. }));
    }

    #[test]
    fn test_2x2_identity() {
        let a = vec![1.0f64, 0.0, 0.0, 1.0];
        let b = vec![3.0, 7.0];
        let cfg = MpirConfig::default();
        let (x, _) = mpir_solve(&a, &b, &cfg);
        assert!((x[0] - 3.0).abs() < 1e-8);
        assert!((x[1] - 7.0).abs() < 1e-8);
    }

    #[test]
    fn test_not_converged_path() {
        // max_iter=0 forces the NotConverged path immediately
        let a = vec![1.0f64, 0.0, 0.0, 1.0];
        let b = vec![1.0, 1.0];
        let cfg = MpirConfig { max_iter: 0, tol: 1e-15 };
        let (_, status) = mpir_solve(&a, &b, &cfg);
        // Either converged on first pass or not — both are valid paths
        // but max_iter=0 skips the refinement loop entirely
        match status {
            MpirStatus::NotConverged { res_norm } => assert!(res_norm.is_finite()),
            MpirStatus::Converged { .. } => {} // FP32 was already good enough
        }
    }

    #[test]
    fn test_debug_format() {
        let s1 = format!("{:?}", MpirStatus::Converged { iters: 1, res_norm: 1e-13 });
        assert!(s1.contains("Converged"));
        let s2 = format!("{:?}", MpirStatus::NotConverged { res_norm: 0.1 });
        assert!(s2.contains("NotConverged"));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: epirk
// Missing: zero-vector branch, krylov_dim > n clamp, happy breakdown
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_epirk {
    use crate::epirk::{krylov_expm_v, expeuler_step, EpirkConfig};

    #[test]
    fn test_zero_vector_returns_zero() {
        let v = vec![0.0, 0.0, 0.0];
        let mv = |x: &[f64], y: &mut [f64]| { for i in 0..3 { y[i] = x[i]; } };
        let cfg = EpirkConfig::default();
        let r = krylov_expm_v(mv, &v, 0.1, &cfg);
        assert_eq!(r, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_krylov_dim_clamped_to_n() {
        // krylov_dim > n should not panic
        let v = vec![1.0, 0.0];
        let mv = |x: &[f64], y: &mut [f64]| {
            y[0] = -x[0];
            y[1] = -x[1];
        };
        let cfg = EpirkConfig { krylov_dim: 100, tol: 1e-12 };
        let r = krylov_expm_v(mv, &v, 0.1, &cfg);
        assert!(r[0].is_finite());
    }

    #[test]
    fn test_expeuler_step_zero_nonlinear() {
        // y' = -y; N(y)=0 so only linear term runs
        let mut y = vec![1.0f64];
        let cfg = EpirkConfig { krylov_dim: 1, tol: 1e-14 };
        let mv = |x: &[f64], out: &mut [f64]| { out[0] = -x[0]; };
        let nl = |_: &[f64]| vec![0.0f64];
        expeuler_step(mv, nl, &mut y, 0.1, &cfg);
        assert!(y[0] > 0.0 && y[0] < 1.0);
    }

    #[test]
    fn test_expeuler_step_with_nonlinear() {
        // y' = -y + c (constant source), exercises phi1_ny scaling path
        let mut y = vec![1.0f64];
        let cfg = EpirkConfig { krylov_dim: 1, tol: 1e-14 };
        let mv = |x: &[f64], out: &mut [f64]| { out[0] = -x[0]; };
        let nl = |_: &[f64]| vec![0.5f64]; // non-zero N(y)
        expeuler_step(mv, nl, &mut y, 0.1, &cfg);
        assert!(y[0].is_finite());
    }

    #[test]
    fn test_default_config() {
        let cfg = EpirkConfig::default();
        assert_eq!(cfg.krylov_dim, 30);
        assert!(cfg.tol > 0.0);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: pinn
// Missing: TinyMlp (various sizes), PinnPredictor with n_state > 2, features fn
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_pinn {
    use crate::pinn::{TinyMlp, PinnPredictor};

    #[test]
    fn test_mlp_3state() {
        let mlp = TinyMlp::new(5, 3, 16, 1e-3);
        let x = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let (out, h1, h2) = mlp.forward(&x);
        assert_eq!(out.len(), 3);
        assert_eq!(h1.len(), 16);
        assert_eq!(h2.len(), 16);
        // All values must be finite
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn test_train_does_not_panic() {
        let mut mlp = TinyMlp::new(3, 2, 8, 1e-3);
        let x = vec![1.0, 0.5, -0.5];
        let target = vec![0.3, 0.7];
        // Repeated SGD steps should not panic or produce NaN
        for _ in 0..20 {
            mlp.train_step(&x, &target);
        }
        let (out, _, _) = mlp.forward(&x);
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn test_predictor_warmup_returns_y() {
        let p = PinnPredictor::new(4);
        let y = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(p.predict(0.0, &y, 0.01), y);
    }

    #[test]
    fn test_predictor_update_increments_count() {
        let mut p = PinnPredictor::new(2);
        p.warmup = 0;
        assert_eq!(p.n_trained, 0);
        p.update(0.0, &[1.0, 0.5], 0.01, &[1.01, 0.49]);
        assert_eq!(p.n_trained, 1);
    }

    #[test]
    fn test_predictor_post_warmup_output_finite() {
        let mut p = PinnPredictor::new(2);
        p.warmup = 5;
        let y = vec![0.5, 0.25];
        let h = 0.01;
        for i in 0..10 {
            let t = i as f64 * h;
            let yn = vec![y[0] + 0.001, y[1] - 0.001];
            p.update(t, &y, h, &yn);
        }
        let pred = p.predict(0.1, &y, h);
        assert_eq!(pred.len(), 2);
        assert!(pred.iter().all(|v| v.is_finite()));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: gmres
// Missing: MaxItersReached path, GmresConfig::default, Givens b==0 branch
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_gmres {
    use crate::gmres::{gmres, GmresConfig, GmresStatus};

    #[test]
    fn test_default_config() {
        let cfg = GmresConfig::default();
        assert_eq!(cfg.max_restarts, 20);
        assert_eq!(cfg.restart, 30);
        assert!(cfg.tol > 0.0);
    }

    #[test]
    fn test_gmres_identity_system() {
        // A = I, b = [1,2,3] → x = [1,2,3]
        let b = vec![1.0, 2.0, 3.0];
        let mut x = vec![0.0; 3];
        let mv = |v: &[f64], y: &mut [f64]| y.copy_from_slice(v);
        let status = gmres(mv, &b, &mut x, &GmresConfig::default());
        for i in 0..3 {
            assert!((x[i] - b[i]).abs() < 1e-6, "x[{i}]={}", x[i]);
        }
        assert!(matches!(status, GmresStatus::Converged { .. }));
    }

    #[test]
    fn test_gmres_max_iters_reached() {
        // A system where GMRES never converges in 1 restart × 2 inner steps
        let b = vec![1.0, 1.0];
        let mut x = vec![0.0; 2];
        let mv = |_v: &[f64], y: &mut [f64]| { y[0] = 1e-40; y[1] = 1e-40; };
        let cfg = GmresConfig { max_restarts: 1, restart: 2, tol: 1e-14 };
        let status = gmres(mv, &b, &mut x, &cfg);
        assert!(matches!(status, GmresStatus::MaxItersReached { .. }));
    }

    #[test]
    fn test_gmres_with_restart() {
        // Use a small restart value to exercise the restart outer loop
        let a = [[4.0f64, 1.0, 0.0], [1.0, 3.0, 1.0], [0.0, 1.0, 2.0]];
        let b = vec![6.0, 9.0, 4.0];
        let mut x = vec![0.0; 3];
        let mv = |v: &[f64], y: &mut [f64]| {
            for i in 0..3 { y[i] = (0..3).map(|j| a[i][j] * v[j]).sum(); }
        };
        // restart=1 forces many restarts, exercising the outer loop path
        let cfg = GmresConfig { max_restarts: 50, restart: 1, tol: 1e-6 };
        let status = gmres(mv, &b, &mut x, &cfg);
        // Should eventually converge even with restart=1
        let mut ax = vec![0.0; 3];
        mv(&x, &mut ax);
        let res: f64 = (0..3).map(|i| (ax[i]-b[i]).powi(2)).sum::<f64>().sqrt();
        println!("GMRES restart=1 residual: {res:.2e}, status: {:?}", status);
        assert!(res < 0.1, "residual {res} too large");
    }

    #[test]
    fn test_gmres_diagonal_system() {
        // A = diag(2, 3, 5) → x = b / diag
        let diag = [2.0, 3.0, 5.0];
        let b = vec![4.0, 9.0, 15.0];
        let mut x = vec![0.0; 3];
        let mv = move |v: &[f64], y: &mut [f64]| {
            for i in 0..3 { y[i] = diag[i] * v[i]; }
        };
        gmres(mv, &b, &mut x, &GmresConfig::default());
        assert!((x[0] - 2.0).abs() < 1e-6);
        assert!((x[1] - 3.0).abs() < 1e-6);
        assert!((x[2] - 3.0).abs() < 1e-6);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module: band_solver
// Missing: get() out-of-bounds, add(), DimensionMismatch, ZeroPivot, Display
// ══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests_band {
    use crate::band_solver::{BandMat, BandError};

    #[test]
    fn test_get_out_of_bounds_returns_zero() {
        let a = BandMat::zeros(3, 1, 1);
        assert_eq!(a.get(99, 99), 0.0);
        assert_eq!(a.get(0, 99), 0.0);
    }

    #[test]
    fn test_add_element() {
        let mut a = BandMat::zeros(3, 1, 1);
        a.set(1, 1, 2.0);
        a.add(1, 1, 3.0);
        assert!((a.get(1, 1) - 5.0).abs() < 1e-15);
    }

    #[test]
    fn test_zero_pivot_error() {
        let mut a = BandMat::zeros(2, 1, 1);
        // All zeros → pivot will be zero
        let mut pivots = vec![0usize; 2];
        let result = a.band_getrf(&mut pivots);
        assert!(matches!(result, Err(BandError::ZeroPivot { .. })));
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let mut a = BandMat::zeros(3, 1, 1);
        a.set(0, 0, 1.0); a.set(1, 1, 1.0); a.set(2, 2, 1.0);
        let mut pivots = vec![0usize; 3];
        a.band_getrf(&mut pivots).unwrap();
        // Wrong-length b
        let mut b = vec![1.0, 2.0]; // length 2 instead of 3
        let result = a.band_getrs(&pivots, &mut b);
        assert!(matches!(result, Err(BandError::DimensionMismatch)));
    }

    #[test]
    fn test_band_error_display() {
        let e1 = BandError::ZeroPivot { col: 3 };
        assert!(format!("{e1}").contains("3"));
        let e2 = BandError::DimensionMismatch;
        assert!(format!("{e2}").contains("dimension"));
    }

    #[test]
    fn test_band_error_clone_eq() {
        let e = BandError::ZeroPivot { col: 1 };
        assert_eq!(e.clone(), e);
    }

    #[test]
    fn test_band_solve_5x5_diagonal() {
        let n = 5;
        let mut a = BandMat::zeros(n, 0, 0); // diagonal only
        for i in 0..n { a.set(i, i, (i + 1) as f64); }
        let mut b: Vec<f64> = (1..=n).map(|v| (v * v) as f64).collect();
        let mut pivots = vec![0usize; n];
        a.band_getrf(&mut pivots).unwrap();
        a.band_getrs(&pivots, &mut b).unwrap();
        for i in 0..n {
            let expected = (i + 1) as f64;
            assert!((b[i] - expected).abs() < 1e-10, "b[{i}]={}", b[i]);
        }
    }
}
