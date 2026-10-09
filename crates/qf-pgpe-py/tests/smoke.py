"""Smoke/known-answer test of the `qf_pgpe` extension (no dependency beyond numpy):
    cargo build --release -p qf-pgpe-py && cp target/release/libqf_pgpe.so /tmp/ext/qf_pgpe.so && PYTHONPATH=/tmp/ext python3 crates/qf-pgpe-py/tests/smoke.py
Checks: K4 (exact plane wave), norm/momentum conservation, imprint + detection of a neutral pair, run == repeated step, input validation."""
import numpy as np
import qf_pgpe

N, L = 64, 32.0
s = qf_pgpe.Pgpe(N, L)
assert (s.n, s.l, s.dx) == (N, L, L / N) and s.n_modes > 0
# K4: psi = exp(i k x) is an exact solution, omega = k^2/2 + g n0
k = 2 * np.pi * 3 / L
x = np.arange(N) * s.dx
psi0 = np.exp(1j * k * x)[:, None] * np.ones((1, N))
s5 = qf_pgpe.Pgpe(N, L, 1.0, 0.005)  # the dt of the K4 known answer (PGPE_BKT_PREREG.md)
c0 = s5.modes(psi0)
t = 10.0
psi = s5.psi(s5.run(c0, t))
err = np.abs(psi - psi0 * np.exp(-1j * (0.5 * k * k + 1.0) * t)).max()
assert err < 1e-9, err
# conservation on a non-trivial state
rng = np.random.default_rng(1)
c = s.modes(1.0 + 0.1 * (rng.standard_normal((N, N)) + 1j * rng.standard_normal((N, N))))
n0, p0, e0 = s.norm(c), np.array(s.momentum(c)), s.energy(c)
c1 = s.run(c, 5.0)
assert abs(s.norm(c1) - n0) / n0 < 1e-8 and abs(s.energy(c1) - e0) / abs(e0) < 1e-6 and np.abs(np.array(s.momentum(c1)) - p0).max() < 1e-5
# (momentum: 2.0e-6 here, identical in the numpy engine -- a noisy field has weight at the cutoff edge, where the cubic term
#  aliases; K3's 1e-10 holds for states with weight in the interior, see lib.rs tests)
# run == repeated step
c2 = c.copy()
for _ in range(100):
    c2 = s.step(c2)
assert np.abs(c2 - s.run(c, 100 * 0.01)).max() < 1e-10
# neutral pair: imprint, then detect two vortices of opposite charge near the imprint positions
pos = np.array([[19.3, 11.1], [11.7, 11.1]])
cp = s.imprint_v2(s.uniform(), pos, np.array([1, -1], dtype=np.int64))
dp, dq = s.detect(cp)
assert sorted(dq.tolist()) == [-1, 1] and np.abs(np.sort(dp, axis=0) - np.sort(pos, axis=0)).max() < 0.5
# validation
for bad in (lambda: s.run(np.zeros((N, N + 1), complex), 1.0), lambda: qf_pgpe.Pgpe(7, 1.0)):
    try:
        bad(); raise SystemExit("expected a ValueError")
    except ValueError:
        pass
print("qf_pgpe smoke test: OK (K4 error %.1e)" % err)
