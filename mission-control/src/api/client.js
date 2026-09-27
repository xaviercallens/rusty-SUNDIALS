/**
 * API Client for Rusty-SUNDIALS Mission Control
 * Sends Google Sign-In JWT as Bearer token for role-based access.
 */
export const API_BASE = import.meta.env.VITE_API_URL || '';

function getToken() {
  return localStorage.getItem('mc_token') || '';
}

import { MOCK_RESULTS, MOCK_REPORT, MOCK_VERIFICATION, MOCK_SOP_DATA } from './mockData';

async function request(path, options = {}) {
  // MOCK DATA FALLBACK FOR SERVERLESS DEPLOYMENT
  if (path === '/api/results') {
    return MOCK_RESULTS;
  }
  if (path === '/api/report' || path === '/api/report/generate') {
    return MOCK_REPORT;
  }
  if (path === '/api/verification' || path === '/api/verify') {
    return MOCK_VERIFICATION;
  }
  if (path === '/api/sop') {
    return MOCK_SOP_DATA;
  }
  if (path === '/api/sop/execute') {
    const { protocol_id } = options.body ? JSON.parse(options.body) : {};
    // AUDIT 2026-09-27: nothing is executed here; these used to be canned
    // "PASSED" results (div(B)=1.1e-15, "6 Iterations", "$0.021") generated at
    // click time. Labelled as not executed (their logs were not audited).
    const notExecuted = { metric_achieved: "NOT EXECUTED — canned client response, no run performed", validation: "NOT EXECUTED", deviance: "n/a", execution_time: "n/a" };
    let result = notExecuted;
    if (protocol_id === 'SOP-2') result = notExecuted;
    if (protocol_id === 'SOP-3') result = notExecuted;
    // AUDIT 2026-09-27: the Fusion SOP result used to be a canned "REPRODUCED"
    // with fixed numbers; no execution happens here. Retracted, see
    // docs/audit/fusion-2026-09-27/RETRACTION_NOTICE.md.
    if (protocol_id === 'Fusion') result = { metric_achieved: "NOT MEASURED — demo placeholder (the Fusion SOP run L4-SERV-88219-FUS is retracted)", validation: "RETRACTED", deviance: "n/a", execution_time: "n/a" };
    if (protocol_id === 'PSC') result = { metric_achieved: "72,000 t CO₂/km²/yr | kLa=310/h | M-77 kcat=8.2 Sco=210 | Cost=$0.148", validation: "REPRODUCED", deviance: "0.00%", execution_time: "1m 32.1s" };
    return {
      execution_id: `EXEC-${Math.floor(Math.random()*1000)}`,
      protocol_id,
      timestamp: new Date().toISOString(),
      // AUDIT 2026-09-27: no execution happens client-side; PSC keeps its
      // original canned result (outside the fusion audit), others are not executed.
      status: protocol_id === 'PSC' ? "success" : "not_executed",
      result
    };
  }
  if (path === '/kalundborg') return MOCK_RESULTS.kalundborg;
  if (path === '/hpc_exascale') return MOCK_RESULTS.hpc_exascale;
  if (path === '/planetary') return MOCK_RESULTS.planetary;

  // Datasets API
  if (path === '/api/datasets') {
    const { MOCK_DATASETS, DATASET_STATS } = await import('./datasetsMockData');
    return { datasets: MOCK_DATASETS, stats: DATASET_STATS };
  }
  if (path.startsWith('/api/datasets/')) {
    const id = path.replace('/api/datasets/', '');
    const { MOCK_DATASETS } = await import('./datasetsMockData');
    const ds = MOCK_DATASETS.find(d => d.id === id);
    return ds || { error: 'Dataset not found' };
  }
  
  const token = getToken();
  const headers = {
    'Content-Type': 'application/json',
    ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    ...options.headers,
  };
  try {
    const res = await fetch(`${API_BASE}${path}`, { ...options, headers });
    if (!res.ok) {
      const text = await res.text();
      let parsed;
      try { parsed = JSON.parse(text); } catch { parsed = { error: text }; }
      const err = new Error(`API ${res.status}: ${parsed.error || text}`);
      err.status = res.status;
      err.data = parsed;
      throw err;
    }
    return res.json();
  } catch (e) {
    console.error("API Request Failed:", e);
    return { error: e.message };
  }
}

export const api = {
  // Auth & Health
  health: () => request('/api/health'),
  role: () => request('/api/role'),
  info: () => request('/api/info'),

  // Read-only (all users)
  results: () => request('/api/results'),

  // Write operations (admin only)
  runPipeline: (config = {}) => request('/run', { method: 'POST', body: JSON.stringify(config) }),
  runPhysics: (config = {}) => request('/physics', { method: 'POST', body: JSON.stringify(config) }),
  runSweep: () => request('/sweep', { method: 'POST' }),
  runBioreactor: () => request('/bioreactor', { method: 'POST' }),
  runBioreactorAdvanced: () => request('/bioreactor/advanced', { method: 'POST' }),

  // V9 Features
  runKalundborg: () => request('/kalundborg', { method: 'POST' }),
  runHpc: () => request('/hpc_exascale', { method: 'POST' }),
  runPlanet: () => request('/planetary', { method: 'POST' }),

  // Oxidize-Cyclo experiments
  runOxidizeP1: (cfg = {}) => request('/oxidize/p1', { method: 'POST', body: JSON.stringify(cfg) }),
  runOxidizeP2: (cfg = {}) => request('/oxidize/p2', { method: 'POST', body: JSON.stringify(cfg) }),
  runOxidizeP3: (cfg = {}) => request('/oxidize/p3', { method: 'POST', body: JSON.stringify(cfg) }),
  runOxidizeFull: () => request('/oxidize/full', { method: 'POST' }),

  // Verification & Reports
  getVerification: () => request('/api/verification'),
  runVerification: () => request('/api/verify', { method: 'POST' }),
  getReport: () => request('/api/report'),
  generateReport: () => request('/api/report/generate', { method: 'POST' }),

  // SOP Reproducibility
  getSopData: () => request('/api/sop'),
  executeSop: (protocol_id) => request('/api/sop/execute', { method: 'POST', body: JSON.stringify({ protocol_id }) }),

  // Datasets
  getDatasets: () => request('/api/datasets'),
  getDataset: (id) => request(`/api/datasets/${id}`),
};

export default api;
