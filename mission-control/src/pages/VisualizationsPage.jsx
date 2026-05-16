import React, { useState, useEffect } from 'react';
import { Layers, Activity, Server, Zap, ChevronRight, BarChart2 } from 'lucide-react';

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8000';

export default function VisualizationsPage() {
  const [visualizations, setVisualizations] = useState([]);
  const [loading, setLoading] = useState(true);
  const [activeViz, setActiveViz] = useState(null);

  useEffect(() => {
    fetch(`${API_BASE}/api/visualizations`)
      .then(res => res.json())
      .then(data => {
        setVisualizations(data);
        if (data.length > 0) setActiveViz(data[0]);
        setLoading(false);
      })
      .catch(err => {
        console.error("Failed to fetch visualizations", err);
        setLoading(false);
      });
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-xl text-slate-400 animate-pulse">Loading Visualizations...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 to-blue-500">
            Scientific Visualizations
          </h1>
          <p className="text-slate-400 mt-2">
            Explore high-fidelity datasets rendered directly from rusty-SUNDIALS PDE/ODE integration outputs.
          </p>
        </div>
        <button className="px-4 py-2 bg-slate-800 border border-slate-700 rounded-lg hover:bg-slate-700 transition flex items-center gap-2">
          <Server className="w-4 h-4 text-cyan-400" />
          <span>Connect Local VTK Server</span>
        </button>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Sidebar List */}
        <div className="lg:col-span-1 space-y-4">
          <h2 className="text-xl font-semibold border-b border-slate-800 pb-2">Available Results</h2>
          {visualizations.map((viz) => (
            <div 
              key={viz.id} 
              onClick={() => setActiveViz(viz)}
              className={`p-4 rounded-xl cursor-pointer transition border ${
                activeViz?.id === viz.id 
                  ? 'bg-slate-800/80 border-cyan-500/50 shadow-[0_0_15px_rgba(6,182,212,0.1)]' 
                  : 'bg-slate-900/50 border-slate-800 hover:border-slate-700'
              }`}
            >
              <h3 className="font-medium text-lg mb-1">{viz.title}</h3>
              <div className="flex flex-wrap gap-2 mt-3">
                {viz.tags.map(tag => (
                  <span key={tag} className="text-xs px-2 py-1 bg-slate-800 rounded-md text-cyan-400 border border-cyan-900/30">
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Main View Area */}
        <div className="lg:col-span-2">
          {activeViz ? (
            <div className="bg-slate-900/50 border border-slate-800 rounded-xl overflow-hidden flex flex-col h-full">
              <div className="p-6 border-b border-slate-800">
                <h2 className="text-2xl font-bold">{activeViz.title}</h2>
                <p className="text-slate-400 mt-3 leading-relaxed">
                  {activeViz.description}
                </p>
              </div>
              
              <div className="p-6 flex-1 bg-slate-950/50">
                {activeViz.images && activeViz.images.length > 0 ? (
                  <div className="space-y-8">
                    {/* Hero Image */}
                    <div className="rounded-lg overflow-hidden border border-slate-800 shadow-2xl relative group">
                      <div className="absolute top-4 left-4 bg-black/70 backdrop-blur px-3 py-1 rounded text-xs font-mono border border-slate-700 z-10 flex items-center gap-2">
                        <Activity className="w-3 h-3 text-red-400" />
                        Te Collapse / Plasma Current
                      </div>
                      <img 
                        src={`${API_BASE}${activeViz.images[0]}`} 
                        alt="Hero Visualization" 
                        className="w-full h-auto object-cover transform transition duration-700 group-hover:scale-[1.02]"
                      />
                    </div>

                    {/* Secondary Images Grid */}
                    <h3 className="text-lg font-medium border-b border-slate-800 pb-2 mt-8 flex items-center gap-2">
                      <Layers className="w-5 h-5 text-cyan-400" /> Output Stages
                    </h3>
                    <div className="grid grid-cols-2 gap-4">
                      {activeViz.images.slice(1).map((img, idx) => (
                        <div key={idx} className="rounded-lg overflow-hidden border border-slate-800 bg-black">
                           <img 
                            src={`${API_BASE}${img}`} 
                            alt={`Stage ${idx+1}`} 
                            className="w-full h-auto object-cover opacity-80 hover:opacity-100 transition cursor-pointer"
                           />
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center h-64 text-slate-500">
                    <BarChart2 className="w-12 h-12 mb-3 opacity-50" />
                    <p>No visual artifacts available.</p>
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full border border-slate-800 rounded-xl bg-slate-900/30 border-dashed">
              <p className="text-slate-500">Select a visualization from the sidebar.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
