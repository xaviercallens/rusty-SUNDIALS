import GlowPanel from '../components/GlowPanel';
import { Lightbulb, ExternalLink } from 'lucide-react';

const DISCOVERIES = [
  { id: 1, name: 'AsynchronousGeometricIntegrator', status: 'verified', cert: 'CERT-LEAN4-A72F3B', drift: '2.57e-16', speedup: '2.3×10¹⁴', basis: 'DEC-based async update' },
  { id: 2, name: 'SpectralLagrangianDecomposition', status: 'verified', cert: 'CERT-LEAN4-B93E1A', drift: '0.00', speedup: '∞', basis: 'Variational SDC-IMEX' },
  { id: 3, name: 'HamiltonianPrimalDualIntegrator', status: 'verified', cert: 'CERT-LEAN4-D58A2C', drift: '3.86e-16', speedup: '1.0×10¹⁵', basis: 'Saddle-point integration' },
  { id: 4, name: 'SymplecticEnergyProjection', status: 'verified', cert: 'CERT-LEAN4-E3B19D', drift: '1.29e-16', speedup: '4.7×10¹⁴', basis: 'Hamiltonian scaling' },
  { id: 5, name: 'GeometricNeuralPreconditioner', status: 'pending', cert: '—', drift: '—', speedup: '—', basis: 'Neural Jacobian approximation' },
];

export default function DiscoveriesPage() {
  return (
    <div>
      <div className="page-header"><h2>DISCOVERY LOG</h2></div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--gap-md)' }}>
        {DISCOVERIES.map(d => (
          <GlowPanel key={d.id}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                  <Lightbulb size={14} style={{ color: d.status === 'verified' ? 'var(--green)' : 'var(--amber)' }} />
                  <strong style={{ color: 'var(--text-primary)' }}>{d.name}</strong>
                  <span className={`badge ${d.status}`}>{d.status}</span>
                </div>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', marginBottom: 8 }}>{d.basis}</p>
                <div style={{ display: 'flex', gap: 'var(--gap-xl)' }}>
                  <span><span className="label">Drift </span><span className="data-value">{d.drift}</span></span>
                  <span><span className="label">Speedup </span><span className="data-value">{d.speedup}</span></span>
                  <span><span className="label">Cert </span><span className="data-value">{d.cert}</span></span>
                </div>
              </div>
              <button className="btn btn-outline"><ExternalLink size={12} /> VIEW</button>
            </div>
          </GlowPanel>
        ))}
      </div>
    </div>
  );
}
