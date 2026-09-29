//! Direct numeric cross-check against the Python reference `quantumfluids.gpe.solver2d`, on a
//! tiny deterministic 4x4 grid with a fixed (not random-at-test-time) initial field -- generated
//! once with `numpy.random.default_rng(42)` and pasted here as data, so this test does not depend
//! on Python being installed or on reproducing numpy's RNG stream in Rust. This complements the
//! property-based tests in `src/lib.rs` (which establish energy conservation, the core-relaxation
//! behaviour, etc.) with an exact-value check that the two implementations compute the same
//! numbers on the same input, not just numbers with the same qualitative properties.

use num_complex::Complex64;
use qf_gpe2d::{Grid2D, energy, norm, step};

#[rustfmt::skip]
const PSI0: [(f64, f64); 16] = [
    (0.30471707975443135, 0.36875078408249884),
    (-1.0399841062404955, -0.9588826008289989),
    (0.7504511958064572, 0.8784503013072725),
    (0.9405647163912139, -0.049925910986252896),
    (-1.9510351886538364, -0.18486236354526056),
    (-1.302179506862318, -0.6809295444039414),
    (0.12784040316728537, 1.2225413386740303),
    (-0.3162425923435822, -0.15452948206880215),
    (-0.016801157504288795, -0.4283278221631072),
    (-0.85304392757358, -0.3521335504882296),
    (0.8793979748628286, 0.5323091855533487),
    (0.7777919354289483, 0.36544406436407834),
    (0.06603069756121605, 0.4127326115959884),
    (1.1272412069680329, 0.43082100300788273),
    (0.4675093422520456, 2.1416476008704612),
    (-0.8592924628832382, -0.4064150163846156),
];

#[rustfmt::skip]
const AFTER_ONE_STEP: [(f64, f64); 16] = [
    (-1.0269936317092774, -0.7942090131704606),
    (-0.18529352567537044, 1.206911650293803),
    (0.60182706329366, -0.14754753010245367),
    (-0.6329752700800821, 0.7122873535296345),
    (-0.8746818388760148, 0.38238826444428897),
    (-1.694680500795041, 1.4404249821954076),
    (0.09295293158486749, -0.5045138397693973),
    (1.1599117310092038, 0.046808057951273914),
    (-1.1527143789070835, -0.14422301024633477),
    (-0.3575046734684894, 0.527297280892114),
    (1.0278817588067612, 0.181986240030572),
    (-1.0047733514113557, 0.37226167707798596),
    (0.0778733880547167, -0.8187636518808361),
    (0.5894918629966118, -0.16718183313592314),
    (1.1475370893404084, -0.21034419330068863),
    (1.6072635164894948, 1.1534364524826466),
];

const ENERGY_BEFORE: f64 = 76.76511552085219;
const ENERGY_AFTER: f64 = 77.35967863837827;
const NORM_BEFORE: f64 = 2.0031429112197565;

#[test]
fn matches_python_reference_on_a_deterministic_4x4_grid() {
    let grid = Grid2D::new(4, 0.3);
    let psi: Vec<Complex64> = PSI0
        .iter()
        .map(|&(re, im)| Complex64::new(re, im))
        .collect();

    assert!((energy(&psi, &grid, 1.0) - ENERGY_BEFORE).abs() / ENERGY_BEFORE.abs() < 1e-12);
    assert!((norm(&psi, &grid) - NORM_BEFORE).abs() / NORM_BEFORE.abs() < 1e-12);

    let out = step(&psi, &grid, Complex64::new(0.05, 0.0), 1.0, 1.0);
    for (i, (&got, &(re, im))) in out.iter().zip(&AFTER_ONE_STEP).enumerate() {
        let expected = Complex64::new(re, im);
        let diff = (got - expected).norm();
        assert!(
            diff < 1e-10,
            "index {i}: got {got}, expected {expected}, diff {diff:.3e}"
        );
    }

    let e_after = energy(&out, &grid, 1.0);
    assert!(
        (e_after - ENERGY_AFTER).abs() / ENERGY_AFTER.abs() < 1e-10,
        "energy after: got {e_after}, expected {ENERGY_AFTER}"
    );
}
