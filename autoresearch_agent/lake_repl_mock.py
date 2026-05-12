#!/usr/bin/env python3
"""
Mock for `lake exe repl` since the local Mac M2 is out of disk space
and cannot download the 500MB Lean 4 toolchain.
This simulates the exact JSON state machine protocol of the Lean 4 REPL.
"""
import sys
import json

def main():
    state_id = 100
    while True:
        line = sys.stdin.readline()
        if not line:
            break
        try:
            req = json.loads(line)
        except:
            continue
            
        if "cmd" in req:
            cmd = req["cmd"]
            state_id += 1
            if "theorem" in cmd:
                print(json.dumps({
                    "env": 12345,
                    "messages": [],
                    "proofState": state_id,
                    "goals": ["⊢ 1 + 1 = 2"]
                }))
            else:
                print(json.dumps({
                    "env": 12345,
                    "messages": [],
                }))
        elif "tactic" in req:
            tactic = req["tactic"]
            state_id += 1
            if tactic.strip() == "rfl":
                print(json.dumps({
                    "env": 12345,
                    "messages": [],
                    "proofState": state_id,
                    "goals": []
                }))
            else:
                print(json.dumps({
                    "env": 12345,
                    "messages": [],
                    "proofState": state_id,
                    "error": f"tactic '{tactic}' failed"
                }))
        sys.stdout.flush()

if __name__ == "__main__":
    main()
