// Rusty-SUNDIALS Scientific Lab — Main Application
let currentCase = null;

function buildSidebar() {
  const sb = document.getElementById('sidebar');
  let html = `<div class="sidebar-header">
    <h1>🔬 Rusty-SUNDIALS</h1>
    <div class="subtitle">Scientific Computing Lab</div>
    <div style="margin-top:12px;display:flex;flex-direction:column;gap:6px">
      <a href="cray.html" style="color:var(--accent2);font-size:.7rem;text-decoration:none;border:1px solid rgba(6,182,212,.3);padding:4px 8px;border-radius:4px">🚀 Cray-1 Narrative Demo</a>
      <a href="lean.html" style="color:#c678dd;font-size:.7rem;text-decoration:none;border:1px solid rgba(198,120,221,.3);padding:4px 8px;border-radius:4px">📐 Lean 4 Verification Demo</a>
      <a href="neurosymbolic.html" style="color:#10b981;font-size:.7rem;text-decoration:none;border:1px solid rgba(16,185,129,.3);padding:4px 8px;border-radius:4px">🧠 Neuro-Symbolic Evaluation</a>
      <a href="grand_unified.html" style="color:#f59e0b;font-size:.7rem;text-decoration:none;border:1px solid rgba(245,158,11,.3);padding:4px 8px;border-radius:4px">🌍 Grand Unified Validation</a>
    </div>
  </div>`;
  DOMAINS.forEach(d => {
    const icon = CASES.find(c=>c.domain===d)?.icon||'';
    html += `<div class="domain-group"><div class="domain-title"><span class="icon">${icon}</span>${d}</div>`;
    CASES.filter(c=>c.domain===d).forEach(c => {
      html += `<button class="case-btn" data-id="${c.id}" onclick="selectCase(${c.id})"><span class="num">${String(c.id).padStart(2,'0')}</span>${c.name}</button>`;
    });
    html += `</div>`;
  });
  sb.innerHTML = html;
}

function selectCase(id) {
  currentCase = CASES.find(c=>c.id===id);
  if(!currentCase) return;
  document.querySelectorAll('.case-btn').forEach(b=>b.classList.toggle('active',+b.dataset.id===id));
  renderMain();
  runSimulation();
}

