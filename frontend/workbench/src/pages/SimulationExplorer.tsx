import { Card } from '@/ui/components/Card';

export function SimulationExplorer() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-2xl font-bold text-text-primary">Simulation Explorer</h1>
        <p className="text-text-secondary">Explore counterfactuals, compare strategies, and analyze rollouts</p>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Rollout Visualization</h2>
            <div className="bg-surface-base border border-surface-border rounded-lg h-96 flex items-center justify-center text-text-muted">
              Timeline scrubber + service map + metric charts
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Metric Charts</h2>
            <div className="bg-surface-base border border-surface-border rounded-lg h-64 flex items-center justify-center text-text-muted">
              Error rate, latency P99, cost over time
            </div>
          </Card>
        </div>
        
        <div className="space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Counterfactual Explorer</h2>
            <div className="space-y-3">
              <input 
                type="text" 
                placeholder="What if traffic 3x during migration?"
                className="w-full px-3 py-2 bg-surface-base border border-surface-border rounded-lg text-text-primary placeholder-text-muted"
              />
              <div className="space-y-2 text-sm">
                <div className="p-2 bg-surface-base border border-surface-border rounded">
                  <div className="flex justify-between text-xs mb-1">
                    <span className="text-text-muted">What if traffic 3x?</span>
                    <span className="text-risk-high">Success: 78%</span>
                  </div>
                  <div className="text-xs text-text-muted">Baseline: 94% | Cost: $450 vs $230</div>
                </div>
                <div className="p-2 bg-surface-base border border-surface-border rounded">
                  <div className="flex justify-between text-xs mb-1">
                    <span className="text-text-muted">What if skip staging?</span>
                    <span className="text-risk-critical">Success: 61%</span>
                  </div>
                  <div className="text-xs text-text-muted">Data loss risk: 12%</div>
                </div>
              </div>
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Strategy Comparison</h2>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left text-text-muted border-b border-surface-border">
                    <th className="pb-2">Strategy</th>
                    <th className="pb-2">Success</th>
                    <th className="pb-2">Cost</th>
                    <th className="pb-2">Time</th>
                    <th className="pb-2">Rollback</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b border-surface-border/50">
                    <td className="py-2 font-medium">Conservative</td>
                    <td className="py-2 text-system-local">98%</td>
                    <td className="py-2 font-mono">$310</td>
                    <td className="py-2 font-mono">5.1 hrs</td>
                    <td className="py-2 font-mono"><30s</td>
                  </tr>
                  <tr className="border-b border-surface-border/50">
                    <td className="py-2 font-medium">Baseline</td>
                    <td className="py-2 text-system-local">94%</td>
                    <td className="py-2 font-mono">$230</td>
                    <td className="py-2 font-mono">4.2 hrs</td>
                    <td className="py-2 font-mono"><30s</td>
                  </tr>
                  <tr>
                    <td className="py-2 font-medium">Aggressive</td>
                    <td className="py-2 text-risk-medium">82%</td>
                    <td className="py-2 font-mono">$180</td>
                    <td className="py-2 font-mono">3.1 hrs</td>
                    <td className="py-2 font-mono">45s</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}