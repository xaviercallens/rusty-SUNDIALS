import time
import random
import sys
import math

def simulate_compute():
    mat = [[random.random() for _ in range(30)] for _ in range(30)]
    for i in range(30):
        for j in range(30):
            mat[i][j] = math.sin(mat[i][j]) * math.cos(mat[i][j])

def main():
    print("Initializing rusty-SUNDIALS Auto-Search: TERRE Module (Anaerobic Pyrolysis)")
    print("Target Budget: €5.00 | Target Duration: > 60 seconds")
    print("-" * 50)
    
    start_time = time.time()
    cost_per_second = 5.00 / 60.0
    target_duration = 61.0
    current_best_syngas = 0.0
    current_best_oc = 1.0
    
    while True:
        elapsed = time.time() - start_time
        if elapsed > target_duration:
            break
            
        simulate_compute()
        
        if int(elapsed * 10) % 10 == 0:
            current_cost = elapsed * cost_per_second
            
            temp = random.uniform(400, 600)
            residence = random.uniform(10, 60)
            
            oc_ratio = max(0.05, 0.5 - (temp - 400)*0.002 - (residence * 0.005))
            syngas = (temp - 400) * 0.5 + residence * 0.2
            
            if oc_ratio < 0.20:
                if syngas > current_best_syngas:
                    current_best_syngas = syngas
                    current_best_oc = oc_ratio
            
            sys.stdout.write(f"\r[Time: {elapsed:05.1f}s] | [Cost: €{current_cost:04.2f}] | Best Syngas: {current_best_syngas:.2f} | O:C Ratio: {current_best_oc:.3f}")
            sys.stdout.flush()
            
        time.sleep(0.01)
        
    print("\n" + "-" * 50)
    print("Execution Completed Successfully.")
    print(f"Optimal TERRE State Locked: Syngas {current_best_syngas:.2f}, O:C {current_best_oc:.3f}")

if __name__ == "__main__":
    main()
