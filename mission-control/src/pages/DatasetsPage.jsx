import { useState, useMemo } from 'react';
import {
  Database, Search, Filter, ExternalLink, Download, Share2, Play,
  ChevronDown, ChevronRight, Star, BookOpen, Layers, CheckCircle2,
  AlertCircle, Clock, Eye, Plus, Copy, Atom, Zap, BarChart3, Globe
} from 'lucide-react';
import { MOCK_DATASETS, DATASET_CATEGORIES, DATASET_STATS } from '../api/datasetsMockData';

const STATUS_CONFIG = {
  integrated: { label: 'Integrated', class: 'verified', icon: CheckCircle2 },
  generated:  { label: 'Generated',  class: 'verified', icon: CheckCircle2 },
  active:     { label: 'Active',     class: 'active',   icon: Zap },
  available:  { label: 'Available',  class: 'pending',  icon: Clock },
  reference:  { label: 'Reference',  class: 'active',   icon: BookOpen },
  planned:    { label: 'Planned',    class: 'pending',  icon: Clock },
};

const TIER_LABELS = {
  1: { label: 'Tier 1 · Immediate', color: 'var(--green)' },
  2: { label: 'Tier 2 · Research',  color: 'var(--amber)' },
  3: { label: 'Tier 3 · Advanced',  color: 'var(--purple)' },
};

const CATEGORY_ICONS = {
  fusion:         Atom,
  mhd:            Zap,
  visualization:  Eye,
  disruption:     AlertCircle,
  computational:  BarChart3,
};

