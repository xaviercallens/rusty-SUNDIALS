import { useState, useEffect, useCallback } from 'react';
import GlowPanel from '../components/GlowPanel';
import { FileText, Download, Play, RefreshCw } from 'lucide-react';
import { useAuth } from '../hooks/useAuth';
import api from '../api/client';

export default function PublicationsPage() {
  const { role } = useAuth();
  const isAdmin = role === 'admin';
  const [report, setReport] = useState(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [showLatex, setShowLatex] = useState(false);

  useEffect(() => {
    api.getReport()
      .then(d => { if (d && !d.error && !d.status?.includes('error')) setReport(d); setLoading(false); })
      .catch(() => setLoading(false));
  }, []);

  const generate = useCallback(async () => {
    setGenerating(true);
    try {
      const res = await api.generateReport();
      if (!res.error) setReport(res);
    } catch (e) { console.error(e); }
    finally { setGenerating(false); }
  }, []);

  const downloadLatex = useCallback(() => {
    if (!report?.latex_source) return;
    const blob = new Blob([report.latex_source], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = 'oxidize_cyclo_paper.tex'; a.click();
    URL.revokeObjectURL(url);
  }, [report]);

  const sections = report?.sections || [];
  const equations = report?.equations || [];

  return (
    <div>
      <div className="page-header">
        <h2>PUBLICATIONS MANAGER</h2>
        <button className="btn btn-primary" onClick={generate}
                disabled={generating || !isAdmin}>
          {generating ? <><RefreshCw size={14} className="spin" /> GENERATING...</> : <><Play size={14} /> GENERATE REPORT</>}
        </button>
      </div>

      {report ? (
        <>
          {/* Paper header */}
          <GlowPanel title="AUTO-GENERATED PAPER" className="animate-in">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <div>
                <h3 style={{ color: 'var(--text-primary)', margin: 0, fontSize: '1rem' }}>
                  <FileText size={16} style={{ color: 'var(--cyan)', marginRight: 8 }} />
                  {report.title}
                </h3>
                <div style={{ marginTop: 8, fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                  <span style={{ color: 'var(--green)' }}>{report.authors?.join(', ')}</span>
                  <span style={{ margin: '0 12px' }}>•</span>
                  <span>{report.institution}</span>
                  <span style={{ margin: '0 12px' }}>•</span>
                  <span>{report.date}</span>
                </div>
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <button className="btn btn-outline" onClick={downloadLatex}>
                  <Download size={12} /> LaTeX
                </button>
                <button className="btn btn-outline" onClick={() => setShowLatex(!showLatex)}>
                  {showLatex ? 'HIDE' : 'VIEW'} SOURCE
                </button>
              </div>
            </div>
          </GlowPanel>

          {/* Abstract */}
          <GlowPanel title="ABSTRACT" style={{ marginTop: 'var(--gap-md)' }} className="animate-in">
            <p style={{ fontSize: '0.8rem', lineHeight: 1.7, color: 'var(--text-secondary)', fontStyle: 'italic' }}>
              {report.abstract}
            </p>
          </GlowPanel>

          {/* Key equations */}
          {equations.length > 0 && (
            <GlowPanel title="KEY EQUATIONS" style={{ marginTop: 'var(--gap-md)' }} className="animate-in">
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: 'var(--gap-md)' }}>
                {equations.map((eq, i) => (
                  <div key={i} style={{
                    background: '#0a0f1a', padding: 16, borderRadius: 8,
                    border: '1px solid var(--border-dim)',
                  }}>
                    <div style={{ color: 'var(--cyan)', fontSize: '0.7rem', marginBottom: 8 }}>{eq.name}</div>
                    <div style={{ color: 'var(--green)', fontSize: '0.75rem', fontFamily: 'JetBrains Mono', wordBreak: 'break-all' }}>
                      {eq.latex}
                    </div>
                  </div>
                ))}
              </div>
            </GlowPanel>
          )}

          {/* Sections with results */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(350px, 1fr))', gap: 'var(--gap-md)', marginTop: 'var(--gap-md)' }}>
            {sections.filter(s => s.key_results).map((s, i) => (
              <GlowPanel key={i} title={s.name.toUpperCase()} className="animate-in">
                <table className="data-table">
                  <tbody>
                    {Object.entries(s.key_results).filter(([, v]) => v != null).map(([k, v]) => (
                      <tr key={k}>
                        <td style={{ color: 'var(--text-secondary)', fontSize: '0.7rem' }}>{k.replace(/_/g, ' ')}</td>
                        <td style={{ color: 'var(--green)', fontWeight: 'bold', textAlign: 'right' }}>
                          {typeof v === 'number' ? (v < 0.01 ? v.toExponential(3) : v.toFixed ? Number(v.toFixed(6)) : v) : v}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </GlowPanel>
            ))}
          </div>

          {/* LaTeX source */}
          {showLatex && report.latex_source && (
            <GlowPanel title="LaTeX SOURCE" style={{ marginTop: 'var(--gap-md)' }}>
              <pre style={{
                background: '#0a0f1a', padding: 16, borderRadius: 8,
                fontSize: '0.6rem', color: '#a0b4d4', overflow: 'auto',
                maxHeight: 400, lineHeight: 1.4, border: '1px solid var(--border-dim)',
              }}>{report.latex_source}</pre>
            </GlowPanel>
          )}

          {report.generated_at && (
            <div style={{ textAlign: 'right', marginTop: 'var(--gap-md)', fontSize: '0.65rem', color: 'var(--text-tertiary)' }}>
              Generated: {new Date(report.generated_at).toLocaleString()} | {report.elapsed_ms}ms
            </div>
          )}
        </>
      ) : loading ? (
        <GlowPanel title="LOADING...">
          <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--cyan)' }}>Loading report...</p>
        </GlowPanel>
      ) : (
        <GlowPanel title="NO REPORT GENERATED">
          <p style={{ textAlign: 'center', padding: 'var(--gap-2xl)', color: 'var(--text-secondary)' }}>
            {isAdmin ? 'Click "GENERATE REPORT" to create a scientific publication from results.' : 'Sign in as admin to generate reports.'}
          </p>
        </GlowPanel>
      )}
    </div>
  );
}
