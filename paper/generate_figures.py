#!/usr/bin/env python3
"""Generate publication-quality figures for the rusty-SUNDIALS TOMS paper.

Usage:
    # 1. Generate data (from repo root):
    cargo run --example robertson_paper_data --features experimental-nls-v2 > paper/data_exp.csv 2>paper/summary_exp.txt
    cargo run --example robertson_paper_data > paper/data_stable.csv 2>paper/summary_stable.txt
    
    # 2. Generate figures:
    python3 paper/generate_figures.py

Produces:
    paper/fig1_step_size.pdf    — Step size adaptation over time
    paper/fig2_bdf_order.pdf    — BDF order selection over time
    paper/fig3_comparison.pdf   — Rust vs C reference bar chart
"""
import os
import sys
import csv
import matplotlib
matplotlib.use('Agg')  # Non-interactive backend
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

PAPER_DIR = os.path.dirname(os.path.abspath(__file__))

# ─── Publication style ────────────────────────────────────────────────────────
plt.rcParams.update({
    'font.family': 'serif',
    'font.serif': ['Times New Roman', 'DejaVu Serif', 'Georgia'],
    'font.size': 9,
    'axes.labelsize': 10,
    'axes.titlesize': 11,
    'xtick.labelsize': 8,
    'ytick.labelsize': 8,
    'legend.fontsize': 8,
    'figure.dpi': 300,
    'savefig.dpi': 300,
    'savefig.bbox': 'tight',
    'axes.grid': True,
    'grid.alpha': 0.3,
    'lines.linewidth': 1.2,
})

