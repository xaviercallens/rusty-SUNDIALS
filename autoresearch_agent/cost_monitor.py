import time
import os
import json
from datetime import datetime

class CostMonitor:
    """
    Simulates Google Cloud Billing API monitoring.
    Calculates cost based on active scale-to-zero periods.
    """
    PRICES_PER_SECOND = {
        "A100_VERTEX": 3.93 / 3600,   # $3.93 per hour
        "T4_VERTEX": 0.35 / 3600,     # $0.35 per hour
        "CLOUD_RUN": 0.000024         # $0.000024 per second
    }

    def __init__(self, log_dir="discoveries"):
        self.log_dir = log_dir
        self.session_start = time.time()
        self.gpu_wake_time = None
        self.total_cost = 0.0
        self.log_file = os.path.join(self.log_dir, "gcp_billing_metrics.json")
        os.makedirs(self.log_dir, exist_ok=True)
        self.metrics = []

    def log_event(self, event_type, msg, cost_increment=0.0):
        entry = {
            "timestamp": datetime.now().isoformat(),
            "event": event_type,
            "message": msg,
            "cost_added": cost_increment,
            "total_cost_so_far": self.total_cost
        }
        self.metrics.append(entry)
        
        with open(self.log_file, "w") as f:
            json.dump(self.metrics, f, indent=4)
            
        print(f"💰 [Cost Monitor] {msg} (Cost so far: ${self.total_cost:.4f})")

    def wake_a100(self):
        self.gpu_wake_time = time.time()
        self.log_event("GPU_WAKE", "Serverless A100 Scaling up from 0 to 1 replica.")

    def sleep_a100(self):
        if self.gpu_wake_time:
            active_seconds = time.time() - self.gpu_wake_time
            # Simulate cold start minimums + active time
            billed_seconds = max(120, active_seconds) # Usually a 2 minute minimum bill
            cost = billed_seconds * self.PRICES_PER_SECOND["A100_VERTEX"]
            self.total_cost += cost
            self.gpu_wake_time = None
            self.log_event("GPU_SLEEP", f"A100 scaled to 0. Billed for {billed_seconds:.1f}s.", cost)

    def finalize_session(self):
        if self.gpu_wake_time:
            self.sleep_a100()
            
        # Add Cloud Run base time
        session_time = time.time() - self.session_start
        cr_cost = session_time * self.PRICES_PER_SECOND["CLOUD_RUN"]
        self.total_cost += cr_cost
        
        # Add T4 minimum usage for CodeBERT
        t4_cost = 60 * self.PRICES_PER_SECOND["T4_VERTEX"] # Assume 60s
        self.total_cost += t4_cost
        
        self.log_event("SESSION_END", f"Session terminated. Total Cloud Run + CodeBERT + A100 costs finalized.", cr_cost + t4_cost)
        return self.total_cost

