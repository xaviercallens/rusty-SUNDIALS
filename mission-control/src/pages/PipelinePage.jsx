import GlowPanel from '../components/GlowPanel';
import PipelineGraph from '../components/PipelineGraph';
import { Sliders } from 'lucide-react';

const NODE_PARAMS = {
  hypothesize: { model: 'Gemini 2.5 Pro', temperature: 0.7, maxTokens: 8192 },
  physics: { invariants: ['∇·B = 0', 'Energy', 'Helicity'], threshold: 1e-10 },
  synthesize: { language: 'Rust', safetyLevel: 'strict', maxLines: 200 },
  verify: { prover: 'Lean 4', maxAttempts: 3, timeout: 60 },
  deploy: { gridSize: 128, resistivity: 1e-3, timeSpan: 0.1 },
  publish: { template: 'arXiv', style: 'AMS', autoSubmit: false },
};

export default function PipelinePage() {
  return (
    <div>
      <div className="page-header">
        <h2>PIPELINE BUILDER</h2>
        <button className="btn btn-outline"><Sliders size={14} /> SAVE TEMPLATE</button>
      </div>

      <GlowPanel title="STATE MACHINE TOPOLOGY" className="animate-in">
        <PipelineGraph activeNode="hypothesize" />
      </GlowPanel>

      <h3 style={{ margin: 'var(--gap-lg) 0 var(--gap-md)', color: 'var(--cyan)' }}>NODE CONFIGURATION</h3>

      <div className="grid-3">
        {Object.entries(NODE_PARAMS).map(([key, params]) => (
          <GlowPanel key={key} title={key.toUpperCase()}>
            {Object.entries(params).map(([k, v]) => (
              <div key={k} style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', borderBottom: '1px solid var(--border-dim)' }}>
                <span className="label" style={{ fontSize: '0.7rem' }}>{k}</span>
                <span className="data-value" style={{ fontSize: '0.8rem' }}>
                  {Array.isArray(v) ? v.join(', ') : String(v)}
                </span>
              </div>
            ))}
          </GlowPanel>
        ))}
      </div>
    </div>
  );
}