/* ──────────────────────────────────────────────────────────────── */
/*  Stat Card                                                      */
/* ──────────────────────────────────────────────────────────────── */
function StatCard({ label, value, accent = 'var(--cyan)', icon: Icon }) {
  return (
    <div className="metric-card ds-stat-card">
      <div className="metric-label">{label}</div>
      <div className="metric-value" style={{ color: accent }}>
        {Icon && <Icon size={18} style={{ marginRight: 6, opacity: 0.7 }} />}
        {value}
      </div>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────────── */
/*  Dataset Card (Grid View)                                       */
/* ──────────────────────────────────────────────────────────────── */
function DatasetCard({ ds, onSelect, selected }) {
  const statusCfg = STATUS_CONFIG[ds.status] || STATUS_CONFIG.available;
  const StatusIcon = statusCfg.icon;
  const tierCfg = TIER_LABELS[ds.tier];
  const CatIcon = CATEGORY_ICONS[ds.category] || Database;

  return (
    <div
      className={`glow-panel ds-card animate-in ${selected ? 'ds-card-selected' : ''}`}
      onClick={() => onSelect(ds.id)}
      id={`dataset-card-${ds.id}`}
    >
      {/* Header */}
      <div className="ds-card-header">
        <div className="ds-card-icon" style={{ color: tierCfg.color }}>
          <CatIcon size={22} />
        </div>
        <div className="ds-card-title-block">
          <h4 className="ds-card-name">{ds.name}</h4>
          <span className="ds-card-subtitle">{ds.subtitle}</span>
        </div>
        <span className={`badge ${statusCfg.class}`}>
          <StatusIcon size={10} /> {statusCfg.label}
        </span>
      </div>

      {/* Description */}
      <p className="ds-card-desc">{ds.description.slice(0, 140)}…</p>

      {/* Params */}
      <div className="ds-card-params">
        {ds.parameters.slice(0, 3).map((p, i) => (
          <div key={i} className="ds-param-chip">
            <span className="ds-param-name">{p.name}</span>
            <span className="ds-param-val">{p.value}</span>
            {p.unit && <span className="ds-param-unit">{p.unit}</span>}
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="ds-card-footer">
        <span className="ds-card-meta" style={{ color: tierCfg.color }}>
          {tierCfg.label}
        </span>
        <span className="ds-card-meta">
          <Star size={11} /> {ds.citations} citations
        </span>
        <span className="ds-card-meta">{ds.source}</span>
      </div>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────────── */
/*  Dataset Detail Panel                                           */
/* ──────────────────────────────────────────────────────────────── */
function DatasetDetail({ ds, onClose }) {
  const [copiedCmd, setCopiedCmd] = useState(false);
  const statusCfg = STATUS_CONFIG[ds.status] || STATUS_CONFIG.available;
  const StatusIcon = statusCfg.icon;
  const tierCfg = TIER_LABELS[ds.tier];
  const CatIcon = CATEGORY_ICONS[ds.category] || Database;
  const intStatus = STATUS_CONFIG[ds.integration.status] || STATUS_CONFIG.planned;

  const handleCopy = (text) => {
    navigator.clipboard.writeText(text);
    setCopiedCmd(true);
    setTimeout(() => setCopiedCmd(false), 2000);
  };

  return (
    <div className="ds-detail-overlay" onClick={onClose}>
      <div className="ds-detail-panel glow-panel" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="ds-detail-header">
          <div className="ds-detail-icon" style={{ color: tierCfg.color }}>
            <CatIcon size={28} />
          </div>
          <div style={{ flex: 1 }}>
            <h2 style={{ fontSize: '1.1rem', marginBottom: 4 }}>{ds.name}</h2>
            <span className="ds-card-subtitle">{ds.subtitle}</span>
          </div>
          <span className={`badge ${statusCfg.class}`} style={{ fontSize: '0.7rem', padding: '4px 12px' }}>
            <StatusIcon size={12} /> {statusCfg.label}
          </span>
          <button className="btn btn-outline" onClick={onClose} style={{ padding: '6px 12px', marginLeft: 8 }}>✕</button>
        </div>

        {/* Description */}
        <p style={{ color: 'var(--text-secondary)', lineHeight: 1.7, fontSize: '0.85rem', margin: '16px 0' }}>
          {ds.description}
        </p>

        {/* Meta Grid */}
        <div className="ds-meta-grid">
          <div className="ds-meta-item"><span className="label">Source</span><span className="data-value">{ds.source}</span></div>
          <div className="ds-meta-item"><span className="label">License</span><span className="data-value">{ds.license}</span></div>
          <div className="ds-meta-item"><span className="label">Format</span><span className="data-value">{ds.format}</span></div>
          <div className="ds-meta-item"><span className="label">Size</span><span className="data-value">{ds.size}</span></div>
          <div className="ds-meta-item"><span className="label">Version</span><span className="data-value">{ds.version}</span></div>
          <div className="ds-meta-item"><span className="label">Updated</span><span className="data-value">{ds.lastUpdated}</span></div>
        </div>

        {/* Parameters */}
        <div className="panel-title" style={{ marginTop: 20 }}><span className="dot" /> Parameters</div>
        <div className="ds-params-table">
          {ds.parameters.map((p, i) => (
            <div key={i} className="ds-param-row">
              <span className="ds-param-label">{p.name}</span>
              <span className="ds-param-value">{p.value}</span>
              <span className="ds-param-unit-lg">{p.unit}</span>
            </div>
          ))}
        </div>

        {/* Tasks */}
        <div className="panel-title" style={{ marginTop: 20 }}><span className="dot" /> Available Tasks</div>
        <div className="ds-tasks-list">
          {ds.tasks.map((t, i) => (
            <div key={i} className="ds-task-item">
              <CheckCircle2 size={13} style={{ color: 'var(--green)', flexShrink: 0 }} />
              <span>{t}</span>
            </div>
          ))}
        </div>

        {/* Integration */}
        <div className="panel-title" style={{ marginTop: 20 }}><span className="dot" /> Integration with rusty-SUNDIALS</div>
        <div className="ds-integration-block">
          <div className="ds-meta-item"><span className="label">Target</span><span className="data-value" style={{ fontSize: '0.75rem' }}>{ds.integration.target}</span></div>
          <div className="ds-meta-item"><span className="label">Data Flow</span><span className="data-value" style={{ fontSize: '0.75rem' }}>{ds.integration.dataFlow}</span></div>
          <div className="ds-meta-item"><span className="label">Status</span><span className={`badge ${intStatus.class}`}>{intStatus.label}</span></div>
          {ds.integration.usedBy.length > 0 && (
            <div className="ds-meta-item">
              <span className="label">Used By</span>
              <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                {ds.integration.usedBy.map((f, i) => (
                  <span key={i} className="badge active" style={{ fontSize: '0.6rem' }}>{f}</span>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Download / Install */}
        {ds.downloadCmd && (
          <>
            <div className="panel-title" style={{ marginTop: 20 }}><span className="dot" /> Install / Download</div>
            <div className="ds-cmd-block">
              <code>{ds.downloadCmd}</code>
              <button
                className="btn btn-outline"
                style={{ padding: '4px 10px', fontSize: '0.6rem' }}
                onClick={() => handleCopy(ds.downloadCmd)}
              >
                {copiedCmd ? '✓ Copied' : <><Copy size={12} /> Copy</>}
              </button>
            </div>
          </>
        )}

        {/* Actions */}
        <div className="ds-detail-actions">
          {ds.url && (
            <a href={ds.url} target="_blank" rel="noopener noreferrer" className="btn btn-primary" style={{ textDecoration: 'none' }}>
              <ExternalLink size={14} /> Open Repository
            </a>
          )}
          {ds.shared && (
            <button className="btn btn-outline" onClick={() => handleCopy(ds.url || ds.downloadCmd || ds.name)}>
              <Share2 size={14} /> Share
            </button>
          )}
          {(ds.status === 'integrated' || ds.status === 'generated') && (
            <button className="btn btn-outline" style={{ borderColor: 'var(--green-dim)', color: 'var(--green)' }}>
              <Play size={14} /> Run Experiment
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────────── */
/*  Main DatasetsPage                                              */
/* ──────────────────────────────────────────────────────────────── */
export default function DatasetsPage() {
  const [search, setSearch] = useState('');
  const [categoryFilter, setCategoryFilter] = useState('all');
  const [tierFilter, setTierFilter] = useState('all');
  const [statusFilter, setStatusFilter] = useState('all');
  const [selectedId, setSelectedId] = useState(null);

  const filtered = useMemo(() => {
    return MOCK_DATASETS.filter(ds => {
      const matchesSearch = !search ||
        ds.name.toLowerCase().includes(search.toLowerCase()) ||
        ds.subtitle.toLowerCase().includes(search.toLowerCase()) ||
        ds.description.toLowerCase().includes(search.toLowerCase()) ||
        ds.source.toLowerCase().includes(search.toLowerCase());
      const matchesCat = categoryFilter === 'all' || ds.category === categoryFilter;
      const matchesTier = tierFilter === 'all' || ds.tier === Number(tierFilter);
      const matchesStatus = statusFilter === 'all' || ds.status === statusFilter;
      return matchesSearch && matchesCat && matchesTier && matchesStatus;
    });
  }, [search, categoryFilter, tierFilter, statusFilter]);

  const selectedDs = MOCK_DATASETS.find(d => d.id === selectedId);

  return (
    <div className="ds-page">
      {/* Header */}
      <div className="page-header">
        <h2><Database size={22} style={{ marginRight: 10, verticalAlign: 'middle' }} />Scientific Datasets</h2>
        <span className="breadcrumb">Mission Control <span>›</span> Datasets <span>›</span> ITER / Fusion</span>
      </div>

      {/* Stats Row */}
      <div className="grid-4 ds-stats-row animate-in">
        <StatCard label="Total Datasets" value={DATASET_STATS.total} accent="var(--cyan)" icon={Database} />
        <StatCard label="Integrated" value={DATASET_STATS.integrated} accent="var(--green)" icon={CheckCircle2} />
        <StatCard label="Total Citations" value={DATASET_STATS.totalCitations} accent="var(--amber)" icon={Star} />
        <StatCard label="Generated Files" value={DATASET_STATS.totalFiles} accent="var(--purple)" icon={Layers} />
      </div>

      {/* Pipeline Diagram */}
      <div className="glow-panel ds-pipeline-panel animate-in" style={{ animationDelay: '50ms' }}>
        <div className="panel-title"><span className="dot" /> Data Pipeline: Datasets → Experiments</div>
        <div className="ds-pipeline-flow">
          <div className="ds-pipeline-node" style={{ borderColor: 'var(--purple)' }}>
            <Globe size={18} style={{ color: 'var(--purple)' }} />
            <span>ITER / MAST</span>
            <small>TokaMark, IMAS, FAIR-MAST</small>
          </div>
          <div className="ds-pipeline-arrow">→</div>
          <div className="ds-pipeline-node" style={{ borderColor: 'var(--amber)' }}>
            <Zap size={18} style={{ color: 'var(--amber)' }} />
            <span>Preprocessing</span>
            <small>FreeGS, PlasmaPy, imas-python</small>
          </div>
          <div className="ds-pipeline-arrow">→</div>
          <div className="ds-pipeline-node" style={{ borderColor: 'var(--cyan)' }}>
            <Database size={18} style={{ color: 'var(--cyan)' }} />
            <span>data/fusion/</span>
            <small>CSV, Rust constants, HDF5</small>
          </div>
          <div className="ds-pipeline-arrow">→</div>
          <div className="ds-pipeline-node" style={{ borderColor: 'var(--green)' }}>
            <Play size={18} style={{ color: 'var(--green)' }} />
            <span>CVODE Solver</span>
            <small>MHD benchmarks, tearing modes</small>
          </div>
          <div className="ds-pipeline-arrow">→</div>
          <div className="ds-pipeline-node" style={{ borderColor: 'var(--red)' }}>
            <Eye size={18} style={{ color: 'var(--red)' }} />
            <span>Visualization</span>
            <small>IMAS-ParaView, VR</small>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="ds-filters animate-in" style={{ animationDelay: '100ms' }}>
        <div className="ds-search-wrapper">
          <Search size={16} className="ds-search-icon" />
          <input
            id="dataset-search"
            type="text"
            placeholder="Search datasets, sources, keywords…"
            value={search}
            onChange={e => setSearch(e.target.value)}
            className="ds-search-input"
          />
        </div>
        <div className="ds-filter-group">
          <select id="filter-category" value={categoryFilter} onChange={e => setCategoryFilter(e.target.value)} className="ds-select">
            <option value="all">All Categories</option>
            {DATASET_CATEGORIES.map(c => <option key={c.id} value={c.id}>{c.label}</option>)}
          </select>
          <select id="filter-tier" value={tierFilter} onChange={e => setTierFilter(e.target.value)} className="ds-select">
            <option value="all">All Tiers</option>
            <option value="1">Tier 1 · Immediate</option>
            <option value="2">Tier 2 · Research</option>
            <option value="3">Tier 3 · Advanced</option>
          </select>
          <select id="filter-status" value={statusFilter} onChange={e => setStatusFilter(e.target.value)} className="ds-select">
            <option value="all">All Status</option>
            <option value="integrated">Integrated</option>
            <option value="generated">Generated</option>
            <option value="available">Available</option>
            <option value="reference">Reference</option>
          </select>
        </div>
      </div>

      {/* Results count */}
      <div className="ds-results-info animate-in" style={{ animationDelay: '120ms' }}>
        <span className="label">{filtered.length} dataset{filtered.length !== 1 ? 's' : ''} found</span>
      </div>

      {/* Dataset Grid */}
      <div className="ds-grid">
        {filtered.map(ds => (
          <DatasetCard
            key={ds.id}
            ds={ds}
            onSelect={setSelectedId}
            selected={selectedId === ds.id}
          />
        ))}
        {filtered.length === 0 && (
          <div className="ds-empty">
            <Database size={48} style={{ color: 'var(--text-tertiary)', marginBottom: 16 }} />
            <p>No datasets match your filters.</p>
          </div>
        )}
      </div>

      {/* Generated Files */}
      <div className="glow-panel ds-generated-panel animate-in" style={{ animationDelay: '200ms' }}>
        <div className="panel-title"><span className="dot" /> Generated Data Files</div>
        <table className="data-table">
          <thead>
            <tr>
              <th>File</th>
              <th>Size</th>
              <th>Path</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {DATASET_STATS.generatedFiles.map((f, i) => (
              <tr key={i}>
                <td style={{ color: 'var(--cyan)' }}>{f.name}</td>
                <td>{f.size}</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>{f.path}</td>
                <td>
                  <button className="btn btn-outline" style={{ padding: '3px 10px', fontSize: '0.6rem' }}>
                    <Eye size={11} /> View
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Detail Modal */}
      {selectedDs && <DatasetDetail ds={selectedDs} onClose={() => setSelectedId(null)} />}
    </div>
  );
}