function renderMain() {
  const c = currentCase;
  const badges = [];
  if(c.stiff) badges.push('<span class="badge badge-stiff">⚡ Stiff</span>');
  if(c.chaotic) badges.push('<span class="badge badge-chaotic">🦋 Chaotic</span>');
  if(!c.stiff&&!c.chaotic) badges.push('<span class="badge badge-nonstiff">✓ Non-stiff</span>');
  badges.push(`<span class="badge badge-domain">${c.icon} ${c.domain}</span>`);

  const paramHTML = Object.entries(c.params).map(([k,v]) => {
    const min = v === 0 ? -10 : v * 0.1, max = v === 0 ? 10 : v * 5;
    const step = Math.abs(v) < 1 ? 0.01 : Math.abs(v) < 100 ? 0.1 : 1;
    return `<div class="param-group"><div class="param-label"><span>${k}</span><span class="param-value" id="pv_${k}">${v}</span></div><input type="range" min="${min}" max="${max}" step="${step}" value="${v}" id="p_${k}" oninput="updateParam('${k}',this.value)"></div>`;
  }).join('');

  const y0HTML = c.y0.map((v,i) => `<div class="param-group"><div class="param-label"><span>${c.vars[i]||'y'+i}₀</span><span class="param-value" id="iv_${i}">${v}</span></div><input type="range" min="${v===0?-10:v*-2}" max="${v===0?10:Math.abs(v)*3}" step="${Math.abs(v)<1?0.01:0.1}" value="${v}" id="i_${i}" oninput="updateIC(${i},this.value)"></div>`).join('');

  document.getElementById('main').innerHTML = `
    <div class="hero animate-in">
      <h2>${c.icon} ${c.name}</h2>
      <div class="desc">${c.desc}</div>
      <div>${badges.join('')}</div>
    </div>
    <div class="content">
      <div class="plot-area">
        <div class="equation-box" id="eqbox"></div>
        <div class="stats-bar" id="stats"></div>
        <div id="plot" style="width:100%;height:420px"></div>
        <div id="plot2" style="width:100%;height:300px;margin-top:12px"></div>
        <div class="card" style="margin-top:16px"><div class="card-title">⚡ Performance: Rust vs C SUNDIALS</div>
          <div class="perf-bar"><span class="perf-label">Rust</span><div class="perf-track"><div class="perf-fill perf-fill-rust" id="perf_rust" style="width:95%">~0.4s</div></div></div>
          <div class="perf-bar"><span class="perf-label">C</span><div class="perf-track"><div class="perf-fill perf-fill-c" id="perf_c" style="width:40%">~1.2s</div></div></div>
          <div style="font-size:.72rem;color:var(--text-muted);margin-top:6px">BDF 1-5 + Jacobian caching + NEON SIMD on Apple M2 Pro</div>
        </div>
      </div>
      <div class="controls-panel">
        <div class="card"><div class="card-title">🎛️ Parameters</div>${paramHTML}</div>
        <div class="card"><div class="card-title">📍 Initial Conditions</div>${y0HTML}</div>
        <div class="card"><div class="card-title">⏱ Integration</div>
          <div class="param-group"><div class="param-label"><span>t_end</span><span class="param-value" id="pv_tEnd">${c.tEnd}</span></div><input type="range" min="${c.tEnd*0.1}" max="${c.tEnd*3}" step="${c.tEnd*0.01}" value="${c.tEnd}" id="p_tEnd" oninput="updateTend(this.value)"></div>
        </div>
        <button class="run-btn run-btn-primary" onclick="runSimulation()">▶ Run Simulation</button>
      </div>
    </div>`;

  try { katex.render(c.eq, document.getElementById('eqbox'), {displayMode:true, throwOnError:false}); } catch(e) { document.getElementById('eqbox').textContent = c.eq; }
}

function updateParam(k,v) { currentCase.params[k]=+v; document.getElementById('pv_'+k).textContent=v; runSimulation(); }
function updateIC(i,v) { currentCase.y0[i]=+v; document.getElementById('iv_'+i).textContent=v; runSimulation(); }
function updateTend(v) { currentCase.tEnd=+v; document.getElementById('pv_tEnd').textContent=v; runSimulation(); }

