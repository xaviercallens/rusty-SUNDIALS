import numpy as np
from scipy.integrate import odeint

def vdp(y, t, mu):
    y0, y1 = y
    return [y1, mu * (1.0 - y0**2) * y1 - y0]

mu = 10.0
y0 = [2.0, 0.0]
t = [5.0, 10.0, 15.0, 20.0, 25.0]

# Add 0.0 to the beginning because odeint needs the initial time
t_eval = [0.0] + t
sol = odeint(vdp, y0, t_eval, args=(mu,))

print(f"{'t':>12} {'y0 (Scipy)':>14} {'y1 (Scipy)':>14}")
print("-" * 42)
for i, time in enumerate(t):
    print(f"{time:12.4e} {sol[i+1][0]:14.6e} {sol[i+1][1]:14.6e}")
