# Makefile for Charon -> Aeneas LLBC Extraction Pipeline
# Used by the AutoResearch LangGraph orchestrator to bridge Rust into Lean 4

.PHONY: all clean extract prove

CRATE_NAME = sundials_core
RUST_SRC = crates/sundials-core

all: extract prove

extract:
	@echo "Running Charon to extract LLBC from $(CRATE_NAME)..."
	charon --dest proofs/llbc --crate $(RUST_SRC)

prove: extract
	@echo "Running Aeneas to translate LLBC into Lean 4 pure mathematical specifications..."
	aeneas proofs/llbc/$(CRATE_NAME).llbc -backend lean -dest proofs/lean4/generated

clean:
	rm -rf proofs/llbc
	rm -rf proofs/lean4/generated
