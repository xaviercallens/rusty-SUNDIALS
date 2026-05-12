#!/bin/bash
# ==============================================================================
# V6 Auto-Research: 1D Magnetic Tearing Mode — GCP Spot VM Bootstrap Script
# Budget: < $100 total | VM: g2-standard-8 + NVIDIA L4 Spot
# ==============================================================================
set -e

echo "============================================================"
echo "  🚀 Rusty-SUNDIALS V6: 1D Tearing Mode Agent Bootstrap"
echo "  📅 $(date)"
echo "============================================================"

# ── 1. SYSTEM PACKAGES ─────────────────────────────────────────
echo "📦 Installing system dependencies..."
sudo apt-get update -qq
sudo apt-get install -y -qq \
    build-essential curl git tmux htop \
    python3-pip python3-venv python3-dev \
    swi-prolog sqlite3 \
    pkg-config libssl-dev libfontconfig1-dev cmake

# ── 2. RUST TOOLCHAIN ──────────────────────────────────────────
if ! command -v rustup &> /dev/null; then
    echo "🦀 Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
rustup default stable
rustup update

# ── 3. LEAN 4 TOOLCHAIN ────────────────────────────────────────
if ! command -v elan &> /dev/null; then
    echo "📐 Installing Lean 4 (elan)..."
    curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh -s -- -y
    export PATH="$HOME/.elan/bin:$PATH"
fi

# ── 4. PYTHON ENVIRONMENT ──────────────────────────────────────
echo "🐍 Setting up Python virtual environment..."
python3 -m venv /opt/v6agent
source /opt/v6agent/bin/activate

pip install --upgrade pip
pip install --no-cache-dir \
    numpy scipy matplotlib sympy \
    google-generativeai google-cloud-storage \
    langgraph langgraph-checkpoint-sqlite \
    pydantic \
    vllm

# ── 5. CLONE THE REPO ──────────────────────────────────────────
echo "📂 Cloning rusty-SUNDIALS..."
if [ ! -d /opt/rusty-SUNDIALS ]; then
    git clone https://github.com/xaviercallens/rusty-SUNDIALS.git /opt/rusty-SUNDIALS
fi
cd /opt/rusty-SUNDIALS

# ── 6. NVIDIA DRIVER CHECK ─────────────────────────────────────
echo "🎮 Checking GPU..."
nvidia-smi || echo "⚠️ No GPU driver yet — deep learning image should have it"

# ── 7. DIRECTORIES ─────────────────────────────────────────────
mkdir -p /opt/rusty-SUNDIALS/discoveries
mkdir -p /opt/rusty-SUNDIALS/checkpoints

echo ""
echo "============================================================"
echo "  ✅ Bootstrap complete!"
echo "  Next: launch vLLM + orchestrator"
echo "============================================================"
