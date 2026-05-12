import GlowPanel from '../components/GlowPanel';
import { PieChart, Pie, Cell, ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from 'recharts';

const BREAKDOWN = [
  { name: 'Gemini API', value: 0.68, color: '#00e5ff' },
  { name: 'Cloud Run', value: 0.12, color: '#a78bfa' },
  { name: 'Cloud Build', value: 0.50, color: '#ffb800' },
  { name: 'GCS Storage', value: 0.01, color: '#00ff88' },
  { name: 'Artifact Registry', value: 0.10, color: '#3b82f6' },
];

const HISTORY = [
  { date: 'May 12 19:00', cost: 0.14 },
  { date: 'May 12 19:24', cost: 0.28 },
  { date: 'May 12 19:45', cost: 0.14 },
  { date: 'May 12 19:48', cost: 0.14 },
  { date: 'May 12 20:06', cost: 0.14 },
  { date: 'May 12 20:06', cost: 0.14 },
];

export default function CostPage() {
  const total = BREAKDOWN.reduce((s, b) => s + b.value, 0);
  const budget = 100;

  return (
    <div>
      <div className="page-header"><h2>COST MONITOR</h2></div>
      <div className="grid-3" style={{ marginBottom: 'var(--gap-lg)' }}>
        <div className="metric-card"><span className="metric-label">Total Spend</span><span className="metric-value">${total.toFixed(2)}</span><span className="metric-delta positive">{((total/budget)*100).toFixed(1)}% of budget</span></div>
        <div className="metric-card"><span className="metric-label">Budget</span><span className="metric-value">${budget}</span><span className="metric-delta positive">${(budget-total).toFixed(2)} remaining</span></div>
        <div className="metric-card"><span className="metric-label">Cost/Discovery</span><span className="metric-value">${(total/6).toFixed(3)}</span><span className="metric-delta positive">6 discoveries</span></div>
      </div>
      <div className="grid-2x2">
        <GlowPanel title="COST BREAKDOWN">
          <ResponsiveContainer width="100%" height={220}>
            <PieChart>
              <Pie data={BREAKDOWN} dataKey="value" nameKey="name" cx="50%" cy="50%" innerRadius={50} outerRadius={80} paddingAngle={3}>
                {BREAKDOWN.map((b, i) => <Cell key={i} fill={b.color} />)}
              </Pie>
              <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8, fontFamily: 'JetBrains Mono', fontSize: 12 }} formatter={v => `$${v.toFixed(2)}`} />
              <Legend wrapperStyle={{ fontFamily: 'JetBrains Mono', fontSize: 11 }} />
            </PieChart>
          </ResponsiveContainer>
        </GlowPanel>
        <GlowPanel title="COST HISTORY">
          <ResponsiveContainer width="100%" height={220}>
            <BarChart data={HISTORY}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1a2744" />
              <XAxis dataKey="date" tick={{ fill: '#7a8ba8', fontSize: 9, fontFamily: 'JetBrains Mono' }} />
              <YAxis tick={{ fill: '#7a8ba8', fontSize: 10 }} tickFormatter={v => `$${v}`} />
              <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8 }} />
              <Bar dataKey="cost" fill="#00e5ff" radius={[4,4,0,0]} />
            </BarChart>
          </ResponsiveContainer>
        </GlowPanel>
      </div>
    </div>
  );
}
