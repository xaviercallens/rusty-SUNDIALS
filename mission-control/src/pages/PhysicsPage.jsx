import { useState, useCallback } from 'react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  BarChart, Bar, ResponsiveContainer, Legend
} from 'recharts';
import { Play, RotateCcw, Droplets, Beaker } from 'lucide-react';
import GlowPanel from '../components/GlowPanel';
import { useAuth } from '../hooks/useAuth';
import api from '../api/client';

export default function PhysicsPage() {
  const { role } = useAuth();
  const isAdmin = role === 'admin';
  const [params, setParams] = useState({ grid: 128, eta: '1e-3', t_end: 0.1 });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [result, setResult] = useState(null);
  const [sweepResult, setSweepResult] = useState(null);
  const [bioResult, setBioResult] = useState(null);
  const [advResult, setAdvResult] = useState(null);

  const runPhysics = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.runPhysics({ t_end: parseFloat(params.t_end) });
      setResult(res); setSweepResult(null); setBioResult(null);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, [params]);

  const runSweep = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.runSweep();
      setSweepResult(res); setResult(null); setBioResult(null);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  const runBioreactor = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.runBioreactor();
      setBioResult(res); setResult(null); setSweepResult(null);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  const runAdvanced = useCallback(async () => {
    setLoading(true); setError(null);
    try {
      const res = await api.runBioreactorAdvanced();
      setAdvResult(res); setResult(null); setSweepResult(null); setBioResult(null);
    } catch (e) {
      setError(e.status === 403 ? 'Admin access required' : e.message);
    } finally {
      setLoading(false);
    }
  }, []);

  return (
    <div>
      <div className="page-header">
        <h2>NUMERICAL COMPUTATION LAB</h2>
        <div style={{ display: 'flex', gap: 'var(--gap-sm)', flexWrap: 'wrap' }}>
          <button className="btn btn-primary" onClick={runPhysics} disabled={loading || !isAdmin}
                  title={!isAdmin ? 'Admin only' : ''}>
            <Play size={14} /> 1D RMHD
          </button>
          <button className="btn btn-outline" onClick={runSweep} disabled={loading || !isAdmin}>
            <RotateCcw size={14} /> STIFFNESS SWEEP
          </button>
          <button className="btn btn-outline" onClick={runBioreactor} disabled={loading || !isAdmin}
                  style={{ borderColor: 'var(--green)', color: 'var(--green)' }}>
            <Droplets size={14} /> BIO-VORTEX P1
          </button>
          <button className="btn btn-outline" onClick={runAdvanced} disabled={loading || !isAdmin}
                  style={{ borderColor: '#c084fc', color: '#c084fc' }}>
            <Beaker size={14} /> ADVANCED BIO P2
          </button>
          {!isAdmin && <span className="badge pending" style={{ alignSelf: 'center', fontSize: '0.6rem' }}>GUEST: READ ONLY</span>}
        </div>
      </div>

      {loading && (
        <GlowPanel title="EXECUTING" dot={true}>
          <div style={{ color: 'var(--cyan)', padding: 'var(--gap-xl)', textAlign: 'center' }}>
            Running remote computation on Google Cloud (europe-west1)...
          </div>
        </GlowPanel>
      )}

      {/* Bioreactor Results */}
      {bioResult?.optimization && !loading && (
        <GlowPanel title="3D ALGAE BIOREACTOR OPTIMIZATION RESULTS" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-lg)' }}>
            <div style={{ flex: 1 }}>
              <table className="data-table">
                <thead>
                  <tr><th>Agitation Config</th><th>Pulsation</th><th>Vortex Ratio</th><th>Wall Shear (Pa)</th><th>Lysis Risk</th><th>Biomass Yield</th></tr>
                </thead>
                <tbody>
                  {bioResult.optimization.map((r, i) => (
                    <tr key={i}>
                      <td>{r.pump_rpm} RPM</td>
                      <td>{r.pulse_freq > 0 ? `${r.pulse_freq}Hz (${r.pulse_duty*100}%)` : 'Steady'}</td>
                      <td style={{ color: 'var(--cyan)' }}>{r.vortex_ratio?.toFixed(2)}x</td>
                      <td style={{ color: r.lysis_risk ? 'var(--red)' : 'var(--text-primary)' }}>{r.avg_shear?.toFixed(2)}</td>
                      <td>
                        <span className={`badge ${r.lysis_risk ? 'failed' : 'verified'}`}>
                          {r.lysis_risk ? 'CELL DEATH' : 'SAFE'}
                        </span>
                      </td>
                      <td style={{ color: 'var(--green)' }}>{r.biomass_growth?.toFixed(4)}x</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          <div style={{ marginTop: 'var(--gap-md)', display: 'flex', gap: 'var(--gap-md)' }}>
            {bioResult.gcs_artifacts?.map((uri, i) => (
              <span key={i} className="badge active" style={{ fontSize: '0.6rem' }}>☁ {uri.split('/').pop()}</span>
            ))}
          </div>
        </GlowPanel>
      )}

      {/* Parameter Controls for Plasma */}
      {!bioResult && (
      <GlowPanel title="PLASMA SIMULATION PARAMETERS" className="animate-in" style={{ marginBottom: 'var(--gap-lg)' }}>
        <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap' }}>
          <ParamInput label="Grid Points N" value={params.grid} onChange={v => setParams(p => ({ ...p, grid: v }))} />
          <ParamInput label="Resistivity η" value={params.eta} onChange={v => setParams(p => ({ ...p, eta: v }))} />
          <ParamInput label="Time Span t_end" value={params.t_end} onChange={v => setParams(p => ({ ...p, t_end: v }))} />
          <div>
            <div className="label" style={{ marginBottom: 6 }}>Solver Method</div>
            <div className="data-value">BDF (CVODE equiv.)</div>
          </div>
          <div>
            <div className="label" style={{ marginBottom: 6 }}>Lundquist Number</div>
            <div className="data-value">S = {(0.1 / parseFloat(params.eta || 1e-3)).toFixed(0)}</div>
          </div>
        </div>
      </GlowPanel>
      )}

      {/* Results (Existing Plasma code) */}
      {result && !loading && (
        <div className="grid-2x2" style={{ marginBottom: 'var(--gap-lg)' }}>
          <GlowPanel title="ENERGY CONSERVATION">
            <div className="grid-2x2" style={{ gap: 'var(--gap-md)' }}>
              <ResultMetric label="Baseline Drift" value={result.baseline?.energy_drift?.toExponential(2)} color="var(--red)" />
              <ResultMetric label="Projected Drift" value={result.projected?.energy_drift?.toExponential(2)} color="var(--green)" />
              <ResultMetric label="Improvement" value={`${result.improvement?.energy_drift_ratio?.toExponential(1)}×`} color="var(--cyan)" />
              <ResultMetric label="Elapsed" value={`${result.elapsed_seconds}s`} color="var(--amber)" />
            </div>
          </GlowPanel>
        </div>
      )}

      {sweepResult?.sweep && !loading && (
        <GlowPanel title="STIFFNESS SWEEP RESULTS">
          <div style={{ display: 'flex', gap: 'var(--gap-lg)' }}>
            <div style={{ flex: 1 }}>
              <ResponsiveContainer width="100%" height={280}>
                <BarChart data={sweepResult.sweep} barGap={4}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#1a2744" />
                  <XAxis dataKey="label" tick={{ fill: '#7a8ba8', fontSize: 10, fontFamily: 'JetBrains Mono' }} />
                  <YAxis tick={{ fill: '#7a8ba8', fontSize: 10, fontFamily: 'JetBrains Mono' }}
                    tickFormatter={v => v.toExponential(0)} />
                  <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8 }} />
                  <Legend wrapperStyle={{ fontFamily: 'JetBrains Mono', fontSize: 11 }} />
                  <Bar dataKey="baseline_drift" fill="#ff3d5a" name="Baseline" radius={[4, 4, 0, 0]} />
                  <Bar dataKey="projected_drift" fill="#00ff88" name="Projected" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
            <div style={{ flex: 1 }}>
              <table className="data-table">
                <thead>
                  <tr><th>S</th><th>Baseline</th><th>Projected</th><th>Improvement</th></tr>
                </thead>
                <tbody>
                  {sweepResult.sweep.map((r, i) => (
                    <tr key={i}>
                      <td>{r.label}</td>
                      <td style={{ color: 'var(--red)' }}>{r.baseline_drift?.toExponential(2)}</td>
                      <td style={{ color: 'var(--green)' }}>{r.projected_drift?.toExponential(2)}</td>
                      <td style={{ color: 'var(--cyan)' }}>{r.improvement?.toExponential(1)}×</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          <div style={{ marginTop: 'var(--gap-md)', display: 'flex', gap: 'var(--gap-md)' }}>
            {sweepResult.gcs_artifacts?.map((uri, i) => (
              <span key={i} className="badge active" style={{ fontSize: '0.6rem' }}>☁ {uri.split('/').pop()}</span>
            ))}
          </div>
        </GlowPanel>
      )}

      {/* Advanced Bioreactor Phase 2 Results */}
      {advResult?.pareto && !loading && (
        <GlowPanel title="ADVANCED BIOREACTOR PHASE 2 — PARETO FRONT" className="animate-in">
          <table className="data-table">
            <thead>
              <tr><th>RPM</th><th>Pulse</th><th>Air (L/min)</th><th>Vortex</th><th>Max Shear</th><th>Growth</th><th>CO₂</th><th>Temp</th><th>Safety</th></tr>
            </thead>
            <tbody>
              {advResult.pareto.map((r, i) => (
                <tr key={i}>
                  <td>{r.pump_rpm}</td>
                  <td>{r.pulse_freq > 0 ? `${r.pulse_freq}Hz (${(r.pulse_duty*100).toFixed(0)}%)` : 'Steady'}</td>
                  <td>{r.air_flow}</td>
                  <td style={{ color: 'var(--cyan)' }}>{r.vortex_ratio?.toFixed(2)}x</td>
                  <td style={{ color: r.lysis_risk ? 'var(--red)' : 'var(--text-primary)' }}>{r.max_shear?.toFixed(2)}</td>
                  <td style={{ color: 'var(--green)' }}>{r.biomass_growth?.toFixed(4)}x</td>
                  <td>{r.co2_final?.toFixed(4)}</td>
                  <td>{r.temp_final?.toFixed(1)}°C</td>
                  <td><span className={`badge ${r.lysis_risk ? 'failed' : 'verified'}`}>{r.lysis_risk ? 'LYSIS' : 'SAFE'}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </GlowPanel>
      )}

      {error && !loading && (
        <GlowPanel title="ERROR" dot={false}>
          <p style={{ color: 'var(--red)', textAlign: 'center', padding: 'var(--gap-lg)' }}>{error}</p>
        </GlowPanel>
      )}

      {!result && !sweepResult && !bioResult && !advResult && !loading && !error && (
        <GlowPanel title="AWAITING EXECUTION" dot={false}>
          <p style={{ color: 'var(--text-secondary)', textAlign: 'center', padding: 'var(--gap-2xl)' }}>
            {isAdmin
              ? <>Select a computation above to execute real physics on Google Cloud.</>
              : <>You are in <strong style={{ color: 'var(--amber)' }}>GUEST</strong> mode. Sign in as admin to run computations.</>}
          </p>
        </GlowPanel>
      )}
    </div>
  );
}

function ParamInput({ label, value, onChange }) {
  return (
    <div>
      <div className="label" style={{ marginBottom: 6 }}>{label}</div>
      <input
        value={value}
        onChange={e => onChange(e.target.value)}
        className="mono"
        style={{
          background: 'var(--bg-deep)', border: '1px solid var(--border-dim)',
          borderRadius: 'var(--radius-sm)', padding: '6px 12px',
          color: 'var(--cyan)', fontSize: '0.9rem', width: 120,
          fontFamily: 'JetBrains Mono, monospace',
        }}
      />
    </div>
  );
}

function ResultMetric({ label, value, color }) {
  return (
    <div className="metric-card">
      <span className="metric-label">{label}</span>
      <span className="metric-value" style={{ color, fontSize: '1.1rem' }}>{value}</span>
    </div>
  );
}
