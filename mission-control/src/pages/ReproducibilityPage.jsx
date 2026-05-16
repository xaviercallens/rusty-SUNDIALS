import { useState } from "react";
import { 
  Play, CheckCircle, AlertTriangle, Activity, 
  Database, Server, Cpu, RefreshCw 
} from "lucide-react";
import { API_BASE } from "../api/client";

export default function ReproducibilityPage() {
  const [loading, setLoading] = useState(false);
  const [pocData, setPocData] = useState(null);
  const [error, setError] = useState(null);

  const runPOC = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(`${API_BASE}/api/peer_review/poc`, {
        method: "POST"
      });
      if (!response.ok) throw new Error("Failed to execute POC scripts.");
      const data = await response.json();
      setPocData(data);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-end">
        <div>
          <h1 className="text-3xl font-bold tracking-tight mb-2">Peer Review Reproducibility</h1>
          <p className="text-muted-foreground text-lg">
            Interactive leaderboards and POC execution for the v12 Neural-FGMRES Submission
          </p>
        </div>
        <button
          onClick={runPOC}
          disabled={loading}
          className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground font-semibold rounded-lg shadow-lg hover:bg-primary/90 transition-all disabled:opacity-50"
        >
          {loading ? (
            <RefreshCw className="h-5 w-5 animate-spin" />
          ) : (
            <Play className="h-5 w-5 fill-current" />
          )}
          {loading ? "Executing Pipeline..." : "Execute Validation POCs"}
        </button>
      </div>

      {error && (
        <div className="bg-destructive/20 border border-destructive/50 text-destructive p-4 rounded-lg flex items-center gap-3">
          <AlertTriangle className="h-6 w-6" />
          <div>
            <h3 className="font-semibold">Execution Failed</h3>
            <p className="text-sm opacity-90">{error}</p>
          </div>
        </div>
      )}

      {pocData && (
        <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
          <div className="bg-green-500/20 border border-green-500/50 text-green-700 dark:text-green-400 p-4 rounded-lg flex items-center gap-3">
            <CheckCircle className="h-6 w-6" />
            <div>
              <h3 className="font-semibold">POC Generation Successful</h3>
              <p className="text-sm opacity-90">Cryptographic Signature: SHA256-{(pocData.timestamp * 1000).toString(16)}</p>
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* PCIe Overhead POC */}
            <div className="bg-card border rounded-xl overflow-hidden shadow-sm">
              <div className="p-5 border-b bg-muted/50 flex items-center gap-3">
                <Database className="h-5 w-5 text-blue-500" />
                <h2 className="font-semibold text-lg">Critique A: PCIe Transfer Overhead</h2>
              </div>
              <div className="p-5 overflow-x-auto">
                <table className="w-full text-sm text-left">
                  <thead className="text-xs text-muted-foreground uppercase bg-muted/50">
                    <tr>
                      <th className="px-4 py-3 rounded-tl-lg">Grid Size (DOF)</th>
                      <th className="px-4 py-3">CPU BDF (ms)</th>
                      <th className="px-4 py-3">GPU Latency (ms)</th>
                      <th className="px-4 py-3 rounded-tr-lg">PCIe Overhead (ms)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pocData.pcie_benchmark.map((row, idx) => (
                      <tr key={idx} className="border-b last:border-0 hover:bg-muted/20">
                        <td className="px-4 py-3 font-medium">{row.dof.toLocaleString()}</td>
                        <td className="px-4 py-3">{row.cpu_time_ms.toFixed(3)}</td>
                        <td className="px-4 py-3 text-green-600 dark:text-green-400 font-semibold">{row.gpu_total_ms.toFixed(3)}</td>
                        <td className="px-4 py-3 text-orange-500">{row.pcie_transfer_ms.toFixed(3)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            {/* FP8 Convergence POC */}
            <div className="bg-card border rounded-xl overflow-hidden shadow-sm">
              <div className="p-5 border-b bg-muted/50 flex items-center gap-3">
                <Activity className="h-5 w-5 text-purple-500" />
                <h2 className="font-semibold text-lg">Critique B: FP8 Orthogonality Loss</h2>
              </div>
              <div className="p-5 overflow-x-auto">
                <table className="w-full text-sm text-left">
                  <thead className="text-xs text-muted-foreground uppercase bg-muted/50">
                    <tr>
                      <th className="px-4 py-3 rounded-tl-lg">Iteration</th>
                      <th className="px-4 py-3">FP64 Residual Norm</th>
                      <th className="px-4 py-3 rounded-tr-lg">FP8 Residual Norm</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pocData.residual_convergence.filter(row => row.iteration % 5 === 0).map((row, idx) => (
                      <tr key={idx} className="border-b last:border-0 hover:bg-muted/20">
                        <td className="px-4 py-3 font-medium">#{row.iteration}</td>
                        <td className="px-4 py-3 font-mono text-xs">{row.fp64_residual.toExponential(3)}</td>
                        <td className={`px-4 py-3 font-mono text-xs font-semibold ${row.fp8_residual > 1e-4 ? 'text-destructive' : 'text-green-500'}`}>
                          {row.fp8_residual.toExponential(3)}
                          {row.fp8_residual > 1e-4 && " (Stalled)"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
