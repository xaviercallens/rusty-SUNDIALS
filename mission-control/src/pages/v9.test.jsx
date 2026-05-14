import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import PhysicsPage from './PhysicsPage';
import DiscoveriesPage from './DiscoveriesPage';
import VerificationPage from './VerificationPage';
import { AuthProvider } from '../hooks/useAuth';

// Mock the API client
vi.mock('../api/client', () => ({
  default: {
    runKalundborg: vi.fn().mockResolvedValue({ global_optima: 'Jubail Industrial City, Saudi Arabia', co2_reduction: 32.4, agri_boost: 240 }),
    runHpc: vi.fn().mockResolvedValue({ a100_speedup: 441.8, precision_error: 9.54e-07 }),
    runPlanet: vi.fn().mockResolvedValue({ optimal_node: 'Namib Coastal Edge', neutrality_years: 0.6, drawdown_megatons: 15492 }),
    results: vi.fn().mockResolvedValue({
      hpc_exascale: { a100_speedup: 441.8, precision_error: 9.54e-07 },
      kalundborg: { global_optima: 'Jubail Industrial City', co2_reduction: 32.4, agri_boost: 240 },
      planetary: { optimal_node: 'Namib Coastal Edge', neutrality_years: 0.6, drawdown_megatons: 15492 }
    }),
    getVerification: vi.fn().mockResolvedValue({
      total_proofs: 4, proved: 4, failed: 0, pass_rate: 100,
      proofs: [
        { theorem: 'theorem hpc_tensor_core_precision', status: 'proved' }
      ]
    }),
    runVerification: vi.fn().mockResolvedValue({ total_proofs: 4, proved: 4 })
  }
}));

describe('v9 Features Intense Testing', () => {
  it('PhysicsPage renders new v9 experiment buttons', () => {
    render(<AuthProvider><PhysicsPage /></AuthProvider>);
    expect(screen.getByText('KALUNDBORG 2.0')).toBeInTheDocument();
    expect(screen.getByText('HPC EXASCALE')).toBeInTheDocument();
    expect(screen.getByText('PLANET CYCLE')).toBeInTheDocument();
  });

  it('DiscoveriesPage renders new v9 discoveries', async () => {
    render(<DiscoveriesPage />);
    await waitFor(() => {
      expect(screen.getByText('Kalundborg 2.0 EIP Global Topology')).toBeInTheDocument();
      expect(screen.getByText('HPC A100 Tensor Core Exascale Validation')).toBeInTheDocument();
      expect(screen.getByText('Earth Digital Twin Geo-Optimization')).toBeInTheDocument();
    });
  });

  it('VerificationPage renders the Lean 4 proof metrics', async () => {
    render(<AuthProvider><VerificationPage /></AuthProvider>);
    await waitFor(() => {
      expect(screen.getByText('theorem hpc_tensor_core_precision')).toBeInTheDocument();
      expect(screen.getByText('100%')).toBeInTheDocument();
    });
  });
});
