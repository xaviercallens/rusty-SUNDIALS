import { useState } from 'react';

const NODES = [
  { id: 'hypothesize', label: 'Hypothesize', x: 60, y: 40 },
  { id: 'physics', label: 'Physics\nCheck', x: 220, y: 40 },
  { id: 'synthesize', label: 'Code\nSynthesize', x: 380, y: 40 },
  { id: 'verify', label: 'Lean\nVerify', x: 60, y: 170 },
  { id: 'deploy', label: 'Exascale\nDeploy', x: 220, y: 170 },
  { id: 'publish', label: 'Auto\nPublish', x: 380, y: 170 },
];

const EDGES = [
  ['hypothesize', 'physics'],
  ['physics', 'synthesize'],
  ['synthesize', 'verify'],
  ['verify', 'deploy'],
  ['deploy', 'publish'],
];

export default function PipelineGraph({ activeNode = 'synthesize' }) {
  const [hovered, setHovered] = useState(null);

  const getNodePos = (id) => NODES.find(n => n.id === id);

  return (
    <svg viewBox="0 0 480 260" style={{ width: '100%', height: '100%' }}>
      <defs>
        <filter id="glow">
          <feGaussianBlur stdDeviation="4" result="blur" />
          <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
        </filter>
        <filter id="glow-strong">
          <feGaussianBlur stdDeviation="8" result="blur" />
          <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
        </filter>
      </defs>

      {/* Edges */}
      {EDGES.map(([from, to]) => {
        const a = getNodePos(from), b = getNodePos(to);
        const passed = NODES.findIndex(n => n.id === activeNode) > NODES.findIndex(n => n.id === from);
        return (
          <line key={`${from}-${to}`}
            x1={a.x + 40} y1={a.y + 35} x2={b.x + 40} y2={b.y + 35}
            stroke={passed ? '#00ff88' : '#253654'}
            strokeWidth={passed ? 2 : 1}
            strokeDasharray={passed ? 'none' : '6 4'}
            filter={passed ? 'url(#glow)' : 'none'}
          />
        );
      })}

      {/* Nodes */}
      {NODES.map((node) => {
        const isActive = node.id === activeNode;
        const idx = NODES.findIndex(n => n.id === node.id);
        const activeIdx = NODES.findIndex(n => n.id === activeNode);
        const isComplete = idx < activeIdx;
        const isHovered = hovered === node.id;

        return (
          <g key={node.id}
            onMouseEnter={() => setHovered(node.id)}
            onMouseLeave={() => setHovered(null)}
            style={{ cursor: 'pointer' }}
          >
            {/* Hex shape */}
            <polygon
              points={hexPoints(node.x + 40, node.y + 35, isHovered ? 42 : 38)}
              fill={isActive ? 'rgba(0,229,255,0.12)' : isComplete ? 'rgba(0,255,136,0.08)' : '#131d2e'}
              stroke={isActive ? '#00e5ff' : isComplete ? '#00ff88' : '#253654'}
              strokeWidth={isActive ? 2 : 1}
              filter={isActive ? 'url(#glow-strong)' : 'none'}
            />
            {isActive && (
              <polygon
                points={hexPoints(node.x + 40, node.y + 35, 42)}
                fill="none"
                stroke="#00e5ff"
                strokeWidth={1}
                opacity={0.3}
              >
                <animate attributeName="opacity" values="0.3;0.6;0.3" dur="2s" repeatCount="indefinite" />
              </polygon>
            )}
            {/* Label */}
            {node.label.split('\n').map((line, i) => (
              <text key={i}
                x={node.x + 40} y={node.y + 32 + i * 14}
                textAnchor="middle"
                fill={isActive ? '#00e5ff' : isComplete ? '#00ff88' : '#7a8ba8'}
                fontSize="9"
                fontFamily="'JetBrains Mono', monospace"
                fontWeight={isActive ? 600 : 400}
              >
                {line}
              </text>
            ))}
            {/* Check mark for complete */}
            {isComplete && (
              <text x={node.x + 60} y={node.y + 12} fontSize="14" fill="#00ff88">✓</text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

function hexPoints(cx, cy, r) {
  return Array.from({ length: 6 }, (_, i) => {
    const angle = (Math.PI / 3) * i - Math.PI / 2;
    return `${cx + r * Math.cos(angle)},${cy + r * Math.sin(angle)}`;
  }).join(' ');
}
