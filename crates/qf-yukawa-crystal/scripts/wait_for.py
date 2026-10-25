"""Block until a file exists or a timeout passes. Usage: python -I wait_for.py PATH SECONDS"""

import os
import sys
import time

path, limit = sys.argv[1], float(sys.argv[2])
t0 = time.time()
while not os.path.exists(path) and time.time() - t0 < limit:
    time.sleep(10)
print("exists" if os.path.exists(path) else "timeout", round(time.time() - t0))
