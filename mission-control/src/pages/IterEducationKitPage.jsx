import React from 'react';
import { Shield, Zap, Activity, Cpu, Database, Network, Fingerprint, Layers, Maximize, PlayCircle } from 'lucide-react';

const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:8080';

export default function IterEducationKitPage() {
  return (
    <div className="flex-1 min-h-screen bg-[#050505] text-cyan-50 font-sans overflow-y-auto overflow-x-hidden relative">
      {/* Background Grid Pattern (Iron Man HUD style) */}
      <div className="fixed inset-0 pointer-events-none opacity-[0.03]" 
           style={{ backgroundImage: 'linear-gradient(#00e5ff 1px, transparent 1px), linear-gradient(90deg, #00e5ff 1px, transparent 1px)', backgroundSize: '40px 40px' }}>
      </div>

      <div className="p-8 max-w-7xl mx-auto relative z-10">
        
        {/* Header Section */}
        <div className="flex justify-between items-end mb-10 border-b border-cyan-500/30 pb-6">
          <div>
            <div className="flex items-center gap-3 text-cyan-400 mb-2">
              <Shield className="w-6 h-6" />
              <span className="font-mono tracking-widest text-sm uppercase">Secure Terminal / Project ITER</span>
            </div>
            <h1 className="text-5xl font-black tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-cyan-300 via-blue-500 to-purple-600 drop-shadow-lg">
              ITER Educational Kit
            </h1>
            <p className="text-slate-400 mt-2 font-mono text-sm max-w-2xl">
              Declassified Briefing: AI-Accelerated Scientific Machine Learning (SciML) for Magnetic Confinement Fusion Disruption Mitigation.
            </p>
          </div>
          <div className="text-right hidden md:block">
            <div className="font-mono text-xs text-cyan-500/70 mb-1">SYSTEM STATUS</div>
            <div className="flex items-center gap-2 text-green-400 font-mono">
              <span className="relative flex h-3 w-3">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-3 w-3 bg-green-500"></span>
              </span>
              ONLINE / SECURE
            </div>
          </div>
        </div>

        {/* Main Dashboard Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 mb-12">
          
          {/* Main Video Display (Iron Man Arc Reactor vibe) */}
          <div className="lg:col-span-2 relative">
            <div className="absolute -inset-1 bg-gradient-to-r from-cyan-500 to-purple-600 rounded-2xl blur opacity-30 animate-pulse"></div>
            <div className="relative bg-black rounded-xl border border-cyan-500/50 overflow-hidden shadow-2xl h-[500px]">
              
              {/* HUD Overlays on Video */}
              <div className="absolute top-4 left-4 z-20 flex gap-2">
                <span className="px-2 py-1 bg-black/60 border border-cyan-500 text-cyan-400 font-mono text-[10px] rounded">LIVE FEED</span>
                <span className="px-2 py-1 bg-red-500/20 border border-red-500 text-red-400 font-mono text-[10px] rounded flex items-center gap-1">
                  <Activity className="w-3 h-3" /> DISRUPTION DETECTED
                </span>
              </div>
              
              <div className="absolute bottom-4 right-4 z-20">
                <div className="w-32 h-16 border border-cyan-500/30 bg-black/40 backdrop-blur-sm p-2 flex flex-col justify-between">
                  <div className="text-[8px] text-cyan-500 font-mono">THERMAL QUENCH</div>
                  <div className="w-full bg-slate-800 h-1 mt-1"><div className="bg-cyan-400 h-full w-[85%]"></div></div>
                  <div className="w-full bg-slate-800 h-1 mt-1"><div className="bg-purple-500 h-full w-[40%]"></div></div>
                </div>
              </div>

              {/* The Video */}
              <video 
                src={`${API_BASE}/static/data/fusion/vtk_output_3d/iter_3d_torus_video.mp4`}
                autoPlay 
                loop 
                muted 
                playsInline
                className="w-full h-full object-cover mix-blend-screen"
              />
              
              {/* Scanline overlay */}
              <div className="absolute inset-0 pointer-events-none bg-[url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI0IiBoZWlnaHQ9IjQiPjxyZWN0IHdpZHRoPSI0IiBoZWlnaHQ9IjIiIGZpbGw9InJnYmEoMCwyNTUsMjU1LDAuMDUpIi8+PC9zdmc+')] opacity-50"></div>
            </div>
          </div>

          {/* Right Side Stats Panel */}
          <div className="flex flex-col gap-4">
            <div className="bg-[#0a0f18] border border-cyan-500/30 rounded-xl p-6 shadow-[0_0_15px_rgba(0,229,255,0.1)]">
              <h3 className="text-cyan-400 font-mono text-sm tracking-widest mb-4 flex items-center gap-2">
                <Database className="w-4 h-4" /> CORE TELEMETRY
              </h3>
              <div className="space-y-4">
                <div>
                  <div className="text-xs text-slate-500 font-mono mb-1">PLASMA CURRENT (Ip)</div>
                  <div className="text-2xl font-bold text-white font-mono">15.0 <span className="text-sm text-cyan-500">MA</span></div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 font-mono mb-1">MAJOR RADIUS (R0)</div>
                  <div className="text-2xl font-bold text-white font-mono">6.2 <span className="text-sm text-cyan-500">m</span></div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 font-mono mb-1">MAGNETIC ENERGY</div>
                  <div className="text-2xl font-bold text-white font-mono">~350 <span className="text-sm text-cyan-500">MJ</span></div>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-br from-indigo-900/40 to-purple-900/20 border border-purple-500/30 rounded-xl p-6 flex-1 flex flex-col justify-center">
              <h3 className="text-purple-400 font-mono text-sm tracking-widest mb-2 flex items-center gap-2">
                <Fingerprint className="w-4 h-4" /> THREAT ANALYSIS
              </h3>
              <p className="text-sm text-slate-300 leading-relaxed">
                The tearing mode <strong className="text-white">m=2, n=1</strong> instability causes magnetic islands to overlap, leading to a catastrophic Thermal Quench (TQ). The plasma loses its thermal energy in milliseconds, risking severe damage to the Tokamak walls.
              </p>
            </div>
          </div>
        </div>

        {/* Detailed Explanation Sections */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 mb-12">
          
          {/* Section 1 */}
          <div className="bg-[#0a0e14] border-l-4 border-cyan-500 p-8 rounded-r-xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-cyan-500/10 rounded-lg"><Network className="w-6 h-6 text-cyan-400" /></div>
              <h2 className="text-2xl font-bold text-white">The SciML Solution</h2>
            </div>
            <p className="text-slate-400 mb-4 leading-relaxed">
              To mitigate disruptions, we must simulate them faster than real-time. Traditional Magnetohydrodynamics (MHD) solvers are bottlenecked by dense Jacobian matrices and immense computational overhead.
            </p>
            <p className="text-slate-400 leading-relaxed">
              <strong className="text-cyan-400">rusty-SUNDIALS</strong> integrates a novel <em>Neural-FGMRES</em> preconditioner. By using a Graph Neural Network (MPNN) to approximate the inverse Jacobian, we bypass the $O(N^3)$ computational wall, achieving a 150x speedup while maintaining strict mathematical guarantees via Lean 4 formal verification.
            </p>
          </div>

          {/* Section 2 */}
          <div className="bg-[#0a0e14] border-l-4 border-orange-500 p-8 rounded-r-xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-orange-500/10 rounded-lg"><Cpu className="w-6 h-6 text-orange-400" /></div>
              <h2 className="text-2xl font-bold text-white">Auto-Research & Edge AI</h2>
            </div>
            <p className="text-slate-400 mb-4 leading-relaxed">
              The AI doesn't just solve equations; it writes them. The <strong>Auto-Research</strong> orchestrator continuously benchmarks architectures (FNO vs MPNN) and adaptive precisions (FP8 to FP32) on Google Cloud GPUs.
            </p>
            <div className="bg-black/50 p-4 rounded border border-slate-800 font-mono text-xs text-orange-200">
              <div className="flex justify-between border-b border-slate-800 pb-2 mb-2">
                <span>> OPTIMIZATION TARGET:</span>
                <span className="text-orange-400">NEWTON ITERS &lt; 5</span>
              </div>
              <div className="flex justify-between border-b border-slate-800 pb-2 mb-2">
                <span>> ADAPTIVE TRAJECTORY:</span>
                <span className="text-cyan-400">FP8 → FP16 → FP32</span>
              </div>
              <div className="flex justify-between">
                <span>> HARDWARE ABLATION:</span>
                <span className="text-green-400">H100 TENSOR CORES ACTIVE</span>
              </div>
            </div>
          </div>

        </div>

        {/* Footer / Schematics Mockup */}
        <div className="border border-slate-800 rounded-xl p-8 bg-[url('https://www.iter.org/img/blueprint_bg.jpg')] bg-cover bg-center relative overflow-hidden">
          <div className="absolute inset-0 bg-[#050505]/90"></div>
          <div className="relative z-10 flex flex-col md:flex-row items-center justify-between gap-6">
            <div>
              <h2 className="text-xl font-bold text-white mb-2 flex items-center gap-2">
                <Layers className="text-cyan-500 w-5 h-5" /> Open Source Mission
              </h2>
              <p className="text-slate-400 max-w-xl text-sm">
                This simulation dataset and visualization pipeline is completely open-source. By combining Rust's safety, Lean 4's formal proofs, and PyTorch's ML capabilities, we are democratizing fusion energy research for independent scientists worldwide.
              </p>
            </div>
            <div className="flex gap-4">
              <button className="px-6 py-3 bg-cyan-600 hover:bg-cyan-500 text-white font-bold rounded-lg transition-colors flex items-center gap-2">
                <PlayCircle className="w-5 h-5" /> View Datasets
              </button>
              <button className="px-6 py-3 bg-transparent border border-cyan-600 text-cyan-400 hover:bg-cyan-900/30 font-bold rounded-lg transition-colors flex items-center gap-2">
                <Maximize className="w-5 h-5" /> Full Screen HUD
              </button>
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}
