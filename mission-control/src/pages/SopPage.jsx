import { useState, useEffect } from 'react';
import GlowPanel from '../components/GlowPanel';
import { Play, CheckCircle, Clock, RefreshCw } from 'lucide-react';
import api from '../api/client';

export default function SopPage() {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  useEffect(() => {
    fetchSopData();
  }, []);

  const fetchSopData = async () => {
    try {
      const res = await api.getSopData();
      if (!res.error) setData(res);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const handleExecute = async (protocolId) => {
    setExecuting(protocolId);
    try {
      const res = await api.executeSop(protocolId);
      if (!res.error) {
        // Append the new result to the history
        setData(prev => ({
          ...prev,
          history: [res, ...(prev?.history || [])]
        }));
      }
    } catch (e) {
      console.error(e);
    } finally {
      setExecuting(false);
    }
  };

  if (loading) {
    return (
      <GlowPanel title="LOADING...">
        <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--cyan)' }}>Loading SOP Protocols...</p>
      </GlowPanel>
    );
  }

  const protocols = data?.protocols || [];
  const history = data?.history || [];

  return (
    <div>
      <div className="page-header">
        <h2>SOP & REPRODUCIBILITY</h2>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
          Execute and validate scientific artifacts natively on GCP serverless.
        </p>
      </div>

      <div className="grid-2" style={{ gap: 'var(--gap-xl)', marginBottom: 'var(--gap-xl)' }}>
        {protocols.map(p => (
          <GlowPanel key={p.id} title={p.name} className="animate-in">
            <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: 'var(--gap-md)' }}>
              {p.description}
            </p>
            <div style={{ background: '#0a0f1a', padding: 12, borderRadius: 8, marginBottom: 'var(--gap-md)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: 4 }}>
                <span style={{ color: 'var(--text-tertiary)' }}>Baseline Metric:</span>
                <span style={{ color: 'var(--cyan)' }}>{p.baseline_metric}</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem' }}>
                <span style={{ color: 'var(--text-tertiary)' }}>Est. Time / Cost:</span>
                <span style={{ color: 'var(--green)' }}>{p.estimated_time} / {p.cost}</span>
              </div>
            </div>
            
            <button 
              className="btn btn-primary" 
              style={{ width: '100%', display: 'flex', justifyContent: 'center' }}
              disabled={executing !== false}
              onClick={() => handleExecute(p.id)}
            >
              {executing === p.id ? (
                <><RefreshCw size={14} className="spin" /> EXECUTING...</>
              ) : (
                <><Play size={14} /> REPRODUCE EXPERIMENT</>
              )}
            </button>
          </GlowPanel>
        ))}
      </div>

      <GlowPanel title="SOP EXECUTION HISTORY & COMPARISON">
        {history.length > 0 ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>Protocol ID</th>
                <th>Execution ID</th>
                <th>Metric Achieved</th>
                <th>Deviance</th>
                <th>Status</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {history.map(h => (
                <tr key={h.execution_id}>
                  <td style={{ fontWeight: 'bold', color: 'var(--cyan)' }}>{h.protocol_id}</td>
                  <td style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{h.execution_id}</td>
                  <td>{h.result?.metric_achieved}</td>
                  <td style={{ color: h.result?.deviance?.includes('+') ? 'var(--amber)' : 'var(--green)' }}>
                    {h.result?.deviance}
                  </td>
                  <td>
                    <span className={`badge ${h.status === 'success' ? 'verified' : 'failed'}`}>
                      {h.status === 'success' ? <CheckCircle size={10} /> : <Clock size={10} />}
                      {' '} {h.result?.validation || h.status}
                    </span>
                  </td>
                  <td style={{ fontSize: '0.8rem' }}>{h.result?.execution_time}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--text-secondary)' }}>
            No executions recorded. Run a protocol above to see results.
          </p>
        )}
      </GlowPanel>
    </div>
  );
}
