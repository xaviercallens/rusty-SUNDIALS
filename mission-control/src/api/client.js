/**
 * API Client for Rusty-SUNDIALS Mission Control
 * Sends Google Sign-In JWT as Bearer token for role-based access.
 */
const API_BASE = import.meta.env.VITE_API_URL || '';

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
    let result = { metric_achieved: "1.1e-15", validation: "PASSED", deviance: "0.00%", execution_time: "40.5s" };
    if (protocol_id === 'SOP-2') result = { metric_achieved: "6 Iterations", validation: "PASSED", deviance: "0.00%", execution_time: "72.1s" };
    if (protocol_id === 'SOP-3') result = { metric_achieved: "$0.021", validation: "PASSED", deviance: "+5.0%", execution_time: "18.2s" };
    return {
      execution_id: `EXEC-${Math.floor(Math.random()*1000)}`,
      protocol_id,
      timestamp: new Date().toISOString(),
      status: "success",
      result
    };
  }
  if (path === '/kalundborg') return MOCK_RESULTS.kalundborg;
  if (path === '/hpc_exascale') return MOCK_RESULTS.hpc_exascale;
  if (path === '/planetary') return MOCK_RESULTS.planetary;
  
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
};

export default api;
