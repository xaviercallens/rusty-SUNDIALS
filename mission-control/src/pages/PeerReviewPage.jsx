import { useState, useEffect, useRef } from 'react';
import GlowPanel from '../components/GlowPanel';
import {
  Brain, Zap, MessageSquare, CheckCircle, XCircle,
  Clock, BarChart2, Award, RefreshCw, ChevronDown, ChevronUp,
  Star, AlertTriangle, Cpu
} from 'lucide-react';

// ── Static demo data (live data comes from /api/peer-review) ──────────────

const DEMO_REVIEWS = [
  {
    id: 'rev-001',
    method_name: 'FLAGNO_Divergence_Corrected',
    hypothesis_hash: 'a3f2d1c09b4e7f11',
    timestamp: '2026-05-15T07:14:33Z',
    consensus_score: 0.785,
    consensus_passed: true,
    lean4_cert: 'CERT-LEAN4-A3F2D1C09B4E',
    verdicts: [
      {
        reviewer: 'gwen',
        model_id: 'google/gemma-2-9b-it',
        score: 0.78,
        passed: true,
        critique: 'Method demonstrates strong physical grounding with ∇·B=0 preservation via Hodge projection. Krylov bound is mathematically sound. Energy conservation requires empirical validation at larger scales.',
        strengths: ['Divergence-free constraint rigorously enforced', 'Sub-linear Krylov scaling claim supported by theory'],
        weaknesses: ['No empirical evidence for claimed speedup factor at exascale', 'Convergence proof for stiffness κ > 10⁸ incomplete'],
        latency_ms: 1842,
      },
      {
        reviewer: 'deepthink',
        model_id: 'gemini-2.5-flash-preview-05-20',
        score: 0.82,
        passed: true,
        critique: 'Deep reasoning validates the Hamiltonian structure preservation. Fractional spectral convolution is theoretically novel for xMHD stiffness. Lean 4 certificate provides machine-verifiable provenance — a significant scientific advantage.',
        strengths: ['Lean 4 cert eliminates informal proof risks', 'Hamiltonian structure ensures long-term stability'],
        weaknesses: ['Speedup claims require CEA/ITER benchmark validation', 'Fractional-order parameter selection ad hoc'],
        latency_ms: 3201,
      },
      {
        reviewer: 'mistral',
        model_id: 'mistral-large-latest',
        score: 0.75,
        passed: true,
        critique: 'The ∇·B=0 enforcement satisfies Maxwell\'s equations. However, the O(1) Krylov bound requires proof under worst-case tearing mode conditions. Energy conservation is asserted but not demonstrated for stiff regimes.',
        strengths: ['∇·B=0 via Hodge decomposition is mathematically rigorous'],
        weaknesses: ['O(1) Krylov bound unverified for extreme stiffness (κ > 10⁸)', 'Reproducibility SOP needs explicit timestep specification'],
        latency_ms: 1124,
      },
    ],
  },
  {
    id: 'rev-002',
    method_name: 'Hamiltonian_Spectral_Relaxation',
    hypothesis_hash: 'b9c3e4f071a28d55',
    timestamp: '2026-05-15T06:52:11Z',
    consensus_score: 0.61,
    consensus_passed: false,
    lean4_cert: null,
    verdicts: [
      {
        reviewer: 'gwen',
        model_id: 'google/gemma-2-9b-it',
        score: 0.55,
        passed: false,
        critique: 'The scalar root-find approach cannot guarantee divergence-free enforcement in 3D tearing mode topology changes. Magnetic monopoles will appear at reconnection sites.',
        strengths: ['Symplectic 2-Form preservation is elegant'],
        weaknesses: ['∇·B = 0 not provably maintained at reconnection events', 'No Lean 4 certificate — informal proof only'],
        latency_ms: 1654,
      },
      {
        reviewer: 'deepthink',
        model_id: 'gemini-2.5-flash-preview-05-20',
        score: 0.68,
        passed: false,
        critique: 'The Poisson bracket approach is theoretically sound for smooth flows but fails at topological singularities. Tearing mode reconnection requires explicit Hodge projection which is missing here.',
        strengths: ['Hamiltonian formalism provides good energy bounds', 'Matrix-free formulation is computationally efficient'],
        weaknesses: ['No handling of topological singularities during reconnection', 'Expected speedup 12× is modest compared to state-of-art AMGX'],
        latency_ms: 2877,
      },
      {
        reviewer: 'mistral',
        model_id: 'mistral-large-latest',
        score: 0.60,
        passed: false,
        critique: 'Method fails peer review due to absence of formal divergence-free proof. Symplectic structure alone is insufficient for MHD — explicit ∇·B constraint must be enforced.',
        strengths: ['Energy conservation claim is credible'],
        weaknesses: ['No ∇·B enforcement mechanism specified', 'Lean 4 proof has unresolved `sorry` tactics'],
        latency_ms: 989,
      },
    ],
  },
];

