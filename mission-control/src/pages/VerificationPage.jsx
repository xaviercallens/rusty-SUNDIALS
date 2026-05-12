import GlowPanel from '../components/GlowPanel';
import { ShieldCheck, CheckCircle, XCircle, Clock } from 'lucide-react';

const PROOFS = [
  { id: 1, theorem: 'energy_preservation', status: 'proved', tactics: 'intro x; exact ⟨e_bound⟩', cert: 'CERT-A72F', time: '0.8s' },
  { id: 2, theorem: 'divB_zero_invariant', status: 'proved', tactics: 'apply spectral_div_free', cert: 'CERT-B93E', time: '1.2s' },
  { id: 3, theorem: 'symplectic_jacobian_det_one', status: 'proved', tactics: 'simp [det_scaling]', cert: 'CERT-D58A', time: '2.1s' },
  { id: 4, theorem: 'helicity_conservation', status: 'failed', tactics: 'sorry -- needs inner product', cert: '—', time: '—' },
  { id: 5, theorem: 'hamiltonian_flow_map', status: 'pending', tactics: '—', cert: '—', time: '—' },
];

export default function VerificationPage() {
  return (
    <div>
      <div className="page-header"><h2>FORMAL VERIFICATION CONSOLE</h2></div>
      <div className="grid-4" style={{ marginBottom: 'var(--gap-lg)' }}>
        <div className="metric-card"><span className="metric-label">Total Proofs</span><span className="metric-value">5</span></div>
        <div className="metric-card"><span className="metric-label">Proved</span><span className="metric-value" style={{color:'var(--green)'}}>3</span></div>
        <div className="metric-card"><span className="metric-label">Failed</span><span className="metric-value" style={{color:'var(--red)'}}>1</span></div>
        <div className="metric-card"><span className="metric-label">Pending</span><span className="metric-value" style={{color:'var(--amber)'}}>1</span></div>
      </div>
      <GlowPanel title="PROOF OBLIGATIONS">
        <table className="data-table">
          <thead><tr><th>Theorem</th><th>Status</th><th>Tactics</th><th>Certificate</th><th>Time</th></tr></thead>
          <tbody>
            {PROOFS.map(p => (
              <tr key={p.id}>
                <td style={{ color: 'var(--text-primary)' }}>{p.theorem}</td>
                <td>
                  <span className={`badge ${p.status === 'proved' ? 'verified' : p.status === 'failed' ? 'failed' : 'pending'}`}>
                    {p.status === 'proved' && <CheckCircle size={10} />}
                    {p.status === 'failed' && <XCircle size={10} />}
                    {p.status === 'pending' && <Clock size={10} />}
                    {' '}{p.status}
                  </span>
                </td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>{p.tactics}</td>
                <td style={{ color: 'var(--cyan)' }}>{p.cert}</td>
                <td>{p.time}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </GlowPanel>
    </div>
  );
}
