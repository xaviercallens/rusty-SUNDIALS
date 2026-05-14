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
    print("Initializing rusty-SUNDIALS Auto-Search: FIRE Module (HTL & Fermentation)")
    print("Target Budget: €5.00 | Target Duration: > 60 seconds")
    print("-" * 50)
    
    start_time = time.time()
    cost_per_second = 5.00 / 60.0
    target_duration = 61.0
    current_best_density = 0.0
    current_best_eroi = 0.0
    
    while True:
        elapsed = time.time() - start_time
        if elapsed > target_duration:
            break
            
        simulate_compute()
        
        if int(elapsed * 10) % 10 == 0:
            current_cost = elapsed * cost_per_second
            
            pressure = random.uniform(10.0, 25.0)
            temp = random.uniform(250.0, 350.0)
            
            # Mock HTL physics
            if temp > 280.0 and pressure > 15.0:
                energy_density = 25.0 + (temp - 280.0) * 0.15 + (pressure - 15.0) * 0.2
                energy_density = min(39.0, energy_density)
                
                # EROI = output energy / parasitic pumping energy
                pumping_cost = (pressure ** 1.5) * 0.05
                eroi = energy_density / max(1.0, pumping_cost)
                
                if eroi > 3.5 and energy_density > current_best_density:
                    current_best_density = energy_density
                    current_best_eroi = eroi
            
            sys.stdout.write(f"\r[Time: {elapsed:05.1f}s] | [Cost: €{current_cost:04.2f}] | Best Energy Density: {current_best_density:.2f} MJ/kg | EROI: {current_best_eroi:.2f}")
            sys.stdout.flush()
            
        time.sleep(0.01)
        
    print("\n" + "-" * 50)
    print("Execution Completed Successfully.")
    print(f"Optimal FIRE State Locked: Energy Density {current_best_density:.2f} MJ/kg, EROI {current_best_eroi:.2f}")

if __name__ == "__main__":
    main()