// ── Helper components ─────────────────────────────────────────────────────

function ReviewerBadge({ reviewer }) {
  const cfg = {
    gwen:      { color: '#00d4aa', bg: 'rgba(0,212,170,0.12)', label: 'Gwen OSS', icon: '🟢' },
    deepthink: { color: '#4285f4', bg: 'rgba(66,133,244,0.12)', label: 'DeepThink', icon: '🔵' },
    mistral:   { color: '#ff7043', bg: 'rgba(255,112,67,0.12)', label: 'Mistral', icon: '🟠' },
  }[reviewer] || { color: '#888', bg: 'rgba(128,128,128,0.1)', label: reviewer, icon: '⚪' };

  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: 5,
      padding: '3px 10px', borderRadius: 20,
      background: cfg.bg, border: `1px solid ${cfg.color}40`,
      color: cfg.color, fontSize: '0.75rem', fontWeight: 700,
      letterSpacing: '0.04em',
    }}>
      <span>{cfg.icon}</span> {cfg.label}
    </span>
  );
}

function ScoreRing({ score, size = 56 }) {
  const pct = Math.round(score * 100);
  const passed = score >= 0.70;
  const color = score >= 0.80 ? '#00ff88' : score >= 0.70 ? '#ffcc00' : '#ff4444';
  const r = (size / 2) - 4;
  const circ = 2 * Math.PI * r;
  const dash = circ * (1 - score);

  return (
    <div style={{ position: 'relative', width: size, height: size, flexShrink: 0 }}>
      <svg width={size} height={size} style={{ transform: 'rotate(-90deg)' }}>
        <circle cx={size/2} cy={size/2} r={r} fill="none"
          stroke="rgba(255,255,255,0.08)" strokeWidth={4} />
        <circle cx={size/2} cy={size/2} r={r} fill="none"
          stroke={color} strokeWidth={4}
          strokeDasharray={circ} strokeDashoffset={dash}
          strokeLinecap="round"
          style={{ transition: 'stroke-dashoffset 1s ease', filter: `drop-shadow(0 0 4px ${color})` }} />
      </svg>
      <div style={{
        position: 'absolute', inset: 0,
        display: 'flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center',
      }}>
        <span style={{ fontSize: size * 0.22, fontWeight: 900, color, lineHeight: 1 }}>{pct}</span>
        <span style={{ fontSize: size * 0.14, color: 'rgba(255,255,255,0.5)', lineHeight: 1 }}>%</span>
      </div>
    </div>
  );
}

