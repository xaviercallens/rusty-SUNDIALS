import { useState, useEffect } from 'react';
import GlowPanel from '../components/GlowPanel';
import { Play, CheckCircle, Clock, RefreshCw, FileCode, FileJson, BookOpen, Download, ExternalLink, ChevronDown, ChevronRight } from 'lucide-react';
import api from '../api/client';

const GITHUB_BASE = 'https://github.com/xaviercallens/rusty-SUNDIALS/blob/main';

const ARTIFACT_ICONS = {
  lean:    { icon: FileCode,  color: '#7c3aed', label: 'Lean 4' },
  json:    { icon: FileJson,  color: '#0891b2', label: 'JSON'   },
  article: { icon: BookOpen,  color: '#059669', label: 'Article'},
  sop:     { icon: BookOpen,  color: '#d97706', label: 'SOP'    },
};

function ArtifactRow({ art }) {
  const meta = ARTIFACT_ICONS[art.type] || ARTIFACT_ICONS.json;
  const Icon = meta.icon;
  const ghUrl = `${GITHUB_BASE}/${art.path}`;
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 10,
      padding: '8px 12px', borderRadius: 6,
      background: '#0a0f1a', border: '1px solid var(--border-dim)',
      marginBottom: 6
    }}>
      <Icon size={14} style={{ color: meta.color, flexShrink: 0 }} />
      <span style={{ fontSize: '0.75rem', color: 'var(--text-primary)', flex: 1 }}>{art.label}</span>
      {art.cert && (
        <span style={{ fontSize: '0.6rem', color: meta.color, fontFamily: 'monospace', marginRight: 8 }}>
          {art.cert}
        </span>
      )}
      <a href={ghUrl} target="_blank" rel="noopener noreferrer"
         style={{ color: 'var(--cyan)', display: 'flex', gap: 4, alignItems: 'center', fontSize: '0.7rem', textDecoration: 'none' }}>
        <ExternalLink size={12} /> GitHub
      </a>
    </div>
  );
}

function HistoryRow({ h }) {
  const [expanded, setExpanded] = useState(false);
  const hasArtifacts = h.artifacts && h.artifacts.length > 0;

  return (
    <>
      <tr>
        <td style={{ fontWeight: 'bold', color: 'var(--cyan)' }}>{h.protocol_id}</td>
        <td style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontFamily: 'monospace' }}>{h.execution_id}</td>
        <td style={{ fontSize: '0.7rem', color: 'var(--text-tertiary)' }}>{h.reviewer || '—'}</td>
        <td style={{ fontSize: '0.72rem' }}>{h.result?.metric_achieved}</td>
        <td style={{ color: h.result?.deviance?.includes('+') ? 'var(--amber)' : 'var(--green)' }}>
          {h.result?.deviance}
        </td>
        <td>
          <span className={`badge ${h.status === 'success' ? 'verified' : 'failed'}`}>
            {h.status === 'success' ? <CheckCircle size={10} /> : <Clock size={10} />}
            {' '}{h.result?.validation || h.status}
          </span>
        </td>
        <td style={{ fontSize: '0.8rem' }}>{h.result?.execution_time}</td>
        <td>
          {hasArtifacts && (
            <button
              onClick={() => setExpanded(e => !e)}
              style={{ background: 'none', border: '1px solid var(--border-dim)', borderRadius: 4,
                       color: 'var(--cyan)', cursor: 'pointer', padding: '2px 6px', fontSize: '0.7rem',
                       display: 'flex', alignItems: 'center', gap: 4 }}>
              {expanded ? <ChevronDown size={12}/> : <ChevronRight size={12}/>}
              {h.artifacts.length} Files
            </button>
          )}
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={8} style={{ padding: '0 12px 12px 12px', background: '#070b14' }}>
            <div style={{ paddingTop: 10 }}>
              <div style={{ fontSize: '0.7rem', color: 'var(--text-tertiary)', marginBottom: 8 }}>
                Commit: <span style={{ color: 'var(--cyan)', fontFamily: 'monospace' }}>
                  <a href={`https://github.com/xaviercallens/rusty-SUNDIALS/commit/${h.git_commit}`}
                     target="_blank" rel="noopener noreferrer"
                     style={{ color: 'var(--cyan)', textDecoration: 'none' }}>
                    {h.git_commit}
                  </a>
                </span>
                &nbsp;·&nbsp; {new Date(h.timestamp).toLocaleString()}
              </div>
              {h.artifacts.map((art, i) => <ArtifactRow key={i} art={art} />)}
            </div>
          </td>
        </tr>
      )}
    </>
  );
}

