import argparse
import pandas as pd
import numpy as np
import gudhi
import matplotlib.pyplot as plt
import os

def load_data(filepath, columns=None):
    """
    Load data from a CSV file.
    If columns is specified, extract only those columns as the point cloud.
    """
    df = pd.read_csv(filepath)
    if columns:
        cols = columns.split(',')
        # Check if all columns are present
        missing = [c for c in cols if c not in df.columns]
        if missing:
            raise ValueError(f"Columns {missing} not found in {filepath}")
        point_cloud = df[cols].values
    else:
        point_cloud = df.values
    return df, point_cloud

def compute_tda(point_cloud, max_edge_length, max_dimension):
    """
    Build a Vietoris-Rips complex and compute persistent homology.
    """
    print(f"Building Vietoris-Rips complex (max edge: {max_edge_length}, max dim: {max_dimension})...")
    rips_complex = gudhi.RipsComplex(points=point_cloud, max_edge_length=max_edge_length)
    simplex_tree = rips_complex.create_simplex_tree(max_dimension=max_dimension)
    
    print(f"Simplex tree created: {simplex_tree.num_simplices()} simplices, {simplex_tree.num_vertices()} vertices.")
    
    print("Computing persistent homology...")
    diag = simplex_tree.persistence()
    betti_numbers = simplex_tree.betti_numbers()
    
    return simplex_tree, diag, betti_numbers

def plot_persistence(diag, output_dir, prefix):
    """
    Plot persistence diagram and barcode.
    """
    os.makedirs(output_dir, exist_ok=True)
    
    plt.figure()
    gudhi.plot_persistence_diagram(diag)
    plt.title(f"{prefix} - Persistence Diagram")
    plt.tight_layout()
    diagram_path = os.path.join(output_dir, f"{prefix}_persistence_diagram.png")
    plt.savefig(diagram_path, dpi=300)
    plt.close()
    
    plt.figure()
    gudhi.plot_persistence_barcode(diag)
    plt.title(f"{prefix} - Persistence Barcode")
    plt.tight_layout()
    barcode_path = os.path.join(output_dir, f"{prefix}_persistence_barcode.png")
    plt.savefig(barcode_path, dpi=300)
    plt.close()
    
    print(f"Saved plots to {output_dir}/")
    return diagram_path, barcode_path

def main():
    parser = argparse.ArgumentParser(description="Generic TDA Pipeline using INRIA's GUDHI.")
    parser.add_argument('--input', type=str, required=True, help="Input CSV file")
    parser.add_argument('--columns', type=str, help="Comma-separated list of columns to use as the point cloud (e.g., 'x,y,rho,p')")
    parser.add_argument('--max-edge', type=float, default=2.0, help="Maximum edge length for the Rips complex")
    parser.add_argument('--max-dim', type=int, default=2, help="Maximum dimension of the Rips complex")
    parser.add_argument('--output-dir', type=str, default='tda_output', help="Directory to save output plots")
    parser.add_argument('--prefix', type=str, default='tda', help="Prefix for output filenames")
    
    args = parser.parse_args()
    
    print(f"--- TDA Pipeline: {args.input} ---")
    df, point_cloud = load_data(args.input, args.columns)
    print(f"Loaded {point_cloud.shape[0]} points in {point_cloud.shape[1]} dimensions.")
    
    # Subsample to avoid OOM on large datasets (e.g. if simulation stuck at attractor)
    max_points = 2000
    if len(point_cloud) > max_points:
        indices = np.linspace(0, len(point_cloud) - 1, max_points, dtype=int)
        point_cloud = point_cloud[indices]
        print(f"Subsampled to {len(point_cloud)} points for TDA.")
    
    # Normalize data for better distance calculations in TDA
    pc_min = point_cloud.min(axis=0)
    pc_max = point_cloud.max(axis=0)
    # Avoid division by zero for constant columns
    range_val = pc_max - pc_min
    range_val[range_val == 0] = 1.0
    normalized_pc = (point_cloud - pc_min) / range_val
    print("Point cloud normalized.")
    
    simplex_tree, diag, betti_numbers = compute_tda(normalized_pc, args.max_edge, args.max_dim)
    
    print("\n--- Topological Signatures ---")
    print(f"Betti numbers: {betti_numbers}")
    
    # Analyze the most persistent features
    print("\nTop 5 most persistent features (Dimension, (Birth, Death)):")
    # Filter out features with infinite death for sorting
    finite_diag = [d for d in diag if d[1][1] != float('inf')]
    sorted_diag = sorted(finite_diag, key=lambda x: x[1][1] - x[1][0], reverse=True)
    
    for i, feature in enumerate(sorted_diag[:5]):
        dim, (birth, death) = feature
        persistence = death - birth
        print(f"  Feature {i+1}: Dim {dim}, Birth: {birth:.4f}, Death: {death:.4f}, Persistence: {persistence:.4f}")
        
    diag_path, barcode_path = plot_persistence(diag, args.output_dir, args.prefix)
    print("--- TDA Pipeline Complete ---")

if __name__ == "__main__":
    main()