def load_csv(path):
    """Load step-by-step CSV data."""
    data = {'step': [], 't': [], 'h': [], 'order': [], 'nfe': [],
            'y1': [], 'y2': [], 'y3': [], 'conservation_error': []}
    with open(path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            for key in data:
                if key in ('step', 'order', 'nfe'):
                    data[key].append(int(row[key]))
                else:
                    data[key].append(float(row[key]))
    return {k: np.array(v) for k, v in data.items()}


def fig1_step_size(data, outpath):
    """Figure 1: Step size adaptation over time (log-log)."""
    fig, ax = plt.subplots(figsize=(5.5, 3.2))
    
    ax.loglog(data['t'], data['h'], 'b-', alpha=0.8, label='rusty-SUNDIALS (BDF)')
    ax.set_xlabel('Time $t$')
    ax.set_ylabel('Step size $h$')
    ax.set_title('Step Size Adaptation — Robertson Chemical Kinetics')
    ax.set_xlim(1e-10, 5e10)
    ax.legend(loc='upper left')
    
    # Annotate transient region
    ax.axvspan(1e-10, 1e-3, alpha=0.05, color='red')
    ax.text(1e-7, ax.get_ylim()[0] * 5, 'Transient\n(stiff)', fontsize=7,
            ha='center', color='red', alpha=0.7)
    ax.text(1e5, ax.get_ylim()[1] * 0.3, 'Steady state\n(large steps)', fontsize=7,
            ha='center', color='blue', alpha=0.7)
    
    fig.tight_layout()
    fig.savefig(outpath)
    plt.close(fig)
    print(f'  ok {outpath}')


def fig2_bdf_order(data, outpath):
    """Figure 2: BDF order selection over time."""
    fig, ax = plt.subplots(figsize=(5.5, 2.5))
    
    ax.semilogx(data['t'], data['order'], 'g-', alpha=0.8, drawstyle='steps-post')
    ax.set_xlabel('Time $t$')
    ax.set_ylabel('BDF Order $q$')
    ax.set_title('Adaptive Order Selection — Robertson Chemical Kinetics')
    ax.set_ylim(0.5, 5.5)
    ax.set_yticks([1, 2, 3, 4, 5])
    ax.set_xlim(1e-10, 5e10)
    
    fig.tight_layout()
    fig.savefig(outpath)
    plt.close(fig)
    print(f'  ok {outpath}')


def fig3_comparison_bar(outpath):
    """Figure 3: Rust V3 vs C reference — bar chart comparison."""
    fig, axes = plt.subplots(1, 4, figsize=(7.0, 2.8))
    
    metrics = ['Steps', 'Newton Iters', 'NI/step', 'Conservation\nError']
    rust_vals = [1076, 1503, 1.40, 8.88e-16]
    c_vals = [1070, 1537, 1.44, 1.1e-15]
    
    colors_rust = ['#2196F3', '#2196F3', '#2196F3', '#2196F3']  # Blue
    colors_c = ['#FF9800', '#FF9800', '#FF9800', '#FF9800']      # Orange
    
    for i, (ax, metric, rv, cv) in enumerate(zip(axes, metrics, rust_vals, c_vals)):
        x = np.array([0, 1])
        bars = ax.bar(x, [rv, cv], width=0.6, color=[colors_rust[i], colors_c[i]],
                      edgecolor='white', linewidth=0.5)
        ax.set_xticks(x)
        ax.set_xticklabels(['Rust', 'C ref'], fontsize=7)
        ax.set_title(metric, fontsize=8, fontweight='bold')
        
        # Value labels
        for bar, val in zip(bars, [rv, cv]):
            if val < 1e-10:
                label = f'{val:.1e}'
            elif val < 10:
                label = f'{val:.2f}'
            else:
                label = f'{val:,.0f}'
            ax.text(bar.get_x() + bar.get_width()/2, bar.get_height(),
                    label, ha='center', va='bottom', fontsize=6.5)
        
        # Ratio annotation
        ratio = rv / cv
        if ratio < 1:
            badge = f'{ratio:.2f}× ok'
            color = '#4CAF50'
        elif ratio <= 1.01:
            badge = f'{ratio:.3f}×'
            color = '#4CAF50'
        else:
            badge = f'{ratio:.2f}×'
            color = '#F44336'
        ax.set_xlabel(badge, fontsize=7, color=color, fontweight='bold')
        
        ax.spines['top'].set_visible(False)
        ax.spines['right'].set_visible(False)
    
    fig.suptitle('rusty-SUNDIALS v11.5.0 vs. LLNL SUNDIALS 7.4.0 (C)',
                 fontsize=10, fontweight='bold', y=1.02)
    fig.tight_layout()
    fig.savefig(outpath)
    plt.close(fig)
    print(f'  ok {outpath}')


def fig4_evolution(outpath):
    """Figure 4: Evolution of RHS evaluations across versions."""
    fig, ax = plt.subplots(figsize=(5.5, 3.0))
    
    versions = ['v11.1.0\nFD Jac', 'v11.2.0\nAnalyt.', 'v11.3.0\nH1-H3',
                'v11.4.0\nH4-H6', 'v11.5.0\nH7-H8']
    rhs = [74778, 2707, 2536, 2603, 2602]
    steps = [16951, 960, 903, 1076, 1076]
    
    x = np.arange(len(versions))
    width = 0.35
    
    bars1 = ax.bar(x - width/2, rhs, width, label='RHS Evaluations', color='#2196F3', alpha=0.85)
    bars2 = ax.bar(x + width/2, steps, width, label='Steps', color='#4CAF50', alpha=0.85)
    
    # C reference lines
    ax.axhline(y=1537, color='#FF9800', linestyle='--', linewidth=1.5, label='C ref RHS (1537)')
    ax.axhline(y=1070, color='#8BC34A', linestyle=':', linewidth=1.5, label='C ref Steps (1070)')
    
    ax.set_xticks(x)
    ax.set_xticklabels(versions, fontsize=7)
    ax.set_ylabel('Count')
    ax.set_title('Solver Performance Evolution — Auto-Research v11.x')
    ax.set_yscale('log')
    ax.legend(fontsize=7, loc='upper right')
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.tight_layout()
    fig.savefig(outpath)
    plt.close(fig)
    print(f'  ok {outpath}')


if __name__ == '__main__':
    print('Generating paper figures...')
    
    # Figures that don't need CSV data (from hardcoded results)
    fig3_comparison_bar(os.path.join(PAPER_DIR, 'fig3_comparison.pdf'))
    fig4_evolution(os.path.join(PAPER_DIR, 'fig4_evolution.pdf'))
    
    # Figures that need CSV data (skip if data not available)
    csv_exp = os.path.join(PAPER_DIR, 'data_exp.csv')
    if os.path.exists(csv_exp):
        data = load_csv(csv_exp)
        fig1_step_size(data, os.path.join(PAPER_DIR, 'fig1_step_size.pdf'))
        fig2_bdf_order(data, os.path.join(PAPER_DIR, 'fig2_bdf_order.pdf'))
    else:
        print(f'  ⚠ {csv_exp} not found — skipping Fig 1-2')
        print(f'    Run: cargo run --example robertson_paper_data --features experimental-nls-v2 > paper/data_exp.csv')
    
    # Also generate PNG versions for the web
    for pdf in ['fig3_comparison.pdf', 'fig4_evolution.pdf']:
        pdfpath = os.path.join(PAPER_DIR, pdf)
        pngpath = pdfpath.replace('.pdf', '.png')
        if os.path.exists(pdfpath):
            fig = plt.figure()
            # Re-generate as PNG
            pass  # PDFs are sufficient for the paper
    
    print('Done.')
