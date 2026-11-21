//! Positive and negative controls of the CVODE cross-check of the certificate's table propagation.

use qf_sandier_serfaty_table::{
    Dual, NSTATE, Perturb, Tol, basis, check_global, check_local, load_default, rel_pos,
};

const TIGHT: Tol = Tol {
    rtol: 1e-13,
    atol: 1e-17,
};

#[test]
fn data_file_parses_with_expected_shape() {
    let d = load_default().unwrap();
    assert_eq!(d.coeffs.modes.len(), 38);
    assert_eq!(d.init.len(), NSTATE);
    assert!(d.nodes.len() > 400, "only {} nodes", d.nodes.len());
    // edge-value lists have k/2 + 1 = 3m + 1 entries
    for (i, m) in d.coeffs.modes.iter().enumerate() {
        assert_eq!(m.edge.len(), 3 * (i + 1) + 1);
    }
    // every box is ordered and the first node (t = small) contains ss0(small)
    for nb in &d.nodes {
        for (l, h) in nb.lo.iter().zip(&nb.hi) {
            assert!(l <= h);
        }
    }
    let first = &d.nodes[0];
    assert!((first.t - d.coeffs.small).abs() < 1e-15);
    for (c, &y) in d.init.iter().enumerate() {
        assert!(
            rel_pos(y, first.lo[c], first.hi[c]).abs() <= 1.0,
            "init component {c}"
        );
    }
}

#[test]
fn basis_derivative_matches_central_difference() {
    let d = load_default().unwrap();
    for (seg, t) in [
        (1usize, 0.50),
        (2, 0.58),
        (3, 0.6208),
        (4, 0.66),
        (5, 0.6955),
    ] {
        let b = basis(&d.coeffs, Dual::var(t), seg);
        let h = 1e-6;
        let fd = (basis(&d.coeffs, Dual::cst(t + h), seg).v
            - basis(&d.coeffs, Dual::cst(t - h), seg).v)
            / (2.0 * h);
        assert!(
            (b.d - fd).abs() < 1e-6 * (1.0 + fd.abs()),
            "seg {seg} t {t}: dual {} vs fd {fd}",
            b.d
        );
        assert!(b.v.is_finite() && b.v != 0.0);
    }
}

/// Positive control: at rtol 1e-13 every CVODE value lies inside the certificate's enclosure,
/// both globally (from ss0(small)) and node to node.
#[test]
fn positive_control_cvode_inside_enclosures() {
    let d = load_default().unwrap();
    let g = check_global(&d, Perturb::default(), TIGHT).unwrap();
    assert_eq!(g.values, g.nodes * NSTATE);
    assert!(g.nodes > 400);
    assert_eq!(g.outside, 0, "global: {g:?}");
    let l = check_local(&d, Perturb::default(), TIGHT).unwrap();
    assert!(l.nodes > 200);
    assert_eq!(l.outside, 0, "local: {l:?}");
}

/// Negative control: multiplying b and b~ by (1 + 1e-6) moves the solution out of nearly every
/// enclosure (all but the initial node's), far beyond the solver drift.
#[test]
fn negative_control_perturbed_coefficient_leaves_enclosures() {
    let d = load_default().unwrap();
    let pert = Perturb {
        b_rel: 1e-6,
        k_rel: 0.0,
    };
    let g = check_global(&d, pert, TIGHT).unwrap();
    assert!(
        g.outside as f64 >= 0.99 * g.values as f64,
        "only {} of {} outside",
        g.outside,
        g.values
    );
    assert!(g.max_excess[0] > 1e-8 && g.max_excess[1] > 1e-8 && g.max_excess[2] > 1e-8);
    // a perturbed decay coefficient k -> k (1 + 1e-6) in ps_k' = b~ - k ps_k / t leaves the ps boxes
    let pk = Perturb {
        b_rel: 0.0,
        k_rel: 1e-6,
    };
    let gk = check_global(&d, pk, TIGHT).unwrap();
    assert!(gk.outside_by_group[2] > 1000, "{gk:?}");
    assert_eq!(gk.outside_by_group[0], 0);
}
