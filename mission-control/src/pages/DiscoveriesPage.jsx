import { useState, useEffect } from 'react';
import GlowPanel from '../components/GlowPanel';
import { Lightbulb, ExternalLink, RefreshCw } from 'lucide-react';
import api from '../api/client';

export default function DiscoveriesPage() {
  const [results, setResults] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.results()
      .then(data => { setResults(data); setLoading(false); })
      .catch(() => setLoading(false));
  }, []);

  const refresh = () => {
    setLoading(true);
    api.results()
      .then(data => { setResults(data); setLoading(false); })
      .catch(() => setLoading(false));
  };

  // Extract discoveries from results
  const discoveries = [];
  if (results?.optimization) {
    results.optimization.forEach((r, i) => {
      discoveries.push({
        id: `bio-${i}`, name: `BioVortex ${r.pump_rpm}RPM ${r.pulse_freq > 0 ? `${r.pulse_freq}Hz` : 'Steady'}`,
        status: r.lysis_risk ? 'failed' : 'verified', physics: '3D_Bioreactor_Vortex',
        metric1: { label: 'Vortex', value: `${r.vortex_ratio?.toFixed(2)}x` },
        metric2: { label: 'Growth', value: `${r.biomass_growth?.toFixed(4)}x` },
        metric3: { label: 'Shear', value: `${r.avg_shear?.toFixed(1)} Pa` },
      });
    });
  }
  if (results?.phase1) {
    discoveries.push({
      id: 'ox-p1', name: 'Oxidize-Cyclo P1: kLa Mass Transfer',
      status: results.phase1.success ? 'verified' : 'pending', physics: 'P1_Mass_Transfer',
      metric1: { label: 'kLa', value: `${results.phase1.avg_kla_final?.toFixed(1)} /s` },
      metric2: { label: 'Biomass', value: `${results.phase1.biomass_final_gL?.toFixed(3)} g/L` },
      metric3: { label: 'CO₂ Util', value: `${results.phase1.co2_utilization_pct?.toFixed(0)}%` },
    });
  }
  if (results?.phase2) {
    discoveries.push({
      id: 'ox-p2', name: 'Oxidize-Cyclo P2: Photonic Optimization',
      status: 'verified', physics: 'P2_Photonic_Optimization',
      metric1: { label: 'Freq', value: `${results.phase2.optimal?.frequency_hz} Hz` },
      metric2: { label: 'µ', value: `${results.phase2.optimal?.growth_rate_1hr?.toFixed(5)}/hr` },
      metric3: { label: 'Efficiency', value: results.phase2.optimal?.efficiency_mu_per_W?.toFixed(6) },
    });
  }
  if (results?.phase3) {
    discoveries.push({
      id: 'ox-p3', name: 'Oxidize-Cyclo P3: pH-Stat Control',
      status: results.phase3.pH_stability === 'EXCELLENT' ? 'verified' : results.phase3.pH_stability === 'GOOD' ? 'verified' : 'pending',
      physics: 'P3_pH_Stat_Control',
      metric1: { label: 'pH', value: results.phase3.final_pH?.toFixed(2) },
      metric2: { label: 'Stability', value: results.phase3.pH_stability },
      metric3: { label: 'Biomass', value: `${results.phase3.final_biomass_gL?.toFixed(3)} g/L` },
    });
  }

  return (
    <div>
      <div className="page-header">
        <h2>DISCOVERY LOG</h2>
        <button className="btn btn-outline" onClick={refresh} disabled={loading}>
          <RefreshCw size={14} /> REFRESH
        </button>
      </div>

      {loading && (
        <GlowPanel title="LOADING...">
          <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--cyan)' }}>
            Fetching latest experiment results...
          </p>
        </GlowPanel>
      )}

      {!loading && discoveries.length === 0 && (
        <GlowPanel title="NO DISCOVERIES YET">
          <p style={{ textAlign: 'center', padding: 'var(--gap-2xl)', color: 'var(--text-secondary)' }}>
            Run an experiment from the Physics Lab to generate discoveries.
          </p>
        </GlowPanel>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--gap-md)' }}>
        {discoveries.map(d => (
          <GlowPanel key={d.id}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                  <Lightbulb size={14} style={{ color: d.status === 'verified' ? 'var(--green)' : d.status === 'failed' ? 'var(--red)' : 'var(--amber)' }} />
                  <strong style={{ color: 'var(--text-primary)' }}>{d.name}</strong>
                  <span className={`badge ${d.status}`}>{d.status}</span>
                </div>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', marginBottom: 8 }}>{d.physics}</p>
                <div style={{ display: 'flex', gap: 'var(--gap-xl)' }}>
                  <span><span className="label">{d.metric1.label} </span><span className="data-value">{d.metric1.value}</span></span>
                  <span><span className="label">{d.metric2.label} </span><span className="data-value">{d.metric2.value}</span></span>
                  <span><span className="label">{d.metric3.label} </span><span className="data-value">{d.metric3.value}</span></span>
                </div>
              </div>
            </div>
          </GlowPanel>
        ))}
      </div>

      {results?.timestamp && (
        <div style={{ textAlign: 'right', marginTop: 'var(--gap-md)', fontSize: '0.65rem', color: 'var(--text-tertiary)' }}>
          Last updated: {new Date(results.timestamp).toLocaleString()}
        </div>
      )}
    </div>
  );
}
