import { useState } from 'react';
import GlowPanel from '../components/GlowPanel';
import { Share2, Image as ImageIcon, Video, BookOpen, Download } from 'lucide-react';

export default function EducationPage() {
  const [generating, setGenerating] = useState(false);
  const [activeTab, setActiveTab] = useState('infographics');

  const handleDownload = () => {
    alert("Downloading Full Media Kit (High-Res Images, Videos, press release .zip)...");
  };

  const infographics = [
    { 
      title: "Protocol K: Breaking the Photosynthesis Limit", 
      desc: "CQD upconversion shifts UV/IR to PAR light, pushing CCU past the thermodynamic 11.4% barrier.", 
      longDesc: "Through mathematical optimization of Radiative Transfer Equations, the agent discovered that Carbon Quantum Dots (CQDs) act as a secondary 'antenna' system. By absorbing wasted high-energy UV and low-energy IR photons, they re-emit them in the Photosynthetically Active Radiation (PAR) spectrum. This tricks the RuBisCO enzyme into continuously firing, resulting in an unprecedented 18.2% equivalent photosynthetic efficiency—the absolute upper bound before thermal denaturation occurs.",
      image: "/protocol_k.png",
      color: "var(--cyan)", score: "18.2% Eff" 
    },
    { 
      title: "Protocol M: Zero-Shear Acoustofluidics", 
      desc: "2.4 MHz ultrasonic standing waves auto-flocculate algae and maximize mass transfer.", 
      longDesc: "A core problem in industrial scaling is that mechanical pumps shear and lyse the delicate algal cells. The agent hypothesized using Acoustic Radiation Forces. By injecting 2.4 MHz standing waves, it perfectly suspends the micro-bubbles and algal cells in the center of the flow stream. This creates zero-shear mass transfer boundaries, scaling the kLa coefficient to an astounding 310 /h while preventing the cells from touching the pipe walls.",
      image: "/protocol_m.png",
      color: "var(--magenta)", score: "99.1% Harvest" 
    },
    { 
      title: "Protocol L: 24/7 Dark Fixation", 
      desc: "Electro-bionic DET powers the Calvin cycle through the night using 1.5V off-peak electricity.", 
      longDesc: "Plants normally shut down at night. By simulating Direct Electron Transfer (DET) via a 1.5V cathode mesh, the agent bypassed the light-dependent reactions entirely. Electrons are pumped directly into the plastoquinone pool, maintaining ATP/NADPH production 24/7. This effectively doubles the daily CO2 absorption capacity of the biorefinery without requiring sunlight.",
      color: "var(--green)", score: "95% Boost" 
    },
    { 
      title: "Protocol N: O2 Vacuuming with PFD", 
      desc: "A fluorocarbon liquid drops through the reactor, absorbing oxygen and suppressing photorespiration.", 
      longDesc: "Oxygen buildup poisons the RuBisCO enzyme. By modeling the Multiphase Cahn-Hilliard equations, the agent introduced Perfluorodecalin (PFD) droplets that sink through the reactor like an 'oxygen vacuum'. This keeps O2 levels below 4.1 mg/L, suppressing the wasteful photorespiration cycle and locking RuBisCO exclusively into carbon fixation mode.",
      color: "var(--blue)", score: "66% Net Boost" 
    }
  ];

  const pscDiscoveries = [
    { title: "Module SUN: Plasmonic Desalination", desc: "Passive thermal desalination using Silver (Ag) doped PAC sponges. Halves the latent heat requirement.", longDesc: "By tuning the Localized Surface Plasmon Resonance (LSPR), the system achieves an Enthalpy of Vaporization of 1320.92 kJ/kg, decoupling freshwater supply from the fossil grid without causing ZLD scaling.", color: "var(--cyan)", score: "1320.92 kJ/kg" },
    { title: "Module WATER: Photonic & Acoustofluidic Bioreactor", desc: "Combines zero-shear standing waves with pulsed light for extreme algae harvesting.", longDesc: "Using rusty-SUNDIALS to optimize the Navier-Stokes boundary layers, the agent maximized mass transfer (kLa) to 115.10 /s. Simultaneously, a Photonic PWM locked at 0.1 Hz / 10% duty cycle yielded a +550% efficiency boost in carbon fixation.", color: "var(--blue)", score: "+550% Eff" },
    { title: "Module TERRE: Anaerobic Pyrolysis", desc: "Thermochemically stabilizes spent waste into millennial-scale Biochar, maintaining an O:C ratio of 0.050.", longDesc: "Using the IDA Radau DAE solver, the agent searched the latent thermal gradient. It locked onto a setpoint that creates a crystalline graphene-like matrix that won't degrade for thousands of years, while extracting combustible syngas.", color: "var(--amber)", score: "110.5 Syngas" },
    { title: "Module FIRE: Hydrothermal Liquefaction", desc: "Depolymerizes wet algal biomass into Bio-crude without drying. Energy density of 37.15 MJ/kg achieved.", longDesc: "Operating near the critical point of water, the AI swept the subcritical reactor pressure and temperature latent space. The closed-loop fermentation process yields a direct fossil-fuel equivalent density with an Energy Return on Investment (EROI) > 6.", color: "var(--magenta)", score: "6.13 EROI" },
    { title: "Planetary Geo-Optimization", desc: "NASA POWER data confirms Namib Coastal Edge as the ultimate planetary node. Net Neutrality achievable in 0.6 years.", longDesc: "Integrating CERES/MERRA-2 global atmospheric datasets with the rusty-SUNDIALS Parareal PinT solver, the agent scanned Earth's topography. Deploying arrays on the Namib coast eliminates pumping penalties and ecological disruption, drawing down 15,492 Mt of CO2.", color: "var(--green)", score: "0.6 Yrs to Neutral" }
  ];

  const fusionDiscoveries = [
    { title: "Protocol F: Tensor-Train Integration", desc: "Solves the 6D 'curse of dimensionality'. Newton integration on Tensor-Train arrays compresses 14.8 TB down to 46 MB. Exascale run locally in 14.2s.", color: "var(--cyan)", score: "320,000x Shrink" },
    { title: "Protocol G: Adjoint Billiard d-SPI", desc: "Adjoint differentiation discovered an 800m/s Argon + 1.2ms Neon billiard pulse. Drops thermal quench heat flux from 84.2 to 11.4 MW/m².", color: "var(--amber)", score: "98.5% Radiated" },
    { title: "Protocol H: Neural Phase-Field Walls", desc: "Simulates liquid Tin/Lithium walls. Triggers an active capillary counter-wave to perfectly neutralize ELM kinetic shock without splashing.", color: "var(--blue)", score: "Zero Splash" },
    { title: "Protocol I: HDC Boolean Control", desc: "Abandons standard PDEs for a 10,000-D boolean space (XOR on FPGA). Drops sub-millisecond magnetic control latency to just 40 nanoseconds.", color: "var(--magenta)", score: "1,375x Faster" }
  ];

  return (
    <div>
      <div className="page-header">
        <h2>EDUCATION & OUTREACH</h2>
        <button className="btn btn-primary" onClick={handleDownload}>
          <Download size={14} style={{ marginRight: 8 }} /> DOWNLOAD FULL MEDIA KIT (.ZIP)
        </button>
      </div>

      <div style={{ display: 'flex', gap: 'var(--gap-md)', marginBottom: 'var(--gap-lg)', flexWrap: 'wrap' }}>
        <button className={`btn ${activeTab === 'infographics' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setActiveTab('infographics')}>
          <ImageIcon size={14} /> Infographics
        </button>
        <button className={`btn ${activeTab === 'symbiotic' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setActiveTab('symbiotic')}>
          <Share2 size={14} /> Planet Symbiotic Cycle
        </button>
        <button className={`btn ${activeTab === 'iter' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setActiveTab('iter')}>
          <Video size={14} /> ITER Fusion Master
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
                {info.image && (
                  <div style={{ width: '100%', borderRadius: 8, overflow: 'hidden', border: '1px solid var(--border-dim)', marginBottom: 'var(--gap-md)' }}>
                    <img src={info.image} alt={info.title} style={{ width: '100%', height: 'auto', display: 'block' }} />
                  </div>
                )}
                <p style={{ color: 'var(--text-primary)', fontSize: '0.9rem', fontWeight: 'bold', marginBottom: '8px' }}>
                  {info.desc}
                </p>
                <div style={{ padding: '12px', background: 'rgba(255,255,255,0.03)', borderRadius: '6px', borderLeft: `3px solid ${info.color}`, marginBottom: '16px' }}>
                  <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', lineHeight: 1.6, margin: 0 }}>
                    {info.longDesc}
                  </p>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ color: info.color, fontWeight: 'bold', fontSize: '1.2rem' }}>
                    {info.score}
                  </div>
                  <div style={{ display: 'flex', gap: 8 }}>
                    <button className="btn btn-outline" title="Share (Unavailable)" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Share2 size={12} /></button>
                    <button className="btn btn-outline" title="Download HD (Unavailable)" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Download size={12} /></button>
                  </div>
                </div>
              </div>
            </GlowPanel>
          ))}
        </div>
      )}

      {activeTab === 'iter' && (
        <div className="animate-in" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--gap-lg)' }}>
          <GlowPanel title="ITER FUSION: DISRUPTIVE PLASMA CONTAINMENT">
            <div style={{ width: '100%', borderRadius: 8, overflow: 'hidden', border: '1px solid var(--border-dim)', marginBottom: 'var(--gap-md)' }}>
              <img src="/iter_fusion_hero.png" alt="ITER Fusion Plasma Core" style={{ width: '100%', height: 'auto', display: 'block' }} />
            </div>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: 1.6 }}>
              The Phase III Autoresearch pipeline bypassed incremental stabilization, generating four horizon computational paradigms targeting ITER's absolute physical limitations: the 6D kinetic curse, thermal quenches, boundary failures, and nanosecond latency.
            </p>
          </GlowPanel>

          <div className="grid-2">
            {fusionDiscoveries.map((info, idx) => (
              <GlowPanel key={idx} title={info.title.toUpperCase()} style={{ borderTop: `2px solid ${info.color}` }}>
                <div style={{ padding: 'var(--gap-md) 0' }}>
                  <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', lineHeight: 1.5, marginBottom: 'var(--gap-md)' }}>
                    {info.desc}
                  </p>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ color: info.color, fontWeight: 'bold', fontSize: '1.2rem' }}>
                      {info.score}
                    </div>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <button className="btn btn-outline" title="Share (Unavailable)" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Share2 size={12} /></button>
                      <button className="btn btn-outline" title="Download HD (Unavailable)" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Download size={12} /></button>
                    </div>
                  </div>
                </div>
              </GlowPanel>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'symbiotic' && (
        <div className="animate-in" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--gap-lg)' }}>
          <GlowPanel title="KALUNDBORG 2.0: ECO-INDUSTRIAL SYMBIOSIS">
            <div style={{ width: '100%', borderRadius: 8, overflow: 'hidden', border: '1px solid var(--border-dim)', marginBottom: 'var(--gap-md)' }}>
              <img src="/kalundborg_eip_infographic_1778767682308.png" alt="Kalundborg 2.0 Architecture" style={{ width: '100%', height: 'auto', display: 'block' }} />
            </div>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: 1.6 }}>
              The Kalundborg 2.0 model orchestrates a 96% Circularity Index. Waste heat (50.3% TERRE, 37.5% FIRE, 12.2% SUN) is scavenged from an industrial anchor to completely power the SymbioticFactory. Outputs such as fresh water and Biochar are pumped to an agricultural hub, sustaining 99.3% drought resistance while maintaining a strict O:C ratio of 0.198 for permanent sequestration.
            </p>
          </GlowPanel>

          <GlowPanel title="PLANET SYMBIOTIC CYCLE: EARTH DIGITAL TWIN">
            <div style={{ width: '100%', borderRadius: 8, overflow: 'hidden', border: '1px solid var(--border-dim)', marginBottom: 'var(--gap-md)' }}>
              <img src="/planetary_geo_nodes_1778766103411.png" alt="Planetary Geo Nodes Map" style={{ width: '100%', height: 'auto', display: 'block' }} />
            </div>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: 1.6 }}>
              The Planet Symbiotic Cycle eliminates external energy dependencies by treating light, water, biomass, and heat as a continuous dynamic fluid. Using rusty-SUNDIALS to simulate global deployments over 25 years, the AI successfully orchestrated a fully off-grid carbon capture pipeline.
            </p>
          </GlowPanel>

          <div className="grid-2">
            {pscDiscoveries.map((info, idx) => (
              <GlowPanel key={idx} title={info.title.toUpperCase()} style={{ borderTop: `2px solid ${info.color}` }}>
                <div style={{ padding: 'var(--gap-md) 0' }}>
                  <p style={{ color: 'var(--text-primary)', fontSize: '0.9rem', fontWeight: 'bold', marginBottom: '8px' }}>
                    {info.desc}
                  </p>
                  <div style={{ padding: '12px', background: 'rgba(255,255,255,0.03)', borderRadius: '6px', borderLeft: `3px solid ${info.color}`, marginBottom: '16px' }}>
                    <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', lineHeight: 1.6, margin: 0 }}>
                      {info.longDesc}
                    </p>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ color: info.color, fontWeight: 'bold', fontSize: '1.2rem' }}>
                      {info.score}
                    </div>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <button className="btn btn-outline" title="Share (Unavailable)" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Share2 size={12} /></button>
                      <button className="btn btn-outline" title="Download HD (Unavailable)" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Download size={12} /></button>
                    </div>
                  </div>
                </div>
              </GlowPanel>
            ))}
          </div>
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
              <button className="btn btn-outline" disabled style={{ marginTop: 12, opacity: 0.5, cursor: 'not-allowed' }}><Share2 size={12} /> Publish to Medium (Unavailable)</button>
            </div>
            <div style={{ padding: '16px', background: 'rgba(255,255,255,0.02)', borderRadius: 8, border: '1px solid var(--border-dim)' }}>
              <h4 style={{ color: 'var(--text-primary)', margin: '0 0 8px 0' }}>"SymbioticFactory Shatters Classical CCU Ceilings"</h4>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', margin: 0 }}>Press release targeting climate-tech journalists. Focuses on the 14,000 tons/km² yield breakthrough.</p>
              <button className="btn btn-outline" disabled style={{ marginTop: 12, opacity: 0.5, cursor: 'not-allowed' }}><Share2 size={12} /> Distribute to Wire (Unavailable)</button>
            </div>
          </div>
        </GlowPanel>
      )}
    </div>
  );
}
