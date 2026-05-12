import { Activity, Cpu, Database, Globe } from 'lucide-react';

export default function Header({ cost = 4.27, budget = 100 }) {
  const pct = (cost / budget) * 100;
  const level = pct > 90 ? 'danger' : pct > 70 ? 'warning' : '';

  return (
    <header className="header">
      <div className="header-title">
        <h1>RUSTY-SUNDIALS V6</h1>
        <span className="version">MISSION CONTROL</span>
      </div>

      <div className="header-status">
        <div className="status-dots">
          <StatusDot icon={Globe} label="API" status="online" />
          <StatusDot icon={Cpu} label="GPU" status="online" />
          <StatusDot icon={Activity} label="LEAN" status="online" />
          <StatusDot icon={Database} label="GCS" status="online" />
        </div>

        <div className={`cost-ticker ${level}`}>
          <span className="spent">${cost.toFixed(2)}</span>
          <span className="divider">/</span>
          <span className="budget">${budget.toFixed(2)}</span>
        </div>
      </div>
    </header>
  );
}

function StatusDot({ icon: Icon, label, status }) {
  return (
    <div className="status-dot">
      <div className={`dot ${status}`} />
      <span className="status-label">{label}</span>
    </div>
  );
}
