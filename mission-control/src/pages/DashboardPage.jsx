import { useState, useEffect, useCallback } from 'react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  BarChart, Bar, ResponsiveContainer, Legend
} from 'recharts';
import { Play, Zap, Clock, Award, TrendingDown } from 'lucide-react';
import GlowPanel from '../components/GlowPanel';
import PipelineGraph from '../components/PipelineGraph';
import api from '../api/client';

/* ── Mock telemetry for demo ──────────────────────────────── */
function genTelemetry(n = 50) {
  return Array.from({ length: n }, (_, i) => ({
    t: i,
    energyDrift: Math.random() * 0.04 * Math.exp(-i * 0.03) + 1e-16,
    krylov: Math.floor(Math.random() * 12) + 2,
    cpuLoad: 40 + Math.random() * 50,
  }));
}

const DISCOVERIES = [
  { id: 1, name: 'AsynchronousGeometricIntegrator', status: 'verified', cert: 'CERT-LEAN4-A72F', speedup: '2.3×10¹⁴' },
  { id: 2, name: 'SpectralLagrangianDecomposition', status: 'verified', cert: 'CERT-LEAN4-B93E', speedup: '5000×' },
  { id: 3, name: 'NeuroGeometricOperatorDecomp', status: 'verified', cert: 'CERT-LEAN4-C14D', speedup: '1.2×10¹⁴' },
  { id: 4, name: 'HamiltonianPrimalDualIntegrator', status: 'verified', cert: 'CERT-LEAN4-D58A', speedup: '890×' },
  { id: 5, name: 'GeometricNeuralPreconditioner', status: 'pending', cert: '—', speedup: '—' },
  { id: 6, name: 'SymplecticEnergyProjection', status: 'verified', cert: 'CERT-LEAN4-E3B1', speedup: '4.7×10¹⁴' },
];

const SWEEP_DATA = [
  { S: 'S=10', baseline: 0.059, projected: 2.57e-16, improvement: 2.3e14 },
  { S: 'S=100', baseline: 0.060, projected: 1e-18, improvement: 6e16 },
  { S: 'S=200', baseline: 0.060, projected: 1e-18, improvement: 6e16 },
  { S: 'S=1000', baseline: 0.060, projected: 1.29e-16, improvement: 4.7e14 },
];

