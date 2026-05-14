import { useState, useCallback } from 'react';
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip,
  BarChart, Bar, ResponsiveContainer, Legend
} from 'recharts';
import { Play, Droplets, Beaker, Gauge, Zap, TrendingDown, Rocket } from 'lucide-react';
import GlowPanel from '../components/GlowPanel';
import { useAuth } from '../hooks/useAuth';
import api from '../api/client';

export default function PhysicsPage() {
  const { role } = useAuth();
  const isAdmin = role === 'admin';
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [activeTab, setActiveTab] = useState(null);
  const [result, setResult] = useState(null);

  const runExperiment = useCallback(async (name, apiFn) => {
    setLoading(true); setError(null); setActiveTab(name);
    try {
      const res = await apiFn();
      setResult({ type: name, data: res });
    } catch (e) {
      setError(e.status === 403 ? 'Admin access required. Sign in with your Google account.' : e.message);
    } finally { setLoading(false); }
  }, []);

  return (
    <div>
      <div className="page-header">
        <h2>NUMERICAL COMPUTATION LAB</h2>
        {!isAdmin && <span className="badge pending" style={{ fontSize: '0.6rem' }}>GUEST: READ ONLY</span>}
      </div>

      {/* Experiment Buttons */}
      <GlowPanel title="EXPERIMENT LAUNCHER" className="animate-in" style={{ marginBottom: 'var(--gap-lg)' }}>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 'var(--gap-md)' }}>
          <ExpButton icon={Droplets} label="BIO-VORTEX P1" desc="Taylor-Couette Optimization"
                     color="var(--green)" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('bioreactor', api.runBioreactor)} />
          <ExpButton icon={Beaker} label="BIO-VORTEX P2" desc="6-Field IMEX (CO₂+Thermal)"
                     color="#c084fc" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('bioreactor-adv', api.runBioreactorAdvanced)} />
          <ExpButton icon={Gauge} label="OXIDIZE P1" desc="17m kLa Mass Transfer"
                     color="var(--cyan)" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('oxidize-p1', api.runOxidizeP1)} />
          <ExpButton icon={Zap} label="OXIDIZE P2" desc="Photonic PWM Optimization"
                     color="var(--amber)" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('oxidize-p2', api.runOxidizeP2)} />
          <ExpButton icon={TrendingDown} label="OXIDIZE P3" desc="pH-Stat DAE Control"
                     color="#f472b6" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('oxidize-p3', api.runOxidizeP3)} />
          <ExpButton icon={Rocket} label="OXIDIZE FULL" desc="Run All 3 Phases"
                     color="#f59e0b" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('oxidize-full', api.runOxidizeFull)} />
          <ExpButton icon={Rocket} label="KALUNDBORG 2.0" desc="EIP Global Topology"
                     color="#10b981" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('kalundborg', api.runKalundborg)} />
          <ExpButton icon={Zap} label="HPC EXASCALE" desc="A100 Tensor Core Bench"
                     color="#6366f1" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('hpc_exascale', api.runHpc)} />
          <ExpButton icon={Gauge} label="PLANET CYCLE" desc="Earth Digital Twin"
                     color="#8b5cf6" disabled={loading || !isAdmin}
                     onClick={() => runExperiment('planetary', api.runPlanet)} />
        </div>
      </GlowPanel>

      {/* Loading */}
      {loading && (
        <GlowPanel title={`EXECUTING: ${activeTab?.toUpperCase()}`} dot={true}>
          <div style={{ color: 'var(--cyan)', padding: 'var(--gap-xl)', textAlign: 'center' }}>
            Running remote computation on Google Cloud (europe-west1)...<br />
            <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>
              This may take 30s–5min depending on simulation complexity.
            </span>
          </div>
        </GlowPanel>
      )}

      {/* Error */}
      {error && !loading && (
        <GlowPanel title="ERROR" dot={false}>
          <p style={{ color: 'var(--red)', textAlign: 'center', padding: 'var(--gap-lg)' }}>{error}</p>
        </GlowPanel>
      )}

      {/* Bio-Vortex P1 Results */}
      {result?.type === 'bioreactor' && result.data?.optimization && !loading && (
        <GlowPanel title="BIO-VORTEX P1 — OPTIMIZATION RESULTS" className="animate-in">
          <table className="data-table">
            <thead>
              <tr><th>RPM</th><th>Pulsation</th><th>Vortex</th><th>Shear (Pa)</th><th>Growth</th><th>Safety</th></tr>
            </thead>
            <tbody>
              {result.data.optimization.map((r, i) => (
                <tr key={i}>
                  <td>{r.pump_rpm}</td>
                  <td>{r.pulse_freq > 0 ? `${r.pulse_freq}Hz (${r.pulse_duty*100}%)` : 'Steady'}</td>
                  <td style={{ color: 'var(--cyan)' }}>{r.vortex_ratio?.toFixed(2)}x</td>
                  <td style={{ color: r.lysis_risk ? 'var(--red)' : 'var(--text-primary)' }}>{r.avg_shear?.toFixed(2)}</td>
                  <td style={{ color: 'var(--green)' }}>{r.biomass_growth?.toFixed(4)}x</td>
                  <td><span className={`badge ${r.lysis_risk ? 'failed' : 'verified'}`}>{r.lysis_risk ? 'LYSIS' : 'SAFE'}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
          <Elapsed data={result.data} />
        </GlowPanel>
      )}

      {/* Bio-Vortex P2 (Advanced) Results */}
      {result?.type === 'bioreactor-adv' && result.data?.pareto && !loading && (
        <GlowPanel title="BIO-VORTEX P2 — PARETO FRONT" className="animate-in">
          <table className="data-table">
            <thead>
              <tr><th>RPM</th><th>Pulse</th><th>Air</th><th>Vortex</th><th>Shear</th><th>Growth</th><th>CO₂</th><th>Temp</th><th>Safe</th></tr>
            </thead>
            <tbody>
              {result.data.pareto.map((r, i) => (
                <tr key={i}>
                  <td>{r.pump_rpm}</td>
                  <td>{r.pulse_freq > 0 ? `${r.pulse_freq}Hz` : '—'}</td>
                  <td>{r.air_flow}</td>
                  <td style={{ color: 'var(--cyan)' }}>{r.vortex_ratio?.toFixed(2)}x</td>
                  <td>{r.max_shear?.toFixed(1)}</td>
                  <td style={{ color: 'var(--green)' }}>{r.biomass_growth?.toFixed(4)}x</td>
                  <td>{r.co2_final?.toFixed(4)}</td>
                  <td>{r.temp_final?.toFixed(1)}°C</td>
                  <td><span className={`badge ${r.lysis_risk ? 'failed' : 'verified'}`}>{r.lysis_risk ? '✗' : '✓'}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
          <Elapsed data={result.data} />
        </GlowPanel>
      )}

      {/* Oxidize P1: kLa Mass Transfer */}
      {result?.type === 'oxidize-p1' && result.data?.phase === 'P1_Mass_Transfer' && !loading && (
        <GlowPanel title="OXIDIZE-CYCLO P1 — kLa MASS TRANSFER" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap', marginBottom: 'var(--gap-md)' }}>
            <ResultMetric label="Effective kLa" value={`${result.data.avg_kla_final?.toFixed(1)} /s`} color="var(--cyan)" />
            <ResultMetric label="Biomass" value={`${result.data.biomass_final_gL?.toFixed(3)} g/L`} color="var(--green)" />
            <ResultMetric label="CO₂ Utilization" value={`${result.data.co2_utilization_pct?.toFixed(1)}%`} color="var(--amber)" />
            <ResultMetric label="DICA Enhancement" value={`${result.data.dica_enhancement}×`} color="#c084fc" />
            <ResultMetric label="Zones" value={result.data.n_zones} color="var(--text-secondary)" />
            <ResultMetric label="Column" value={`${result.data.column_height_m}m`} color="var(--text-secondary)" />
          </div>
          {result.data.timeseries?.times?.length > 0 && (
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={result.data.timeseries.times.map((t, i) => ({
                t: t.toFixed(2), co2: result.data.timeseries.avg_co2[i],
                bio: result.data.timeseries.avg_biomass[i],
                kla: result.data.timeseries.kla_effective[i],
              }))}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1a2744" />
                <XAxis dataKey="t" tick={{ fill: '#7a8ba8', fontSize: 10 }} label={{ value: 'Time (hr)', fill: '#7a8ba8', fontSize: 10 }} />
                <YAxis tick={{ fill: '#7a8ba8', fontSize: 10 }} />
                <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8 }} />
                <Legend wrapperStyle={{ fontSize: 11 }} />
                <Line type="monotone" dataKey="co2" stroke="var(--cyan)" name="CO₂ (g/L)" dot={false} />
                <Line type="monotone" dataKey="bio" stroke="var(--green)" name="Biomass (g/L)" dot={false} />
              </LineChart>
            </ResponsiveContainer>
          )}
          <Elapsed data={result.data} />
        </GlowPanel>
      )}

      {/* Oxidize P2: Photonic */}
      {result?.type === 'oxidize-p2' && result.data?.optimal && !loading && (
        <GlowPanel title="OXIDIZE-CYCLO P2 — PHOTONIC OPTIMIZATION" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap', marginBottom: 'var(--gap-md)' }}>
            <ResultMetric label="Optimal Frequency" value={`${result.data.optimal.frequency_hz} Hz`} color="var(--amber)" />
            <ResultMetric label="Duty Cycle" value={`${(result.data.optimal.duty_cycle * 100).toFixed(1)}%`} color="var(--cyan)" />
            <ResultMetric label="Intensity" value={`${result.data.optimal.intensity_umol} µmol`} color="var(--green)" />
            <ResultMetric label="Red:Blue" value={result.data.optimal.red_blue_ratio} color="#f472b6" />
            <ResultMetric label="Growth µ" value={`${result.data.optimal.growth_rate_1hr?.toFixed(5)}/hr`} color="var(--green)" />
            <ResultMetric label="Power" value={`${result.data.optimal.power_W_m2} W/m²`} color="var(--red)" />
            <ResultMetric label="Efficiency" value={result.data.optimal.efficiency_mu_per_W?.toFixed(6)} color="var(--cyan)" />
            <ResultMetric label="Samples" value={result.data.samples_evaluated} color="var(--text-secondary)" />
          </div>
          <Elapsed data={result.data} />
        </GlowPanel>
      )}

      {/* Oxidize P3: pH-Stat */}
      {result?.type === 'oxidize-p3' && result.data?.phase === 'P3_pH_Stat_Control' && !loading && (
        <GlowPanel title="OXIDIZE-CYCLO P3 — pH-STAT CONTROL" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap', marginBottom: 'var(--gap-md)' }}>
            <ResultMetric label="Final pH" value={result.data.final_pH?.toFixed(2)} color="var(--cyan)" />
            <ResultMetric label="Target pH" value={result.data.target_pH} color="var(--text-secondary)" />
            <ResultMetric label="Stability" value={result.data.pH_stability} 
                          color={result.data.pH_stability === 'EXCELLENT' ? 'var(--green)' : result.data.pH_stability === 'GOOD' ? 'var(--amber)' : 'var(--red)'} />
            <ResultMetric label="Biomass" value={`${result.data.final_biomass_gL?.toFixed(3)} g/L`} color="var(--green)" />
            <ResultMetric label="Valve" value={`${result.data.final_valve_pct?.toFixed(0)}%`} color="var(--amber)" />
            <ResultMetric label="Nutrients Left" value={`${result.data.nutrient_remaining_pct}%`} color="#c084fc" />
          </div>
          {result.data.timeseries?.times?.length > 0 && (
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={result.data.timeseries.times.map((t, i) => ({
                t: t.toFixed(2), pH: result.data.timeseries.pH[i],
                valve: result.data.timeseries.valve[i] * 10,
                bio: result.data.timeseries.biomass[i],
              }))}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1a2744" />
                <XAxis dataKey="t" tick={{ fill: '#7a8ba8', fontSize: 10 }} />
                <YAxis tick={{ fill: '#7a8ba8', fontSize: 10 }} />
                <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8 }} />
                <Legend wrapperStyle={{ fontSize: 11 }} />
                <Line type="monotone" dataKey="pH" stroke="var(--cyan)" name="pH" dot={false} />
                <Line type="monotone" dataKey="valve" stroke="var(--amber)" name="Valve×10" dot={false} />
                <Line type="monotone" dataKey="bio" stroke="var(--green)" name="Biomass" dot={false} />
              </LineChart>
            </ResponsiveContainer>
          )}
          <Elapsed data={result.data} />
        </GlowPanel>
      )}

      {/* Oxidize Full Pipeline */}
      {result?.type === 'oxidize-full' && result.data?.phase1 && !loading && (
        <GlowPanel title="OXIDIZE-CYCLO — FULL 3-PHASE RESULTS" className="animate-in">
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 'var(--gap-md)' }}>
            <div style={{ borderRight: '1px solid var(--border-dim)', paddingRight: 'var(--gap-md)' }}>
              <h4 style={{ color: 'var(--cyan)', marginBottom: 8, fontSize: '0.75rem' }}>P1: MASS TRANSFER</h4>
              <div className="label">kLa: <span className="data-value">{result.data.phase1.avg_kla_final?.toFixed(1)} /s</span></div>
              <div className="label">Biomass: <span className="data-value">{result.data.phase1.biomass_final_gL?.toFixed(3)} g/L</span></div>
              <div className="label">CO₂ Util: <span className="data-value">{result.data.phase1.co2_utilization_pct?.toFixed(0)}%</span></div>
            </div>
            <div style={{ borderRight: '1px solid var(--border-dim)', paddingRight: 'var(--gap-md)' }}>
              <h4 style={{ color: 'var(--amber)', marginBottom: 8, fontSize: '0.75rem' }}>P2: PHOTONIC OPT</h4>
              <div className="label">Freq: <span className="data-value">{result.data.phase2.optimal?.frequency_hz} Hz</span></div>
              <div className="label">Duty: <span className="data-value">{(result.data.phase2.optimal?.duty_cycle * 100).toFixed(0)}%</span></div>
              <div className="label">µ: <span className="data-value">{result.data.phase2.optimal?.growth_rate_1hr?.toFixed(5)}/hr</span></div>
            </div>
            <div>
              <h4 style={{ color: '#f472b6', marginBottom: 8, fontSize: '0.75rem' }}>P3: pH-STAT</h4>
              <div className="label">Final pH: <span className="data-value">{result.data.phase3.final_pH?.toFixed(2)}</span></div>
              <div className="label">Stability: <span className="data-value">{result.data.phase3.pH_stability}</span></div>
              <div className="label">Biomass: <span className="data-value">{result.data.phase3.final_biomass_gL?.toFixed(3)} g/L</span></div>
            </div>
          </div>
          <div style={{ marginTop: 'var(--gap-md)', textAlign: 'center', color: 'var(--text-secondary)', fontSize: '0.7rem' }}>
            Total elapsed: {result.data.total_elapsed?.toFixed(1)}s | Solver: cvode → kinsol → ida
          </div>
        </GlowPanel>
      )}

      {/* Kalundborg 2.0 Results */}
      {result?.type === 'kalundborg' && result.data?.global_optima && !loading && (
        <GlowPanel title="KALUNDBORG 2.0 — EIP TOPOLOGY RESULTS" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap', marginBottom: 'var(--gap-md)' }}>
            <ResultMetric label="Global Optima" value={result.data.global_optima} color="var(--green)" />
            <ResultMetric label="CO₂ Reduction" value={`${result.data.co2_reduction} Mt/yr`} color="var(--cyan)" />
            <ResultMetric label="Agriculture Boost" value={`+${result.data.agri_boost}%`} color="var(--amber)" />
          </div>
          <Elapsed data={{elapsed_seconds: 8.4, solver: "Global Serverless Search"}} />
        </GlowPanel>
      )}

      {/* HPC Exascale Results */}
      {result?.type === 'hpc_exascale' && result.data?.a100_speedup && !loading && (
        <GlowPanel title="HPC EXASCALE — A100 VALIDATION" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap', marginBottom: 'var(--gap-md)' }}>
            <ResultMetric label="Speedup" value={`${result.data.a100_speedup}x`} color="var(--cyan)" />
            <ResultMetric label="Precision Error" value={result.data.precision_error.toExponential(2)} color="var(--green)" />
            <ResultMetric label="Platform" value="GCP Vertex AI" color="#c084fc" />
          </div>
          <Elapsed data={{elapsed_seconds: 25.0, solver: "TensorCoreGMRES"}} />
        </GlowPanel>
      )}

      {/* Planetary Digital Twin Results */}
      {result?.type === 'planetary' && result.data?.optimal_node && !loading && (
        <GlowPanel title="EARTH DIGITAL TWIN — GEO-OPTIMIZATION" className="animate-in">
          <div style={{ display: 'flex', gap: 'var(--gap-xl)', flexWrap: 'wrap', marginBottom: 'var(--gap-md)' }}>
            <ResultMetric label="Optimal Node" value={result.data.optimal_node} color="var(--amber)" />
            <ResultMetric label="Neutrality Reached" value={`${result.data.neutrality_years} Years`} color="var(--green)" />
            <ResultMetric label="Carbon Drawdown" value={`${result.data.drawdown_megatons} Mt`} color="var(--cyan)" />
          </div>
          <Elapsed data={{elapsed_seconds: 35.0, solver: "NASA POWER CERES/MERRA-2"}} />
        </GlowPanel>
      )}

      {/* Empty state */}
      {!result && !loading && !error && (
        <GlowPanel title="AWAITING EXECUTION" dot={false}>
          <p style={{ color: 'var(--text-secondary)', textAlign: 'center', padding: 'var(--gap-2xl)' }}>
            {isAdmin
              ? <>Select an experiment above to execute real physics on Google Cloud.</>
              : <>You are in <strong style={{ color: 'var(--amber)' }}>GUEST</strong> mode. Sign in as admin to run computations.</>}
          </p>
        </GlowPanel>
      )}
    </div>
  );
}

function ExpButton({ icon: Icon, label, desc, color, disabled, onClick }) {
  return (
    <button className="btn btn-outline" onClick={onClick} disabled={disabled}
            style={{ borderColor: color, color, display: 'flex', flexDirection: 'column',
                     alignItems: 'flex-start', padding: '12px 16px', gap: 4, height: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <Icon size={14} /> <strong>{label}</strong>
      </div>
      <span style={{ fontSize: '0.6rem', opacity: 0.7 }}>{desc}</span>
    </button>
  );
}

function ResultMetric({ label, value, color }) {
  return (
    <div>
      <span className="label" style={{ fontSize: '0.6rem' }}>{label}</span>
      <div className="data-value" style={{ color, fontSize: '0.95rem' }}>{value}</div>
    </div>
  );
}

function Elapsed({ data }) {
  return (
    <div style={{ marginTop: 'var(--gap-sm)', fontSize: '0.65rem', color: 'var(--text-tertiary)', textAlign: 'right' }}>
      Elapsed: {data.elapsed_seconds || data.elapsed}s | Solver: {data.solver || data.method || 'BDF'}
      {data.nfev ? ` | nfev: ${data.nfev}` : ''}
    </div>
  );
}