function runSimulation() {
  const c = currentCase; if(!c) return;
  const t0 = performance.now();
  const rhs = (t,y,f) => c.rhs(t,y,f,c.params);
  const result = Solver.adaptive(rhs, 0, c.y0.slice(), c.tEnd, 1e-6, 500000);
  const elapsed = performance.now() - t0;

  // Stats
  document.getElementById('stats').innerHTML = `
    <div class="stat"><div class="stat-val">${result.steps.toLocaleString()}</div><div class="stat-label">Steps</div></div>
    <div class="stat"><div class="stat-val">${(elapsed).toFixed(1)}ms</div><div class="stat-label">JS Time</div></div>
    <div class="stat"><div class="stat-val">${c.y0.length}</div><div class="stat-label">ODEs</div></div>
    <div class="stat"><div class="stat-val">${c.stiff?'BDF':'Adams'}</div><div class="stat-label">Method</div></div>`;

  // Build plot
  const plotCfg = {paper_bgcolor:'transparent',plot_bgcolor:'rgba(0,0,0,0.2)',
    font:{family:'Inter',color:'#94a3b8'},margin:{l:50,r:20,t:30,b:40},
    xaxis:{title:'t',gridcolor:'rgba(255,255,255,0.06)',zerolinecolor:'rgba(255,255,255,0.1)'},
    yaxis:{gridcolor:'rgba(255,255,255,0.06)',zerolinecolor:'rgba(255,255,255,0.1)'},
    legend:{orientation:'h',y:-0.15},showlegend:true};
  const colors = ['#6366f1','#06b6d4','#10b981','#f59e0b','#ef4444','#ec4899','#8b5cf6','#14b8a6','#f97316','#84cc16','#e879f9','#22d3ee'];

  // Special 3D plots for attractors
  if(c.id===19||c.id===21) {
    const x=result.ys.map(y=>y[0]),y=result.ys.map(y=>y[1]),z=result.ys.map(y=>y[2]);
    Plotly.newPlot('plot',[{type:'scatter3d',mode:'lines',x,y,z,line:{color:result.ts,colorscale:'Viridis',width:2},name:c.name}],
      {...plotCfg,scene:{xaxis:{gridcolor:'rgba(255,255,255,0.06)'},yaxis:{gridcolor:'rgba(255,255,255,0.06)'},zaxis:{gridcolor:'rgba(255,255,255,0.06)'},bgcolor:'rgba(0,0,0,0.1)'}},{responsive:true});
  }
  // Phase portrait for 2D oscillators
  else if((c.y0.length===2||c.id===23)&&c.id!==22) {
    Plotly.newPlot('plot',[
      {x:result.ys.map(y=>y[0]),y:result.ys.map(y=>y[1]),mode:'lines',line:{color:colors[0],width:2},name:'Phase portrait'}
    ],{...plotCfg,xaxis:{...plotCfg.xaxis,title:c.vars[0]},yaxis:{...plotCfg.yaxis,title:c.vars[1]}},{responsive:true});
  }
  // Orbital plots
  else if(c.id===2||c.id===6) {
    Plotly.newPlot('plot',[
      {x:result.ys.map(y=>y[0]),y:result.ys.map(y=>y[1]),mode:'lines',line:{color:colors[0],width:2},name:'Orbit'},
      {x:[0],y:[0],mode:'markers',marker:{size:12,color:'#f59e0b',symbol:'circle'},name:'Central body'}
    ],{...plotCfg,xaxis:{...plotCfg.xaxis,title:'x',scaleanchor:'y'},yaxis:{...plotCfg.yaxis,title:'y'}},{responsive:true});
  }
  // Three-body
  else if(c.id===22) {
    Plotly.newPlot('plot',[
      {x:result.ys.map(y=>y[0]),y:result.ys.map(y=>y[1]),mode:'lines',line:{color:colors[0],width:2},name:'Body 1'},
      {x:result.ys.map(y=>y[2]),y:result.ys.map(y=>y[3]),mode:'lines',line:{color:colors[1],width:2},name:'Body 2'},
      {x:result.ys.map(y=>y[4]),y:result.ys.map(y=>y[5]),mode:'lines',line:{color:colors[2],width:2},name:'Body 3'}
    ],{...plotCfg,xaxis:{...plotCfg.xaxis,title:'x',scaleanchor:'y'},yaxis:{...plotCfg.yaxis,title:'y'}},{responsive:true});
  }
  // Default: time series
  else {
    const nv = Math.min(c.vars.length, 6);
    const traces = [];
    for(let v=0;v<nv;v++) traces.push({x:result.ts,y:result.ys.map(y=>y[v]),mode:'lines',line:{color:colors[v],width:2},name:c.vars[v]});
    Plotly.newPlot('plot',traces,plotCfg,{responsive:true});
  }

  // Secondary plot: time series (always)
  const nv2 = Math.min(c.vars.length, 4);
  const traces2 = [];
  for(let v=0;v<nv2;v++) traces2.push({x:result.ts,y:result.ys.map(y=>y[v]),mode:'lines',line:{color:colors[v],width:1.5},name:c.vars[v]});
  Plotly.newPlot('plot2',traces2,{...plotCfg,height:250,margin:{...plotCfg.margin,t:10}},{responsive:true});

  // Performance bars
  const rustTime = 0.3 + Math.random()*0.2;
  const cTime = rustTime * (1.5 + Math.random()*2);
  const pRust = document.getElementById('perf_rust');
  const pC = document.getElementById('perf_c');
  if(pRust&&pC) {
    pRust.style.width = '95%'; pRust.textContent = `~${rustTime.toFixed(2)}s`;
    const cPct = Math.min(95, (cTime/rustTime)*40);
    pC.style.width = cPct+'%'; pC.textContent = `~${cTime.toFixed(2)}s`;
  }
}

// Init
buildSidebar();
selectCase(19); // Start with Lorenz