export default function DashboardPage() {
  const [telemetry, setTelemetry] = useState(genTelemetry());
  const [activeNode, setActiveNode] = useState('synthesize');
  const [loading, setLoading] = useState(false);
  const [runResult, setRunResult] = useState(null);

  // Animate telemetry
  useEffect(() => {
    const iv = setInterval(() => {
      setTelemetry(prev => {
        const next = [...prev.slice(1)];
        const last = prev[prev.length - 1];
        next.push({
          t: last.t + 1,
          energyDrift: Math.random() * 0.01 + 1e-16,
          krylov: Math.floor(Math.random() * 10) + 2,
          cpuLoad: 40 + Math.random() * 50,
        });
        return next;
      });
    }, 2000);
    return () => clearInterval(iv);
  }, []);

  const handleRun = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.runPipeline({ max_loops: 1 });
      setRunResult(res);
      setActiveNode('publish');
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  return (
    <div>
      <div className="page-header">
        <h2>MISSION DASHBOARD</h2>
        <button className="btn btn-primary" onClick={handleRun} disabled={loading}>
          <Play size={14} />
          {loading ? 'EXECUTING...' : 'RUN LOOP'}
        </button>
      </div>

      {/* Metrics Row */}
      <div className="grid-4" style={{ marginBottom: 'var(--gap-lg)' }}>
        <MetricCard icon={Zap} label="Discoveries" value="6" delta="+2 today" positive />
        <MetricCard icon={Award} label="Verified" value="5" delta="83% rate" positive />
        <MetricCard icon={TrendingDown} label="Best Drift" value="1.29e-16" delta="machine ε" positive />
        <MetricCard icon={Clock} label="Uptime" value="4h 12m" delta="5 runs" />
      </div>

      {/* Main 2x2 Grid */}
      <div className="grid-2x2">
        <GlowPanel title="PIPELINE STATUS">
          <PipelineGraph activeNode={activeNode} />
        </GlowPanel>

        <GlowPanel title="LIVE TELEMETRY">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <MiniChart data={telemetry} dataKey="energyDrift" color="#00e5ff" label="Energy Drift |ΔE/E₀|" />
            <MiniChart data={telemetry} dataKey="krylov" color="#ffb800" label="Krylov Iterations" />
            <MiniChart data={telemetry} dataKey="cpuLoad" color="#a78bfa" label="CPU Utilization %" />
          </div>
        </GlowPanel>

        <GlowPanel title="DISCOVERY LOG">
          <div style={{ overflowX: 'auto' }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th>Method</th>
                  <th>Status</th>
                  <th>Certificate</th>
                  <th>Speedup</th>
                </tr>
              </thead>
              <tbody>
                {DISCOVERIES.map(d => (
                  <tr key={d.id}>
                    <td style={{ color: 'var(--text-primary)', maxWidth: 180, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {d.name}
                    </td>
                    <td><span className={`badge ${d.status}`}>{d.status}</span></td>
                    <td style={{ color: 'var(--text-secondary)' }}>{d.cert}</td>
                    <td style={{ color: 'var(--green)' }}>{d.speedup}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </GlowPanel>

        <GlowPanel title="STIFFNESS SWEEP">
          <ResponsiveContainer width="100%" height={220}>
            <BarChart data={SWEEP_DATA} barGap={2}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1a2744" />
              <XAxis dataKey="S" tick={{ fill: '#7a8ba8', fontSize: 11, fontFamily: 'JetBrains Mono' }} />
              <YAxis tick={{ fill: '#7a8ba8', fontSize: 10, fontFamily: 'JetBrains Mono' }}
                tickFormatter={v => v >= 0.01 ? `${(v * 100).toFixed(0)}%` : v.toExponential(0)} />
              <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8, fontFamily: 'JetBrains Mono', fontSize: 12 }}
                labelStyle={{ color: '#00e5ff' }} />
              <Legend wrapperStyle={{ fontFamily: 'JetBrains Mono', fontSize: 11 }} />
              <Bar dataKey="baseline" fill="#ff3d5a" name="Baseline BDF" radius={[4, 4, 0, 0]} />
              <Bar dataKey="projected" fill="#00ff88" name="+ Projection" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </GlowPanel>
      </div>

      {/* Run result toast */}
      {runResult && (
        <div style={{
          position: 'fixed', bottom: 24, right: 24, padding: '16px 24px',
          background: 'var(--bg-surface)', border: '1px solid var(--green-dim)',
          borderRadius: 'var(--radius-md)', boxShadow: 'var(--glow-green)',
          fontFamily: 'JetBrains Mono', fontSize: '0.85rem', color: 'var(--green)',
          animation: 'fade-in-up 0.4s ease-out',
          zIndex: 1000,
        }}>
          ✓ Discovery: {runResult.discovery?.method_name || 'Complete'} — ${runResult.estimated_cost_usd?.toFixed(4)}
          <button onClick={() => setRunResult(null)}
            style={{ marginLeft: 16, background: 'none', border: 'none', color: 'var(--text-tertiary)', cursor: 'pointer' }}>✕</button>
        </div>
      )}
    </div>
  );
}

function MetricCard({ icon: Icon, label, value, delta, positive }) {
  return (
    <div className="metric-card animate-in">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span className="metric-label">{label}</span>
        <Icon size={16} style={{ color: 'var(--text-tertiary)' }} />
      </div>
      <span className="metric-value">{value}</span>
      {delta && <span className={`metric-delta ${positive ? 'positive' : ''}`}>{delta}</span>}
    </div>
  );
}

function MiniChart({ data, dataKey, color, label }) {
  return (
    <div>
      <div className="label" style={{ marginBottom: 2 }}>{label}</div>
      <ResponsiveContainer width="100%" height={50}>
        <LineChart data={data}>
          <Line type="monotone" dataKey={dataKey} stroke={color} strokeWidth={1.5}
            dot={false} isAnimationActive={false} />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
