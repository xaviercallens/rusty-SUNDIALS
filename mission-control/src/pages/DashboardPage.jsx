import { useState, useEffect, useCallback } from 'react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  BarChart, Bar, ResponsiveContainer, Legend
} from 'recharts';
import { Play, Zap, Clock, Award, TrendingDown, Droplets, FlaskConical, Gauge } from 'lucide-react';
import GlowPanel from '../components/GlowPanel';
import PipelineGraph from '../components/PipelineGraph';
import { useAuth } from '../hooks/useAuth';
import api from '../api/client';

export default function DashboardPage() {
  const { role } = useAuth();
  const isAdmin = role === 'admin';
  const [activeNode, setActiveNode] = useState('synthesize');
  const [loading, setLoading] = useState(false);
  const [lastResults, setLastResults] = useState(null);

  // Load last results on mount
  useEffect(() => {
    api.results()
      .then(data => { if (data.status !== 'no_results_yet') setLastResults(data); })
      .catch(() => {});
  }, []);

  const handleRunBioreactor = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.runBioreactor();
      setLastResults(res);
      setActiveNode('publish');
    } catch (e) { console.error(e); }
    finally { setLoading(false); }
  }, []);

  // Compute metrics from last results
  const metrics = {
    experiments: lastResults ? 1 : 0,
    status: lastResults?.status === 'complete' ? 'complete' : 'pending',
    physics: lastResults?.physics || '—',
    elapsed: lastResults?.elapsed_seconds || '—',
  };

  return (
    <div>
      <div className="page-header">
        <h2>MISSION DASHBOARD</h2>
        <button className="btn btn-primary" onClick={handleRunBioreactor}
                disabled={loading || !isAdmin}>
          <Play size={14} />
          {loading ? 'EXECUTING...' : 'RUN BIO-VORTEX P1'}
        </button>
      </div>

      {/* Metrics Row */}
      <div className="grid-4" style={{ marginBottom: 'var(--gap-lg)' }}>
        <MetricCard icon={Zap} label="Status" value={metrics.status === 'complete' ? '✓ COMPLETE' : 'IDLE'}
                    delta={metrics.physics} positive={metrics.status === 'complete'} />
        <MetricCard icon={Droplets} label="Experiments" value={metrics.experiments}
                    delta="bioreactor" positive />
        <MetricCard icon={FlaskConical} label="Engine"
                    value="SUNDIALS BDF" delta="IMEX + Projection" positive />
        <MetricCard icon={Clock} label="Last Run"
                    value={metrics.elapsed !== '—' ? `${metrics.elapsed}s` : '—'}
                    delta="Cloud Run EU" />
      </div>

      {/* Main Grid */}
      <div className="grid-2x2">
        <GlowPanel title="PIPELINE STATUS">
          <PipelineGraph activeNode={activeNode} />
        </GlowPanel>

        <GlowPanel title="EXPERIMENT CAPABILITIES">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12, padding: '8px 0' }}>
            <ExperimentRow icon={Droplets} name="Bio-Vortex P1" desc="Taylor-Couette algae vortex optimization"
                          solver="cvode-rs (BDF)" status="active" color="var(--green)" />
            <ExperimentRow icon={FlaskConical} name="Bio-Vortex P2" desc="6-field: CO₂ + thermal + diurnal"
                          solver="BDF + IMEX" status="active" color="#c084fc" />
            <ExperimentRow icon={Gauge} name="Oxidize-Cyclo P1" desc="17m column kLa mass transfer (DICA nanobubbles)"
                          solver="cvode-rs (BDF)" status="ready" color="var(--cyan)" />
            <ExperimentRow icon={Zap} name="Oxidize-Cyclo P2" desc="PWM photonic optimization (Monod-Haldane)"
                          solver="kinsol-rs (Newton)" status="ready" color="var(--amber)" />
            <ExperimentRow icon={TrendingDown} name="Oxidize-Cyclo P3" desc="pH-Stat DAE cyber-physical control"
                          solver="ida-rs (Radau)" status="ready" color="#f472b6" />
          </div>
        </GlowPanel>

        {/* Last results */}
        <GlowPanel title="LAST EXPERIMENT RESULTS">
          {lastResults?.optimization ? (
            <div style={{ overflowX: 'auto' }}>
              <table className="data-table">
                <thead>
                  <tr><th>Config</th><th>Vortex</th><th>Shear</th><th>Growth</th><th>Safe</th></tr>
                </thead>
                <tbody>
                  {lastResults.optimization.slice(0, 4).map((r, i) => (
                    <tr key={i}>
                      <td>{r.pump_rpm} RPM</td>
                      <td style={{ color: 'var(--cyan)' }}>{r.vortex_ratio?.toFixed(2)}x</td>
                      <td>{r.avg_shear?.toFixed(1)} Pa</td>
                      <td style={{ color: 'var(--green)' }}>{r.biomass_growth?.toFixed(4)}x</td>
                      <td><span className={`badge ${r.lysis_risk ? 'failed' : 'verified'}`}>
                        {r.lysis_risk ? '✗' : '✓'}
                      </span></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : lastResults?.phase1 ? (
            <div style={{ padding: 12 }}>
              <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap' }}>
                <MiniMetric label="kLa" value={`${lastResults.phase1.avg_kla_final?.toFixed(1)} 1/s`} color="var(--cyan)" />
                <MiniMetric label="Biomass" value={`${lastResults.phase1.biomass_final_gL?.toFixed(2)} g/L`} color="var(--green)" />
                <MiniMetric label="CO₂ Util" value={`${lastResults.phase1.co2_utilization_pct?.toFixed(0)}%`} color="var(--amber)" />
              </div>
            </div>
          ) : (
            <p style={{ color: 'var(--text-secondary)', textAlign: 'center', padding: 'var(--gap-xl)' }}>
              No experiments run yet. {isAdmin ? 'Click a button above to start.' : 'Sign in as admin to run.'}
            </p>
          )}
        </GlowPanel>

        <GlowPanel title="OXIDIZE-CYCLO ARCHITECTURE">
          <div style={{ padding: '8px 0', fontSize: '0.75rem', color: 'var(--text-secondary)', lineHeight: 1.7 }}>
            <div style={{ marginBottom: 8, color: 'var(--cyan)', fontWeight: 'bold' }}>CYCLOREACTOR V2.0 SPECS</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '4px 16px' }}>
              <span>Column Height</span><span className="data-value">17.0 m</span>
              <span>Bubble Size</span><span className="data-value">&lt; 5 µm</span>
              <span>DICA Enhancement</span><span className="data-value">50×</span>
              <span>Flue CO₂</span><span className="data-value">12–15%</span>
              <span>Light Guide</span><span className="data-value">680nm + 450nm PWM</span>
              <span>pH Target</span><span className="data-value">7.5 ± 0.05</span>
              <span>Solver Stack</span><span className="data-value">cvode / kinsol / ida</span>
              <span>Edge Deploy</span><span className="data-value">RPi / STM32</span>
            </div>
          </div>
        </GlowPanel>
      </div>

      {loading && (
        <div style={{
          position: 'fixed', bottom: 24, right: 24, padding: '16px 24px',
          background: 'var(--bg-surface)', border: '1px solid var(--cyan-dim)',
          borderRadius: 'var(--radius-md)', fontFamily: 'JetBrains Mono',
          fontSize: '0.85rem', color: 'var(--cyan)', zIndex: 1000,
          animation: 'fade-in-up 0.4s ease-out',
        }}>
          ⏳ Running computation on Google Cloud (europe-west1)...
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

function ExperimentRow({ icon: Icon, name, desc, solver, status, color }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 12,
                  padding: '8px 0', borderBottom: '1px solid var(--border-dim)' }}>
      <Icon size={16} style={{ color, flexShrink: 0 }} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <strong style={{ color: 'var(--text-primary)', fontSize: '0.8rem' }}>{name}</strong>
          <span className={`badge ${status === 'active' ? 'verified' : 'pending'}`}
                style={{ fontSize: '0.5rem' }}>{status.toUpperCase()}</span>
        </div>
        <div style={{ fontSize: '0.65rem', color: 'var(--text-secondary)' }}>{desc}</div>
      </div>
      <span style={{ fontSize: '0.6rem', color: 'var(--text-tertiary)', whiteSpace: 'nowrap' }}>{solver}</span>
    </div>
  );
}

function MiniMetric({ label, value, color }) {
  return (
    <div>
      <div className="label" style={{ fontSize: '0.6rem' }}>{label}</div>
      <div className="data-value" style={{ color, fontSize: '0.95rem' }}>{value}</div>
    </div>
  );
}