function VerdictCard({ verdict, defaultOpen = false }) {
  const [open, setOpen] = useState(defaultOpen);
  const passed = verdict.passed;

  return (
    <div style={{
      border: `1px solid ${passed ? 'rgba(0,255,136,0.2)' : 'rgba(255,68,68,0.2)'}`,
      borderRadius: 12, overflow: 'hidden', marginBottom: 10,
      background: passed ? 'rgba(0,255,136,0.03)' : 'rgba(255,68,68,0.03)',
    }}>
      {/* Header */}
      <div
        onClick={() => setOpen(o => !o)}
        style={{
          display: 'flex', alignItems: 'center', gap: 12,
          padding: '12px 16px', cursor: 'pointer',
          background: 'rgba(255,255,255,0.02)',
        }}
      >
        <ScoreRing score={verdict.score} size={44} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <ReviewerBadge reviewer={verdict.reviewer} />
            {passed
              ? <CheckCircle size={14} color="#00ff88" />
              : <XCircle size={14} color="#ff4444" />}
            <span style={{ fontSize: '0.7rem', color: 'rgba(255,255,255,0.4)' }}>
              {verdict.model_id}
            </span>
          </div>
          <p style={{
            fontSize: '0.78rem', color: 'rgba(255,255,255,0.65)',
            margin: '4px 0 0', lineHeight: 1.4,
            overflow: 'hidden', textOverflow: 'ellipsis',
            display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical',
          }}>
            {verdict.critique}
          </p>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: '0.68rem', color: 'rgba(255,255,255,0.35)',
                         display: 'flex', alignItems: 'center', gap: 3 }}>
            <Clock size={10} /> {verdict.latency_ms}ms
          </span>
          {open ? <ChevronUp size={16} color="rgba(255,255,255,0.4)" />
                : <ChevronDown size={16} color="rgba(255,255,255,0.4)" />}
        </div>
      </div>

      {/* Expanded detail */}
      {open && (
        <div style={{ padding: '12px 16px', borderTop: '1px solid rgba(255,255,255,0.05)' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
            <div>
              <div style={{ fontSize: '0.68rem', color: '#00ff88', fontWeight: 700,
                            letterSpacing: '0.06em', marginBottom: 6 }}>STRENGTHS</div>
              {verdict.strengths.map((s, i) => (
                <div key={i} style={{ display: 'flex', gap: 6, marginBottom: 4,
                                      fontSize: '0.75rem', color: 'rgba(255,255,255,0.7)' }}>
                  <CheckCircle size={12} color="#00ff88" style={{ flexShrink: 0, marginTop: 2 }} />
                  {s}
                </div>
              ))}
            </div>
            <div>
              <div style={{ fontSize: '0.68rem', color: '#ff6666', fontWeight: 700,
                            letterSpacing: '0.06em', marginBottom: 6 }}>WEAKNESSES</div>
              {verdict.weaknesses.map((w, i) => (
                <div key={i} style={{ display: 'flex', gap: 6, marginBottom: 4,
                                      fontSize: '0.75rem', color: 'rgba(255,255,255,0.7)' }}>
                  <AlertTriangle size={12} color="#ff6666" style={{ flexShrink: 0, marginTop: 2 }} />
                  {w}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function ReviewCard({ review, index }) {
  const passed = review.consensus_passed;
  const pct = Math.round(review.consensus_score * 100);
  const passedCount = review.verdicts.filter(v => v.passed).length;
  const ts = new Date(review.timestamp).toLocaleTimeString();

  return (
    <GlowPanel title={
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ fontFamily: 'monospace', color: 'rgba(255,255,255,0.5)',
                       fontSize: '0.7rem' }}>#{String(index+1).padStart(2,'0')}</span>
        <span style={{ color: 'var(--accent-primary)', maxWidth: 220,
                       overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {review.method_name}
        </span>
        <span style={{
          padding: '2px 8px', borderRadius: 20, fontSize: '0.65rem', fontWeight: 700,
          background: passed ? 'rgba(0,255,136,0.15)' : 'rgba(255,68,68,0.15)',
          color: passed ? '#00ff88' : '#ff4444',
          border: `1px solid ${passed ? 'rgba(0,255,136,0.3)' : 'rgba(255,68,68,0.3)'}`,
        }}>
          {passed ? '✅ ACCEPTED' : '❌ REJECTED'}
        </span>
      </div>
    }>
      {/* Consensus row */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 16,
                    padding: '12px 14px', borderRadius: 10,
                    background: 'rgba(255,255,255,0.03)',
                    border: '1px solid rgba(255,255,255,0.06)' }}>
        <ScoreRing score={review.consensus_score} size={64} />
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: '0.68rem', color: 'rgba(255,255,255,0.5)',
                        letterSpacing: '0.06em', marginBottom: 4 }}>
            CONSENSUS SCORE
          </div>
          <div style={{ fontSize: '1.4rem', fontWeight: 900,
                        color: passed ? '#00ff88' : '#ff4444', lineHeight: 1 }}>
            {pct}%
          </div>
          <div style={{ fontSize: '0.72rem', color: 'rgba(255,255,255,0.5)', marginTop: 4 }}>
            {passedCount}/{review.verdicts.length} reviewers passed · Median of all scores
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          {review.lean4_cert && (
            <div style={{ fontSize: '0.65rem', color: 'rgba(0,255,136,0.7)',
                          fontFamily: 'monospace', marginBottom: 4 }}>
              🔐 {review.lean4_cert}
            </div>
          )}
          <div style={{ fontSize: '0.65rem', color: 'rgba(255,255,255,0.35)',
                        fontFamily: 'monospace' }}>
            {review.hypothesis_hash}
          </div>
          <div style={{ fontSize: '0.65rem', color: 'rgba(255,255,255,0.35)', marginTop: 2 }}>
            <Clock size={10} style={{ display: 'inline', marginRight: 3 }} />{ts}
          </div>
        </div>
      </div>

      {/* Individual verdicts */}
      <div style={{ fontSize: '0.68rem', color: 'rgba(255,255,255,0.4)',
                    letterSpacing: '0.06em', marginBottom: 8 }}>
        INDIVIDUAL VERDICTS
      </div>
      {review.verdicts.map((v, i) => (
        <VerdictCard key={v.reviewer} verdict={v} defaultOpen={i === 0 && passed} />
      ))}
    </GlowPanel>
  );
}

// ── Stats bar ──────────────────────────────────────────────────────────────

function StatsBar({ reviews }) {
  const total = reviews.length;
  const accepted = reviews.filter(r => r.consensus_passed).length;
  const avgScore = total > 0
    ? reviews.reduce((s, r) => s + r.consensus_score, 0) / total : 0;
  const avgLatency = total > 0
    ? reviews.reduce((s, r) => {
        const l = r.verdicts.reduce((a, v) => a + v.latency_ms, 0) / r.verdicts.length;
        return s + l;
      }, 0) / total : 0;

  const stats = [
    { label: 'TOTAL REVIEWS', value: total, icon: <MessageSquare size={16} />, color: '#4285f4' },
    { label: 'ACCEPTED', value: accepted, icon: <CheckCircle size={16} />, color: '#00ff88' },
    { label: 'REJECTED', value: total - accepted, icon: <XCircle size={16} />, color: '#ff4444' },
    { label: 'AVG SCORE', value: `${Math.round(avgScore * 100)}%`,
      icon: <Star size={16} />, color: '#ffcc00' },
    { label: 'AVG LATENCY', value: `${Math.round(avgLatency)}ms`,
      icon: <Clock size={16} />, color: '#ff7043' },
  ];

  return (
    <div style={{ display: 'flex', gap: 12, marginBottom: 20, flexWrap: 'wrap' }}>
      {stats.map(s => (
        <div key={s.label} style={{
          flex: '1 1 120px', padding: '12px 16px',
          background: 'rgba(255,255,255,0.03)',
          border: '1px solid rgba(255,255,255,0.07)',
          borderRadius: 10, display: 'flex', alignItems: 'center', gap: 10,
        }}>
          <span style={{ color: s.color, opacity: 0.8 }}>{s.icon}</span>
          <div>
            <div style={{ fontSize: '0.62rem', color: 'rgba(255,255,255,0.4)',
                          letterSpacing: '0.05em' }}>{s.label}</div>
            <div style={{ fontSize: '1.1rem', fontWeight: 900, color: s.color,
                          lineHeight: 1.1 }}>{s.value}</div>
          </div>
        </div>
      ))}
    </div>
  );
}

// ── Reviewer legend ────────────────────────────────────────────────────────

function ReviewerLegend() {
  const reviewers = [
    {
      id: 'gwen',
      name: 'Gwen (Gemma-2 9B)',
      desc: 'Open-source Google model via HuggingFace / local vLLM',
      env: 'GWEN_API_URL or HUGGINGFACE_API_TOKEN',
      color: '#00d4aa',
      icon: '🟢',
      source: 'OSS',
    },
    {
      id: 'deepthink',
      name: 'Google DeepThink',
      desc: 'Gemini 2.5 Flash with extended thinking mode (8192 token budget)',
      env: 'GEMINI_API_KEY or Vertex AI ADC',
      color: '#4285f4',
      icon: '🔵',
      source: 'Google',
    },
    {
      id: 'mistral',
      name: 'Mistral Large',
      desc: 'Mistral AI frontier model via api.mistral.ai with JSON mode',
      env: 'MISTRAL_API_KEY',
      color: '#ff7043',
      icon: '🟠',
      source: 'Mistral AI',
    },
  ];

  return (
    <GlowPanel title="REVIEWER CONFIGURATION">
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))',
                    gap: 12 }}>
        {reviewers.map(r => (
          <div key={r.id} style={{
            padding: '12px 14px', borderRadius: 10,
            background: 'rgba(255,255,255,0.03)',
            border: `1px solid ${r.color}30`,
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
              <span style={{ fontSize: '1rem' }}>{r.icon}</span>
              <div>
                <div style={{ fontWeight: 700, color: r.color, fontSize: '0.82rem' }}>{r.name}</div>
                <div style={{ fontSize: '0.62rem', color: 'rgba(255,255,255,0.4)',
                              letterSpacing: '0.04em' }}>{r.source}</div>
              </div>
            </div>
            <div style={{ fontSize: '0.73rem', color: 'rgba(255,255,255,0.6)',
                          marginBottom: 6, lineHeight: 1.4 }}>
              {r.desc}
            </div>
            <div style={{ fontSize: '0.65rem', color: 'rgba(255,255,255,0.35)',
                          fontFamily: 'monospace',
                          background: 'rgba(0,0,0,0.3)', padding: '4px 8px',
                          borderRadius: 6 }}>
              env: {r.env}
            </div>
          </div>
        ))}
      </div>
      <div style={{ marginTop: 12, padding: '8px 12px', borderRadius: 8,
                    background: 'rgba(255,204,0,0.06)', border: '1px solid rgba(255,204,0,0.2)',
                    fontSize: '0.72rem', color: 'rgba(255,204,0,0.8)' }}>
        <strong>Consensus:</strong> Median score of all reviewers. Passes if ≥ 2/3 score ≥ 0.70.
        Fallback deterministic responses used when API keys are not configured.
      </div>
    </GlowPanel>
  );
}

// ── Main Page ──────────────────────────────────────────────────────────────

export default function PeerReviewPage() {
  const [reviews, setReviews] = useState(DEMO_REVIEWS);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState('all'); // 'all' | 'passed' | 'failed'

  const refresh = async () => {
    setLoading(true);
    try {
      const resp = await fetch('/api/peer-review/recent');
      if (resp.ok) {
        const data = await resp.json();
        if (data.reviews?.length) setReviews(data.reviews);
      }
    } catch (_) { /* use demo data */ }
    finally { setLoading(false); }
  };

  const filtered = reviews.filter(r => {
    if (filter === 'passed') return r.consensus_passed;
    if (filter === 'failed') return !r.consensus_passed;
    return true;
  });

  return (
    <div>
      <div className="page-header" style={{ display: 'flex', alignItems: 'center',
                                            justifyContent: 'space-between', gap: 12,
                                            flexWrap: 'wrap' }}>
        <div>
          <h2 style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <Brain size={20} color="var(--accent-primary)" />
            AUTOMATED PEER REVIEW <span style={{ fontSize: '0.6em', opacity: 0.5,
                                                  fontWeight: 400 }}>v10</span>
          </h2>
          <p style={{ fontSize: '0.75rem', color: 'rgba(255,255,255,0.45)', marginTop: 4 }}>
            Multi-LLM peer review: Gwen OSS · Google DeepThink · Mistral Large
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          {/* Filter tabs */}
          {['all', 'passed', 'failed'].map(f => (
            <button key={f}
              onClick={() => setFilter(f)}
              style={{
                padding: '6px 14px', borderRadius: 20, cursor: 'pointer',
                border: '1px solid',
                borderColor: filter === f
                  ? (f === 'passed' ? '#00ff88' : f === 'failed' ? '#ff4444' : 'var(--accent-primary)')
                  : 'rgba(255,255,255,0.12)',
                background: filter === f ? 'rgba(0,212,170,0.1)' : 'transparent',
                color: filter === f ? 'var(--accent-primary)' : 'rgba(255,255,255,0.5)',
                fontSize: '0.72rem', fontWeight: 700, letterSpacing: '0.04em',
                textTransform: 'uppercase',
              }}
            >
              {f}
            </button>
          ))}
          <button
            onClick={refresh}
            disabled={loading}
            className="btn btn-primary"
            style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: '0.75rem' }}
          >
            <RefreshCw size={13} style={{ animation: loading ? 'spin 1s linear infinite' : 'none' }} />
            REFRESH
          </button>
        </div>
      </div>

      {/* Stats bar */}
      <StatsBar reviews={reviews} />

      {/* Reviewer config legend */}
      <ReviewerLegend />

      {/* Review cards */}
      <div style={{ marginTop: 20, display: 'flex', flexDirection: 'column', gap: 16 }}>
        {filtered.length === 0 && (
          <div style={{ textAlign: 'center', padding: 60, color: 'rgba(255,255,255,0.3)',
                        fontSize: '0.85rem' }}>
            No reviews match the current filter.
          </div>
        )}
        {filtered.map((review, i) => (
          <ReviewCard key={review.id} review={review} index={i} />
        ))}
      </div>

      {/* v10 pipeline note */}
      <div style={{ marginTop: 24, padding: '14px 18px', borderRadius: 12,
                    background: 'rgba(66,133,244,0.06)',
                    border: '1px solid rgba(66,133,244,0.18)',
                    fontSize: '0.75rem', color: 'rgba(255,255,255,0.55)', lineHeight: 1.6 }}>
        <strong style={{ color: '#4285f4' }}>v10 Auto-Research Loop:</strong>&nbsp;
        Every hypothesis that passes the 5-gate physics validator and SUNDIALS simulation is
        automatically submitted to all three peer reviewers in sequence. Consensus (median ≥ 0.70,
        ≥ 2/3 passed) is required before a discovery is published and uploaded to GCS.
        Lean 4 certificates are displayed alongside each review for machine-verifiable provenance.
      </div>
    </div>
  );
}
