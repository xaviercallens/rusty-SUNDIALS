import { useState } from 'react';
import GlowPanel from '../components/GlowPanel';
import { Share2, Image as ImageIcon, Video, BookOpen, Download } from 'lucide-react';

export default function EducationPage() {
  const [generating, setGenerating] = useState(false);
  const [activeTab, setActiveTab] = useState('infographics');

  const handleGenerate = () => {
    setGenerating(true);
    setTimeout(() => setGenerating(false), 2000);
  };

  const infographics = [
    { title: "Breaking the Photosynthesis Limit", desc: "CQD upconversion shifts UV/IR to PAR light, pushing CCU past the thermodynamic 11.4% barrier.", color: "var(--cyan)", score: "18.2% Eff" },
    { title: "24/7 Dark Fixation", desc: "Electro-bionic DET powers the Calvin cycle through the night using 1.5V off-peak electricity.", color: "var(--green)", score: "95% Boost" },
    { title: "Zero-Shear Acoustofluidics", desc: "2.4 MHz ultrasonic standing waves auto-flocculate algae and maximize mass transfer.", color: "var(--magenta)", score: "99.1% Harvest" },
    { title: "O2 Vacuuming with PFD", desc: "A fluorocarbon liquid drops through the reactor, absorbing oxygen and suppressing photorespiration.", color: "var(--blue)", score: "66% Net Boost" }
  ];

  return (
    <div>
      <div className="page-header">
        <h2>EDUCATION & OUTREACH</h2>
        <button className="btn btn-primary" onClick={handleGenerate} disabled={generating}>
          {generating ? 'GENERATING MEDIA KIT...' : 'GENERATE MEDIA KIT'}
        </button>
      </div>

      <div style={{ display: 'flex', gap: 'var(--gap-md)', marginBottom: 'var(--gap-lg)' }}>
        <button className={`btn ${activeTab === 'infographics' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setActiveTab('infographics')}>
          <ImageIcon size={14} /> Infographics
        </button>
        <button className={`btn ${activeTab === 'animations' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setActiveTab('animations')}>
          <Video size={14} /> Animations
        </button>
        <button className={`btn ${activeTab === 'articles' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setActiveTab('articles')}>
          <BookOpen size={14} /> Public Articles
        </button>
      </div>

      {activeTab === 'infographics' && (
        <div className="grid-2 animate-in">
          {infographics.map((info, idx) => (
            <GlowPanel key={idx} title={`POST ${idx + 1}: ${info.title.toUpperCase()}`} style={{ borderTop: `2px solid ${info.color}` }}>
              <div style={{ padding: 'var(--gap-md) 0' }}>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', lineHeight: 1.5, marginBottom: 'var(--gap-md)' }}>
                  {info.desc}
                </p>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ color: info.color, fontWeight: 'bold', fontSize: '1.2rem' }}>
                    {info.score}
                  </div>
                  <div style={{ display: 'flex', gap: 8 }}>
                    <button className="btn btn-outline" title="Share"><Share2 size={12} /></button>
                    <button className="btn btn-outline" title="Download HD"><Download size={12} /></button>
                  </div>
                </div>
              </div>
            </GlowPanel>
          ))}
        </div>
      )}

      {activeTab === 'animations' && (
        <GlowPanel title="GENERATIVE REACTOR ANIMATIONS" className="animate-in">
          <p style={{ color: 'var(--text-secondary)', marginBottom: 'var(--gap-md)' }}>
            High-fidelity Manim / Blender visual outputs demonstrating the HamiltonianGraphAttentionIntegrator flow fields.
          </p>
          <div style={{ 
            background: '#0a0f1a', 
            height: 300, 
            borderRadius: 8, 
            border: '1px solid var(--border-dim)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexDirection: 'column'
          }}>
            <Video size={48} style={{ color: 'var(--cyan)', opacity: 0.5, marginBottom: 16 }} />
            <span style={{ color: 'var(--text-tertiary)', fontSize: '0.8rem' }}>xMHD Flow Field (Waiting for Render)</span>
          </div>
        </GlowPanel>
      )}

      {activeTab === 'articles' && (
        <GlowPanel title="PRESS RELEASES & BLOGS" className="animate-in">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--gap-md)' }}>
            <div style={{ padding: '16px', background: 'rgba(255,255,255,0.02)', borderRadius: 8, border: '1px solid var(--border-dim)' }}>
              <h4 style={{ color: 'var(--text-primary)', margin: '0 0 8px 0' }}>"How AI is Breaking the Photosynthesis Limit"</h4>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', margin: 0 }}>Auto-generated Medium article explaining Protocol K's quantum dot upconversion to a general audience. Estimated read time: 4 mins.</p>
              <button className="btn btn-outline" style={{ marginTop: 12 }}><Share2 size={12} /> Publish to Medium</button>
            </div>
            <div style={{ padding: '16px', background: 'rgba(255,255,255,0.02)', borderRadius: 8, border: '1px solid var(--border-dim)' }}>
              <h4 style={{ color: 'var(--text-primary)', margin: '0 0 8px 0' }}>"SymbioticFactory Shatters Classical CCU Ceilings"</h4>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', margin: 0 }}>Press release targeting climate-tech journalists. Focuses on the 14,000 tons/km² yield breakthrough.</p>
              <button className="btn btn-outline" style={{ marginTop: 12 }}><Share2 size={12} /> Distribute to Wire</button>
            </div>
          </div>
        </GlowPanel>
      )}
    </div>
  );
}
