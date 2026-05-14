import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard, GitBranch, Lightbulb, ShieldCheck,
  FlaskConical, FileText, DollarSign, Settings, BookOpen, Trophy, CheckSquare
} from 'lucide-react';

const navItems = [
  { to: '/', icon: LayoutDashboard, label: 'Mission' },
  { to: '/pipeline', icon: GitBranch, label: 'Pipeline' },
  { to: '/discoveries', icon: Lightbulb, label: 'Discover' },
  { to: '/verification', icon: ShieldCheck, label: 'Verify' },
  { to: '/physics', icon: FlaskConical, label: 'Physics' },
  { to: '/publications', icon: FileText, label: 'Publish' },
  { to: '/docs', icon: BookOpen, label: 'Docs' },
  { to: '/leaderboard', icon: Trophy, label: 'Leaderboard' },
  { to: '/education', icon: Lightbulb, label: 'Media' },
  { to: '/sop', icon: CheckSquare, label: 'SOP Reproduce' },
  { to: '/cost', icon: DollarSign, label: 'Cost' },
  { to: '/settings', icon: Settings, label: 'Config' },
];

export default function Sidebar() {
  return (
    <aside className="sidebar">
      <div className="sidebar-logo">
        <div className="logo-ring">RS</div>
      </div>
      <nav className="sidebar-nav">
        {navItems.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
          >
            <Icon />
            <span className="nav-label">{label}</span>
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
