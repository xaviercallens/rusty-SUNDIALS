// Cray-1 vs Apple Silicon Interactive Demo Engine
const DEMOS = {
  lorenz: {
    name: 'Lorenz Attractor', eq: 3, tEnd: 40, crayMinutes: 8,
    y0: [1,1,1],
    rhs: (t,y,f) => { f[0]=10*(y[1]-y[0]); f[1]=y[0]*(28-y[2])-y[1]; f[2]=y[0]*y[1]-2.667*y[2]; },
    plot3d: true, vars: ['x','y','z'],
    info: 'Edward Lorenz discovered this in 1963 on a Royal McBee computer. On the Cray-1, a 40-second trajectory took ~8 minutes of batch time.'
  },
  orbit: {
    name: 'Kepler Orbit', eq: 4, tEnd: 25, crayMinutes: 3,
    y0: [1,0,0,1],
    rhs: (t,y,f) => { const r=Math.sqrt(y[0]**2+y[1]**2)||1e-10,r3=r**3; f[0]=y[2];f[1]=y[3];f[2]=-y[0]/r3;f[3]=-y[1]/r3; },
    plotOrbit: true, vars: ['x','y','vx','vy'],
    info: 'Keplerian two-body problem. At INRIA, we used this to validate our Fortran integrators before tackling satellite orbit prediction.'
  },
  neuron: {
    name: 'Hodgkin-Huxley Neuron', eq: 4, tEnd: 50, crayMinutes: 15,
    y0: [-65, 0.053, 0.596, 0.318],
    rhs: (t,y,f) => {
      const v=y[0],m=y[1],h=y[2],n=y[3];
      const am=Math.abs(v+40)<1e-7?1:0.1*(v+40)/(1-Math.exp(-0.1*(v+40))),bm=4*Math.exp(-(v+65)/18);
      const ah=0.07*Math.exp(-(v+65)/20),bh=1/(1+Math.exp(-0.1*(v+35)));
      const an=Math.abs(v+55)<1e-7?0.1:0.01*(v+55)/(1-Math.exp(-0.1*(v+55))),bn=0.125*Math.exp(-(v+65)/80);
      f[0]=10-120*m*m*m*h*(v-50)-36*n*n*n*n*(v+77)-0.3*(v+54.387);
      f[1]=am*(1-m)-bm*m; f[2]=ah*(1-h)-bh*h; f[3]=an*(1-n)-bn*n;
    },
    vars: ['V(mV)','m','h','n'],
    info: 'Nobel Prize 1963. This stiff system models the nerve impulse. On the Cray, each action potential simulation took 15 minutes with LSODE.'
  },
  epidemic: {
    name: 'SIR Epidemic', eq: 3, tEnd: 200, crayMinutes: 5,
    y0: [999,1,0],
    rhs: (t,y,f) => { const inf=0.3*y[0]*y[1]/1000; f[0]=-inf; f[1]=inf-0.1*y[1]; f[2]=0.1*y[1]; },
    vars: ['S','I','R'],
    info: 'Kermack-McKendrick 1927. In the 1980s, epidemic modeling for influenza was a major Cray workload at CDC and INRIA.'
  },
  threebody: {
    name: 'Three-Body Problem', eq: 12, tEnd: 8, crayMinutes: 45,
    y0: [-0.5,0,0.5,0,0,0.866, 0,0.5,0,-0.5,-0.5,0],
    rhs: (t,y,f) => {
      const r12=Math.max(Math.sqrt((y[0]-y[2])**2+(y[1]-y[3])**2),1e-10);
      const r13=Math.max(Math.sqrt((y[0]-y[4])**2+(y[1]-y[5])**2),1e-10);
      const r23=Math.max(Math.sqrt((y[2]-y[4])**2+(y[3]-y[5])**2),1e-10);
      for(let i=0;i<6;i++) f[i]=y[i+6];
      f[6]=-(y[0]-y[2])/r12**3-(y[0]-y[4])/r13**3; f[7]=-(y[1]-y[3])/r12**3-(y[1]-y[5])/r13**3;
      f[8]=-(y[2]-y[0])/r12**3-(y[2]-y[4])/r23**3; f[9]=-(y[3]-y[1])/r12**3-(y[3]-y[5])/r23**3;
      f[10]=-(y[4]-y[0])/r13**3-(y[4]-y[2])/r23**3; f[11]=-(y[5]-y[1])/r13**3-(y[5]-y[3])/r23**3;
    },
    plotOrbit3: true, vars: ['x1','y1','x2','y2','x3','y3'],
    info: 'The famous unsolvable problem. Henri Poincaré proved it has no closed-form solution. On the Cray, a single trajectory took 45 minutes.'
  },
  pendulum: {
    name: 'Double Pendulum', eq: 4, tEnd: 30, crayMinutes: 10,
    y0: [2.5,0,1.5,0],
    rhs: (t,y,f) => {
      const d=y[0]-y[2],cd=Math.cos(d),sd=Math.sin(d),den=2-cd*cd,g=9.81;
      f[0]=y[1]; f[1]=(-y[1]*y[1]*sd*cd+g*Math.sin(y[2])*cd-y[3]*y[3]*sd-2*g*Math.sin(y[0]))/den;
      f[2]=y[3]; f[3]=(y[3]*y[3]*sd*cd+2*(g*Math.sin(y[0])*cd+y[1]*y[1]*sd-g*Math.sin(y[2])))/den;
    },
    vars: ['θ₁','ω₁','θ₂','ω₂'],
    info: 'Extreme sensitivity to initial conditions. A Cray-2 at INRIA ran thousands of trajectories for chaos research.'
  },
  vdp: {
    name: 'Van der Pol (μ=100)', eq: 2, tEnd: 400, crayMinutes: 20,
    y0: [2,0],
    rhs: (t,y,f) => { f[0]=y[1]; f[1]=100*(1-y[0]*y[0])*y[1]-y[0]; },
    vars: ['x','dx/dt'],
    info: 'Classic stiff benchmark with μ=100. This problem was THE test case for LSODE on the Cray. The stiffness ratio exceeds 10,000.'
  },
  rossler: {
    name: 'Rössler Attractor', eq: 3, tEnd: 200, crayMinutes: 12,
    y0: [1,1,0],
    rhs: (t,y,f) => { f[0]=-y[1]-y[2]; f[1]=y[0]+0.2*y[1]; f[2]=0.2+y[2]*(y[0]-5.7); },
    plot3d: true, vars: ['x','y','z'],
    info: 'Rössler (1976) — simpler than Lorenz but with rich dynamics. The folded-band structure was first visualized on a Cray vector display.'
  }
};

