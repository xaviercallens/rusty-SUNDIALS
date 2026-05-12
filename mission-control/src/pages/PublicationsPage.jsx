import GlowPanel from '../components/GlowPanel';
import { FileText, Download } from 'lucide-react';

const PAPERS = [
  { id: 1, title: 'Energy-Preserving Symplectic Projection for Stiff MHD Tearing Modes', status: 'Ready to Submit', method: 'SymplecticEnergyProjection', gcs: 'gs://rusty-sundials-discoveries/runs/20260512_200614/PAPER_SymplecticEnergyProjection.tex' },
  { id: 2, title: 'Hamiltonian Primal-Dual Integration for Resistive MHD', status: 'Formal Verification', method: 'HamiltonianPrimalDualIntegrator', gcs: 'gs://rusty-sundials-discoveries/runs/20260512_192423/PAPER_HamiltonianPrimalDualIntegrator.tex' },
  { id: 3, title: 'Geometric Neural Preconditioning for Stiff ODE Systems', status: 'Draft', method: 'GeometricNeuralPreconditioner', gcs: '' },
];

const STATUSES = { 'Draft': 'pending', 'Internal Review': 'active', 'Formal Verification': 'active', 'Ready to Submit': 'verified' };

export default function PublicationsPage() {
  return (
    <div>
      <div className="page-header"><h2>PUBLICATIONS MANAGER</h2></div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--gap-md)' }}>
        {PAPERS.map(p => (
          <GlowPanel key={p.id}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                  <FileText size={14} style={{ color: 'var(--cyan)' }} />
                  <strong>{p.title}</strong>
                  <span className={`badge ${STATUSES[p.status]}`}>{p.status}</span>
                </div>
                <div style={{ display: 'flex', gap: 'var(--gap-xl)' }}>
                  <span><span className="label">Method </span><span className="data-value">{p.method}</span></span>
                  {p.gcs && <span><span className="label">GCS </span><span className="data-value" style={{fontSize:'0.7rem'}}>{p.gcs.split('/').pop()}</span></span>}
                </div>
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <button className="btn btn-outline"><Download size={12} /> LaTeX</button>
                <button className="btn btn-primary">PDF</button>
              </div>
            </div>
          </GlowPanel>
        ))}
      </div>
    </div>
  );
}