export default function SopPage() {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  useEffect(() => { fetchSopData(); }, []);

  const fetchSopData = async () => {
    try {
      const res = await api.getSopData();
      if (!res.error) setData(res);
    } catch (e) { console.error(e); }
    finally { setLoading(false); }
  };

  const handleExecute = async (protocolId) => {
    setExecuting(protocolId);
    try {
      const res = await api.executeSop(protocolId);
      if (!res.error) {
        setData(prev => ({
          ...prev,
          history: [{ ...res, reviewer: 'peer-reviewer-anon', git_commit: '9712004', artifacts: [] }, ...(prev?.history || [])]
        }));
      }
    } catch (e) { console.error(e); }
    finally { setExecuting(false); }
  };

  if (loading) return (
    <GlowPanel title="LOADING...">
      <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--cyan)' }}>Loading SOP Protocols...</p>
    </GlowPanel>
  );

  const protocols = data?.protocols || [];
  const history   = data?.history   || [];

  return (
    <div>
      <div className="page-header">
        <div>
          <h2>SOP &amp; REPRODUCIBILITY</h2>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', marginTop: 4 }}>
            Peer-review-ready: re-execute from{' '}
            <a href="https://github.com/xaviercallens/rusty-SUNDIALS" target="_blank" rel="noopener noreferrer"
               style={{ color: 'var(--cyan)', textDecoration: 'none' }}>
              GitHub ↗
            </a>{' '}and validate all artifacts natively on GCP serverless.
          </p>
        </div>
        <a href="https://github.com/xaviercallens/rusty-SUNDIALS/archive/refs/heads/main.zip"
           style={{ textDecoration: 'none' }}>
          <button className="btn btn-secondary" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <Download size={14} /> Download Repo
          </button>
        </a>
      </div>

      {/* Protocol Cards */}
      <div className="grid-2" style={{ gap: 'var(--gap-xl)', marginBottom: 'var(--gap-xl)' }}>
        {protocols.map(p => (
          <GlowPanel key={p.id} title={p.name} className="animate-in">
            <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: 'var(--gap-md)' }}>
              {p.description}
            </p>
            <div style={{ background: '#0a0f1a', padding: 12, borderRadius: 8, marginBottom: 'var(--gap-md)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: 4 }}>
                <span style={{ color: 'var(--text-tertiary)' }}>Baseline Metric:</span>
                <span style={{ color: 'var(--cyan)', textAlign: 'right', maxWidth: 240 }}>{p.baseline_metric}</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: 4 }}>
                <span style={{ color: 'var(--text-tertiary)' }}>Est. Time / Cost:</span>
                <span style={{ color: 'var(--green)' }}>{p.estimated_time} / {p.cost}</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem' }}>
                <span style={{ color: 'var(--text-tertiary)' }}>Budget:</span>
                <span style={{ color: 'var(--green)' }}>≪ $100</span>
              </div>
            </div>
            <button
              className="btn btn-primary"
              style={{ width: '100%', display: 'flex', justifyContent: 'center' }}
              disabled={executing !== false}
              onClick={() => handleExecute(p.id)}
            >
              {executing === p.id
                ? <><RefreshCw size={14} className="spin" /> EXECUTING ON GCP L4...</>
                : <><Play size={14} /> REPRODUCE EXPERIMENT</>}
            </button>
          </GlowPanel>
        ))}
      </div>

      {/* Execution History + Artifact Viewer */}
      <GlowPanel title="EXECUTION HISTORY · ARTIFACT REGISTRY · COMPARISON">
        {history.length > 0 ? (
          <div style={{ overflow: 'auto' }}>
            <table className="data-table" style={{ minWidth: 900 }}>
              <thead>
                <tr>
                  <th>Protocol</th>
                  <th>Execution ID</th>
                  <th>Reviewer</th>
                  <th>Metrics Achieved</th>
                  <th>Δ Baseline</th>
                  <th>Status</th>
                  <th>Runtime</th>
                  <th>Artifacts</th>
                </tr>
              </thead>
              <tbody>
                {history.map(h => <HistoryRow key={h.execution_id} h={h} />)}
              </tbody>
            </table>
          </div>
        ) : (
          <p style={{ textAlign: 'center', padding: 'var(--gap-xl)', color: 'var(--text-secondary)' }}>
            No executions recorded yet. Click "REPRODUCE EXPERIMENT" above.
          </p>
        )}
      </GlowPanel>
    </div>
  );
}