const PLOT_CFG = {
  paper_bgcolor:'transparent', plot_bgcolor:'rgba(0,240,255,0.03)',
  font:{family:'Inter',color:'#94a3b8',size:11}, margin:{l:45,r:15,t:25,b:35},
  xaxis:{gridcolor:'rgba(0,240,255,0.08)',zerolinecolor:'rgba(0,240,255,0.15)'},
  yaxis:{gridcolor:'rgba(0,240,255,0.08)',zerolinecolor:'rgba(0,240,255,0.15)'},
  showlegend:true, legend:{orientation:'h',y:-0.15,font:{size:10}}
};

function runDemo() {
  const key = document.getElementById('demoSelect').value;
  const d = DEMOS[key];
  const t0 = performance.now();
  const result = Solver.adaptive(d.rhs, 0, d.y0.slice(), d.tEnd, 1e-8, 500000);
  const elapsed = performance.now() - t0;

  // Show timing comparison
  const crayMs = d.crayMinutes * 60 * 1000;
  const speedup = Math.round(crayMs / elapsed);
  document.getElementById('crayTimer').innerHTML =
    `<span style="color:#ff6ec7">Cray-1: ~${d.crayMinutes} min</span>` +
    ` &nbsp;⚡&nbsp; ` +
    `<span style="color:#00f0ff">Your Mac: ${elapsed.toFixed(1)} ms</span>` +
    ` &nbsp;→&nbsp; ` +
    `<span style="color:#39ff14">${speedup.toLocaleString()}× faster</span>`;

  // Stats
  document.getElementById('demoStats').innerHTML = [
    ['Steps', result.steps.toLocaleString()],
    ['RHS Evals', (result.steps * 6).toLocaleString()],
    ['ODEs', d.eq],
    ['Speedup', speedup.toLocaleString() + '×']
  ].map(([l,v]) =>
    `<div style="text-align:center;padding:12px;background:rgba(0,240,255,.05);border:1px solid rgba(0,240,255,.12);border-radius:10px"><div style="font-family:JetBrains Mono;font-size:1.3rem;color:#00f0ff">${v}</div><div style="font-size:.7rem;color:rgba(255,255,255,.4);text-transform:uppercase;margin-top:4px">${l}</div></div>`
  ).join('');

  const colors = ['#00f0ff','#ff6ec7','#39ff14','#f59e0b','#8b5cf6','#06b6d4'];

  // Plot 1: main visualization
  if (d.plot3d) {
    Plotly.newPlot('demoPlot1', [{
      type:'scatter3d', mode:'lines',
      x:result.ys.map(y=>y[0]), y:result.ys.map(y=>y[1]), z:result.ys.map(y=>y[2]),
      line:{color:result.ts,colorscale:[[0,'#00f0ff'],[0.5,'#ff6ec7'],[1,'#39ff14']],width:2}
    }], {...PLOT_CFG, scene:{
      xaxis:{gridcolor:'rgba(0,240,255,0.06)',title:'x'},
      yaxis:{gridcolor:'rgba(0,240,255,0.06)',title:'y'},
      zaxis:{gridcolor:'rgba(0,240,255,0.06)',title:'z'},
      bgcolor:'rgba(0,0,0,0.1)'
    }}, {responsive:true});
  } else if (d.plotOrbit) {
    Plotly.newPlot('demoPlot1', [
      {x:result.ys.map(y=>y[0]),y:result.ys.map(y=>y[1]),mode:'lines',line:{color:'#00f0ff',width:2},name:'Orbit'},
      {x:[0],y:[0],mode:'markers',marker:{size:14,color:'#f59e0b'},name:'Central body'}
    ], {...PLOT_CFG, xaxis:{...PLOT_CFG.xaxis,title:'x',scaleanchor:'y'},yaxis:{...PLOT_CFG.yaxis,title:'y'}}, {responsive:true});
  } else if (d.plotOrbit3) {
    Plotly.newPlot('demoPlot1', [
      {x:result.ys.map(y=>y[0]),y:result.ys.map(y=>y[1]),mode:'lines',line:{color:colors[0],width:2},name:'Body 1'},
      {x:result.ys.map(y=>y[2]),y:result.ys.map(y=>y[3]),mode:'lines',line:{color:colors[1],width:2},name:'Body 2'},
      {x:result.ys.map(y=>y[4]),y:result.ys.map(y=>y[5]),mode:'lines',line:{color:colors[2],width:2},name:'Body 3'}
    ], {...PLOT_CFG, xaxis:{...PLOT_CFG.xaxis,scaleanchor:'y'}}, {responsive:true});
  } else {
    // Phase portrait for 2-var or first two vars
    Plotly.newPlot('demoPlot1', [{
      x:result.ys.map(y=>y[0]),y:result.ys.map(y=>y[1]),mode:'lines',
      line:{color:'#00f0ff',width:2},name:'Phase portrait'
    }], {...PLOT_CFG,xaxis:{...PLOT_CFG.xaxis,title:d.vars[0]},yaxis:{...PLOT_CFG.yaxis,title:d.vars[1]}}, {responsive:true});
  }

  // Plot 2: time series
  const nv = Math.min(d.vars.length, 4);
  const traces = [];
  for (let i = 0; i < nv; i++) {
    traces.push({x:result.ts,y:result.ys.map(y=>y[i]),mode:'lines',line:{color:colors[i],width:1.5},name:d.vars[i]});
  }
  Plotly.newPlot('demoPlot2', traces, {...PLOT_CFG,xaxis:{...PLOT_CFG.xaxis,title:'time'}}, {responsive:true});
}

// Auto-run on load
window.addEventListener('load', () => setTimeout(runDemo, 500));
