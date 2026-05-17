#!/usr/bin/env python3
"""
Generate 3D Video Visualization from Open Dataset
Reads the 7 CSV time steps from the 3D dataset, interpolates to 60 frames,
and uses ffmpeg to generate a smooth video of the plasma disruption.
"""
import numpy as np
import os
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm
from scipy.interpolate import interp1d
import subprocess

PROJECT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_3D = os.path.join(PROJECT, "data", "fusion", "rust_sim_output_3d")
OUT = os.path.join(PROJECT, "data", "fusion", "vtk_output_3d")
os.makedirs(OUT, exist_ok=True)

# ITER Parameters
R0, a, kappa, delta_shape = 6.2, 2.0, 1.7, 0.33
N_RHO, N_THETA, N_PHI = 100, 200, 16
Te0 = 25e3

def shaped_boundary(theta, r_frac=1.0):
    R = R0 + r_frac * a * np.cos(theta + delta_shape * np.sin(theta))
    Z = r_frac * kappa * a * np.sin(theta)
    return R, Z

def load_all_data():
    times = [0.0, 0.3, 0.4, 0.5, 0.7, 0.9, 1.0]
    # Initialize array: [t, phi, rho, theta]
    Te_all = np.zeros((len(times), N_PHI, N_RHO, N_THETA))
    
    for i, t in enumerate(times):
        csv_path = os.path.join(DATA_3D, f"iter_3d_t{t:.2f}.csv")
        print(f"Loading {csv_path}...")
        with open(csv_path, 'r') as f:
            next(f)
            for line in f:
                parts = line.split(',')
                if parts[0] == 'plasma':
                    ir, it, ip = int(parts[1]), int(parts[2]), int(parts[3])
                    Te_all[i, ip, ir, it] = float(parts[4])
    return times, Te_all

def generate_video():
    print("Loading open dataset...")
    times, Te_all = load_all_data()
    
    # Interpolate for smooth video (60 frames)
    print("Interpolating data to 60 frames...")
    interpolator = interp1d(times, Te_all, axis=0, kind='linear')
    frame_times = np.linspace(0.0, 1.0, 60)
    Te_smooth = interpolator(frame_times)
    
    # Grid setup for poloidal cross-section (phi=0)
    rho = np.linspace(0.01, 1.0, N_RHO)
    theta = np.linspace(0, 2*np.pi, N_THETA, endpoint=False)
    RHO, THETA = np.meshgrid(rho, theta, indexing='ij')
    R = np.zeros_like(RHO)
    Z = np.zeros_like(RHO)
    for i in range(N_RHO):
        R[i,:], Z[i,:] = shaped_boundary(theta, rho[i])
        
    frames_dir = os.path.join(OUT, "frames")
    os.makedirs(frames_dir, exist_ok=True)
    
    print("Rendering frames...")
    for f_idx, t in enumerate(frame_times):
        fig, ax = plt.subplots(1, 1, figsize=(8, 10))
        fig.patch.set_facecolor('#0a0e17')
        ax.set_facecolor('#0a0e17')
        
        Te_frame = np.clip(Te_smooth[f_idx, 0, :, :], 2.0, None)
        
        cf = ax.pcolormesh(R, Z, Te_frame, cmap='inferno',
                           norm=LogNorm(vmin=2, vmax=25000), shading='gouraud')
                           
        cbar = plt.colorbar(cf, ax=ax, label='Electron Temperature $T_e$ [eV]', shrink=0.8, pad=0.02)
        cbar.ax.yaxis.label.set_color('#e0e0e0')
        cbar.ax.tick_params(colors='#e0e0e0')
        
        ax.set_xlabel('R [m]', color='#e0e0e0')
        ax.set_ylabel('Z [m]', color='#e0e0e0')
        ax.tick_params(colors='#e0e0e0')
        ax.set_aspect('equal')
        
        # Add timestamp and title
        ax.set_title(f"ITER 3D Disruption (φ=0°)\nTime: {t*1000:.0f} ms", color='#00ff88', fontsize=14)
        
        # Vessel wall
        theta_v = np.linspace(0, 2*np.pi, 200)
        Rv, Zv = shaped_boundary(theta_v, 1.12)
        ax.plot(Rv, Zv, 'w-', linewidth=1.5, alpha=0.6)
        
        frame_path = os.path.join(frames_dir, f"frame_{f_idx:04d}.png")
        plt.savefig(frame_path, dpi=120, bbox_inches='tight', facecolor=fig.get_facecolor())
        plt.close()
        if f_idx % 10 == 0:
            print(f"  Rendered {f_idx}/60")
            
    print("Generating mp4 video using ffmpeg...")
    video_path = os.path.join(OUT, "iter_3d_disruption.mp4")
    cmd = [
        "ffmpeg", "-y", "-framerate", "30", "-i", os.path.join(frames_dir, "frame_%04d.png"),
        "-c:v", "libx264", "-pix_fmt", "yuv420p", video_path
    ]
    subprocess.run(cmd, check=True)
    print(f"✅ Video saved to {video_path}")

if __name__ == '__main__':
    generate_video()
