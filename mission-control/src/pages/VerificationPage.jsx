import { useState, useEffect, useCallback } from 'react';
import GlowPanel from '../components/GlowPanel';
import { ShieldCheck, CheckCircle, XCircle, Clock, Play, RefreshCw } from 'lucide-react';
import { useAuth } from '../hooks/useAuth';
import api from '../api/client';

export default function VerificationPage() {
  const { role } = useAuth();
  const isAdmin = role === 'admin';
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    api.getVerification()
      .then(d => { if (!d.status) setData(d); setLoading(false); })
      .catch(() => setLoading(false));
  }, []);

  const runVerify = useCallback(async () => {
    setRunning(true);
    try {
      const res = await api.runVerification();
      if (!res.error) setData(res);
    } catch (e) { console.error(e); }
    finally { setRunning(false); }
  }, []);

  const proofs = data?.proofs || [];
  const proved = data?.proved || 0;
  const failed = data?.failed || 0;
  const pending = data?.pending || 0;
  const total = data?.total_proofs || 0;

  return (
    <div>
      <div className="page-header">
        <h2>FORMAL VERIFICATION CONSOLE</h2>
        <button className="btn btn-primary" onClick={runVerify}
                disabled={running || !isAdmin}>
          {running ? <><RefreshCw size={14} className="spin" /> VERIFYING...</> : <><Play size={14} /> RUN LEAN 4 VERIFICATION</>}
        </button>
      </div>

      {/* Metrics */}
      <div className="grid-4" style={{ marginBottom: 'var(--gap-lg)' }}>
        <div className="metric-card animate-in">
          <span className="metric-label">Total Proofs</span>
          <span className="metric-value">{total || '—'}</span>
        </div>
        <div className="metric-card animate-in">
          <span className="metric-label">Proved</span>
          <span className="metric-value" style={{ color: 'var(--green)' }}>{proved || '—'}</span>
        </div>
        <div className="metric-card animate-in">
          <span className="metric-label">Failed</span>
          <span className="metric-value" style={{ color: 'var(--red)' }}>{failed || '—'}</span>
        </div>
        <div className="metric-card animate-in">
          <span className="metric-label">Pass Rate</span>
          <span className="metric-value" style={{ color: data?.pass_rate >= 80 ? 'var(--green)' : 'var(--amber)' }}>
            {data?.pass_rate ? `${data.pass_rate}%` : '—'}
          </span>
        </div>
      </div>

      {/* Proof Table */}
      {proofs.length > 0 ? (
        <GlowPanel title="LEAN 4 PROOF OBLIGATIONS" className="animate-in">
          <table className="data-table">
            <thead>
              <tr><th>Theorem</th><th>Module</th><th>Status</th><th>Evidence</th><th>Certificate</th><th>Time</th></tr>
            </thead>
            <tbody>
              {proofs.map((p, i) => (
                <tr key={i}>
                  <td style={{ color: 'var(--text-primary)', fontWeight: 'bold' }}>{p.theorem}</td>
                  <td style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{p.module}</td>
                  <td>
                    <span className={`badge ${p.status === 'proved' ? 'verified' : p.status === 'failed' ? 'failed' : 'pending'}`}>
                      {p.status === 'proved' && <CheckCircle size={10} />}
                      {p.status === 'failed' && <XCircle size={10} />}
                      {p.status === 'pending' && <Clock size={10} />}
                      {' '}{p.status}
                    </span>
                  </td>
                  <td style={{ fontSize: '0.65rem', color: 'var(--text-secondary)', maxWidth: 250 }}>{p.evidence}</td>
                  <td style={{ color: 'var(--cyan)', fontSize: '0.75rem' }}>{p.certificate}</td>
                  <td>{p.time_ms}ms</td>
                </tr>
              ))}
            </tbody>
          </table>
        </GlowPanel>
      ) : loading ? (
        <GlowPanel title="LOADING...">
          <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--cyan)' }}>Loading verification data...</p>
        </GlowPanel>
      ) : (
        <GlowPanel title="NO VERIFICATION DATA">
          <p style={{ textAlign: 'center', padding: 'var(--gap-2xl)', color: 'var(--text-secondary)' }}>
            {isAdmin ? 'Click "RUN LEAN 4 VERIFICATION" to verify experiment results.' : 'Sign in as admin to run verification.'}
          </p>
        </GlowPanel>
      )}

      {/* Lean 4 source preview */}
      {proofs.length > 0 && (
        <GlowPanel title="LEAN 4 SOURCE PREVIEW" style={{ marginTop: 'var(--gap-lg)' }}>
          <div style={{ maxHeight: 300, overflow: 'auto' }}>
            {proofs.filter(p => p.lean4).slice(0, 3).map((p, i) => (
              <div key={i} style={{ marginBottom: 'var(--gap-md)' }}>
                <div style={{ color: 'var(--cyan)', fontSize: '0.7rem', marginBottom: 4 }}>
                  {p.status === 'proved' ? '✓' : '✗'} {p.theorem}
                </div>
                <pre style={{
                  background: '#0a0f1a', padding: 12, borderRadius: 8,
                  fontSize: '0.65rem', color: '#a0b4d4', overflow: 'auto',
                  border: '1px solid var(--border-dim)', lineHeight: 1.5,
                }}>{p.lean4.trim()}</pre>
              </div>
            ))}
          </div>
        </GlowPanel>
      )}

      {data?.timestamp && (
        <div style={{ textAlign: 'right', marginTop: 'var(--gap-md)', fontSize: '0.65rem', color: 'var(--text-tertiary)' }}>
          Verified: {new Date(data.timestamp).toLocaleString()} | {data.elapsed_ms}ms
        </div>
      )}
    </div>
  );
}
