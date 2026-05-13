import { Activity, Cpu, Database, Globe, Shield } from 'lucide-react';
import { useAuth } from '../hooks/useAuth';

export default function Header({ cost = 4.27, budget = 100 }) {
  const { role, email, signIn, signOut } = useAuth();
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

        <div className={`badge ${role === 'admin' ? 'verified' : 'pending'}`}
             style={{ marginLeft: 12, fontSize: '0.6rem', padding: '2px 8px',
                      display: 'flex', alignItems: 'center', gap: 4, cursor: 'pointer' }}
             title={email || 'Click to sign in'}
             onClick={email ? signOut : signIn}>
          <Shield size={10} />
          {email ? (role === 'admin' ? '⚡ ADMIN' : '👁 GUEST') : '🔒 SIGN IN'}
        </div>
        {email && (
          <span style={{ fontSize: '0.55rem', color: 'var(--text-muted)', marginLeft: 6,
                         maxWidth: 140, overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {email}
          </span>
        )}
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
