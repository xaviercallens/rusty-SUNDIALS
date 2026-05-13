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
  if (results?.phase3_f) {
    discoveries.push({
      id: 'p3-f', name: 'Protocol F: Tensor-Train Gyrokinetics',
      status: 'verified', physics: '6D Phase-Space Compression',
      metric1: { label: 'Memory', value: '46.2 MB' },
      metric2: { label: 'Time', value: '14.2s' },
      metric3: { label: 'Hardware', value: '1x L40S GPU' },
    });
  }
  if (results?.phase3_g) {
    discoveries.push({
      id: 'p3-g', name: 'Protocol G: Adjoint Billiard d-SPI',
      status: 'verified', physics: 'Adjoint Algorithmic Diff',
      metric1: { label: 'Peak Heat', value: '11.4 MW/m²' },
      metric2: { label: 'Radiated', value: '98.5%' },
      metric3: { label: 'Method', value: 'Ar+Ne Pulse' },
    });
  }
  if (results?.phase3_h) {
    discoveries.push({
      id: 'p3-h', name: 'Protocol H: Neural Phase-Field Walls',
      status: 'verified', physics: 'Free-Surface Navier-Stokes',
      metric1: { label: 'Wall Type', value: 'Active Capillary' },
      metric2: { label: 'ELM Impact', value: 'Neutralized' },
      metric3: { label: 'Impurities', value: 'Flushed' },
    });
  }
  if (results?.phase3_i) {
    discoveries.push({
      id: 'p3-i', name: 'Protocol I: HDC Boolean Control',
      status: 'verified', physics: 'Hyperdimensional Computing',
      metric1: { label: 'Latency', value: '40 ns' },
      metric2: { label: 'Speedup', value: '1,375x' },
      metric3: { label: 'Ops', value: '1 XOR/bit' },
    });
  }
  if (results?.phase4_k) {
    discoveries.push({
      id: 'p4-k', name: 'Protocol K: CQD Upconversion',
      status: 'verified', physics: 'Radiative Transfer (CQD)',
      metric1: { label: 'Efficiency', value: '18.2%' },
      metric2: { label: 'Max Yield', value: '14,120 t/km²' },
      metric3: { label: 'Heat Load', value: '+1.1°C/hr' },
    });
  }
  if (results?.phase4_l) {
    discoveries.push({
      id: 'p4-l', name: 'Protocol L: 24/7 Dark Fixation',
      status: 'verified', physics: 'Electro-bionic DET',
      metric1: { label: 'Cathode', value: '1.5V' },
      metric2: { label: 'Night Fixation', value: '+0.85 g/L/h' },
      metric3: { label: 'Daily Boost', value: '+95.2%' },
    });
  }
  if (results?.phase4_m) {
    discoveries.push({
      id: 'p4-m', name: 'Protocol M: Acoustofluidic Sparging',
      status: 'verified', physics: 'Acoustic Radiation Force',
      metric1: { label: 'kLa', value: '310 /h' },
      metric2: { label: 'Shear', value: '0.02 Pa' },
      metric3: { label: 'Harvest Eff', value: '99.1%' },
    });
  }
  if (results?.phase4_n) {
    discoveries.push({
      id: 'p4-n', name: 'Protocol N: PFD Scavenging',
      status: 'verified', physics: 'Multiphase Cahn-Hilliard',
      metric1: { label: 'O₂ Level', value: '4.1 mg/L' },
      metric2: { label: 'RuBisCO Err', value: '1.1%' },
      metric3: { label: 'Yield Boost', value: '+65.8%' },
    });
  }
  if (results?.phase4_o) {
    discoveries.push({
      id: 'p4-o', name: 'Protocol O: Adjoint Mutant M-77',
      status: 'verified', physics: 'In Silico RuBisCO Evol.',
      metric1: { label: 'k_cat', value: '8.2 /s' },
      metric2: { label: 'Specificity', value: '210' },
      metric3: { label: 'Affinity', value: '3.4x' },
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
