use crate::solver::Cvode;
use sundials_core::Real;

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

pub fn test_send_sync() {
    let f = |_t: Real, _y: &[Real], _ydot: &mut [Real]| -> Result<(), String> { Ok(()) };
    assert_send::<Cvode<fn(Real, &[Real], &mut [Real]) -> Result<(), String>>>();
}
