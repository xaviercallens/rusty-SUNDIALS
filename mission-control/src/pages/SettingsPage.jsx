import GlowPanel from '../components/GlowPanel';
import { Key, Globe, Bell, Save } from 'lucide-react';

export default function SettingsPage() {
  return (
    <div>
      <div className="page-header"><h2>CONFIGURATION</h2></div>
      <div className="grid-2x2">
        <GlowPanel title="API KEYS">
          <SettingRow label="Gemini API Key" value="AIza...●●●●●●●●" icon={<Key size={14}/>} />
          <SettingRow label="Anthropic Key" value="sk-ant...●●●●" icon={<Key size={14}/>} />
        </GlowPanel>
        <GlowPanel title="GCP PROJECT">
          <SettingRow label="Project ID" value="gen-lang-client-0625573011" icon={<Globe size={14}/>} />
          <SettingRow label="Region" value="europe-west1" icon={<Globe size={14}/>} />
          <SettingRow label="GCS Bucket" value="rusty-sundials-discoveries" icon={<Globe size={14}/>} />
        </GlowPanel>
        <GlowPanel title="BUDGET CONTROLS">
          <SettingRow label="Budget Limit" value="$100.00" icon={<Bell size={14}/>} />
          <SettingRow label="Alert at 50%" value="$50.00" icon={<Bell size={14}/>} />
          <SettingRow label="Kill Switch at" value="$95.00" icon={<Bell size={14}/>} />
        </GlowPanel>
        <GlowPanel title="NOTIFICATIONS">
          <SettingRow label="Email" value="callensxavier@gmail.com" icon={<Bell size={14}/>} />
          <SettingRow label="Slack Webhook" value="Not configured" icon={<Bell size={14}/>} />
        </GlowPanel>
      </div>
      <div style={{marginTop:'var(--gap-lg)'}}><button className="btn btn-primary" disabled style={{ opacity: 0.5, cursor: 'not-allowed' }}><Save size={14}/> SAVE CONFIGURATION (Unavailable)</button></div>
    </div>
  );
}

function SettingRow({ label, value, icon }) {
  return (
    <div style={{ display:'flex', justifyContent:'space-between', alignItems:'center', padding:'10px 0', borderBottom:'1px solid var(--border-dim)' }}>
      <div style={{ display:'flex', alignItems:'center', gap:8, color:'var(--text-secondary)' }}>{icon}<span className="label">{label}</span></div>
      <span className="data-value" style={{fontSize:'0.8rem'}}>{value}</span>
    </div>
  );
}
