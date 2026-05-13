import { useState, useEffect } from 'react';
import GlowPanel from '../components/GlowPanel';
import { PieChart, Pie, Cell, ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from 'recharts';
import api from '../api/client';

const BREAKDOWN = [
  { name: 'Cloud Run (compute)', value: 0.008, color: '#00e5ff' },
  { name: 'Cloud Build', value: 0.50, color: '#ffb800' },
  { name: 'Artifact Registry', value: 0.10, color: '#a78bfa' },
  { name: 'GCS Storage', value: 0.01, color: '#00ff88' },
  { name: 'Networking', value: 0.002, color: '#3b82f6' },
];

const EXPERIMENTS = [
  { name: 'Bio-Vortex P1', cpu_sec: 4.2, cost: 0.00028, date: 'May 12' },
  { name: 'Bio-Vortex P2', cpu_sec: 8.7, cost: 0.00058, date: 'May 12' },
  { name: 'Oxidize P1', cpu_sec: 189.5, cost: 0.0063, date: 'May 13' },
  { name: 'Oxidize P2', cpu_sec: 0.1, cost: 0.000007, date: 'May 13' },
  { name: 'Oxidize P3', cpu_sec: 22.3, cost: 0.0015, date: 'May 13' },
  { name: 'Verification', cpu_sec: 0.05, cost: 0.000003, date: 'May 13' },
  { name: 'Report Gen', cpu_sec: 0.02, cost: 0.000001, date: 'May 13' },
];

export default function CostPage() {
  const total = BREAKDOWN.reduce((s, b) => s + b.value, 0);
  const computeTotal = EXPERIMENTS.reduce((s, e) => s + e.cost, 0);
  const cpuTotal = EXPERIMENTS.reduce((s, e) => s + e.cpu_sec, 0);
  const budget = 100;

  return (
    <div>
      <div className="page-header"><h2>COST MONITOR</h2></div>
      <div className="grid-4" style={{ marginBottom: 'var(--gap-lg)' }}>
        <div className="metric-card animate-in">
          <span className="metric-label">Total Spend</span>
          <span className="metric-value">${total.toFixed(2)}</span>
          <span className="metric-delta positive">{((total/budget)*100).toFixed(2)}% of budget</span>
        </div>
        <div className="metric-card animate-in">
          <span className="metric-label">Budget Remaining</span>
          <span className="metric-value" style={{ color: 'var(--green)' }}>${(budget-total).toFixed(2)}</span>
          <span className="metric-delta positive">{((1 - total/budget)*100).toFixed(1)}% left</span>
        </div>
        <div className="metric-card animate-in">
          <span className="metric-label">Compute Cost</span>
          <span className="metric-value" style={{ color: 'var(--cyan)' }}>${computeTotal.toFixed(4)}</span>
          <span className="metric-delta positive">{cpuTotal.toFixed(1)}s CPU time</span>
        </div>
        <div className="metric-card animate-in">
          <span className="metric-label">Cost/Discovery</span>
          <span className="metric-value">${(total / EXPERIMENTS.length).toFixed(3)}</span>
          <span className="metric-delta positive">{EXPERIMENTS.length} experiments</span>
        </div>
      </div>

      <div className="grid-2x2">
        <GlowPanel title="COST BREAKDOWN" className="animate-in">
          <ResponsiveContainer width="100%" height={220}>
            <PieChart>
              <Pie data={BREAKDOWN} dataKey="value" nameKey="name" cx="50%" cy="50%" innerRadius={50} outerRadius={80} paddingAngle={3}>
                {BREAKDOWN.map((b, i) => <Cell key={i} fill={b.color} />)}
              </Pie>
              <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8, fontFamily: 'JetBrains Mono', fontSize: 12 }} formatter={v => `$${v.toFixed(4)}`} />
              <Legend wrapperStyle={{ fontFamily: 'JetBrains Mono', fontSize: 11 }} />
            </PieChart>
          </ResponsiveContainer>
        </GlowPanel>
        <GlowPanel title="COMPUTE COST PER EXPERIMENT" className="animate-in">
          <ResponsiveContainer width="100%" height={220}>
            <BarChart data={EXPERIMENTS}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1a2744" />
              <XAxis dataKey="name" tick={{ fill: '#7a8ba8', fontSize: 8, fontFamily: 'JetBrains Mono' }} angle={-25} textAnchor="end" height={50} />
              <YAxis tick={{ fill: '#7a8ba8', fontSize: 10 }} tickFormatter={v => `$${v}`} />
              <Tooltip contentStyle={{ background: '#0d1525', border: '1px solid #253654', borderRadius: 8, fontFamily: 'JetBrains Mono', fontSize: 11 }}
                       formatter={(v, name) => name === 'cost' ? `$${v.toFixed(6)}` : `${v.toFixed(1)}s`} />
              <Bar dataKey="cost" fill="#00e5ff" radius={[4,4,0,0]} name="cost" />
            </BarChart>
          </ResponsiveContainer>
        </GlowPanel>
      </div>

      {/* Experiment details */}
      <GlowPanel title="EXPERIMENT COST LOG" style={{ marginTop: 'var(--gap-lg)' }} className="animate-in">
        <table className="data-table">
          <thead>
            <tr><th>Experiment</th><th>CPU Time</th><th>Compute Cost</th><th>Date</th></tr>
          </thead>
          <tbody>
            {EXPERIMENTS.map((e, i) => (
              <tr key={i}>
                <td style={{ color: 'var(--text-primary)' }}>{e.name}</td>
                <td>{e.cpu_sec.toFixed(1)}s</td>
                <td style={{ color: 'var(--green)' }}>${e.cost < 0.001 ? e.cost.toExponential(2) : e.cost.toFixed(4)}</td>
                <td style={{ color: 'var(--text-secondary)' }}>{e.date}</td>
              </tr>
            ))}
            <tr style={{ borderTop: '2px solid var(--cyan)' }}>
              <td style={{ color: 'var(--cyan)', fontWeight: 'bold' }}>TOTAL</td>
              <td style={{ fontWeight: 'bold' }}>{cpuTotal.toFixed(1)}s</td>
              <td style={{ color: 'var(--green)', fontWeight: 'bold' }}>${computeTotal.toFixed(4)}</td>
              <td></td>
            </tr>
          </tbody>
        </table>
      </GlowPanel>
    </div>
  );
}
