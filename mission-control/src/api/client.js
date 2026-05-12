/**
 * API Client for Rusty-SUNDIALS Mission Control
 * Connects to the Cloud Run backend
 */
const API_BASE = import.meta.env.VITE_API_URL || '';

async function request(path, options = {}) {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...options.headers },
    ...options,
  });
  if (!res.ok) throw new Error(`API ${res.status}: ${await res.text()}`);
  return res.json();
}

export const api = {
  // Health
  health: () => request('/health'),
  
  // Pipeline
  runPipeline: (config = {}) => request('/run', { method: 'POST', body: JSON.stringify(config) }),
  runPhysics: (config = {}) => request('/physics', { method: 'POST', body: JSON.stringify(config) }),
  runSweep: () => request('/sweep', { method: 'POST' }),
  runBioreactor: () => request('/bioreactor', { method: 'POST' }),
  
  // Service info
  info: () => request('/'),
};

export default api;
