import { Card } from '@/ui/components/Card';
import { Button } from '@/ui/components/Button';
import { Input } from '@/ui/components/Input';
import { Textarea } from '@/ui/components/Textarea';

export function IntentWorkspace() {
  return (
    <div className="space-y-6 max-w-4xl">
      <div>
        <h1 className="font-display text-2xl font-bold text-text-primary">Intent Workspace</h1>
        <p className="text-text-secondary">Compose, compile, simulate, and execute intents</p>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Intent Composer */}
        <div className="lg:col-span-2 space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Intent Composer</h2>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">Natural Language</label>
                <Textarea
                  placeholder="Migrate auth to passkeys, zero downtime, under 2hrs"
                  rows={4}
                  className="font-mono"
                />
              </div>
              
              <div className="flex items-center gap-2">
                <Button variant="primary" className="flex-1">Compile to IR</Button>
                <Button variant="secondary">Load Template</Button>
              </div>
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Structured IR</h2>
            <div className="bg-surface-base border border-surface-border rounded-lg p-4 font-mono text-sm text-text-muted max-h-96 overflow-auto">
              {`// Compile natural language first to see the IR
{
  "id": "01HXK3JQ9V...",
  "name": "Migrate auth to passkeys",
  "goal": { "description": "...", "category": "Migration" },
  "constraints": [...],
  "success_criteria": [...],
  "tasks": [...],
  "risk_score": 0.7
}`}
            </div>
          </Card>
        </div>
        
        {/* Simulation Preview */}
        <div className="space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Simulation Preview</h2>
            <div className="space-y-3 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-text-muted">Success Probability</span>
                <span className="font-bold text-system-local">94.2%</span>
              </div>
              <div className="h-2 bg-surface-base rounded-full overflow-hidden">
                <div className="h-full bg-intent-simulating rounded-full" style={{ width: '94.2%' }} />
              </div>
              
              <div className="flex items-center justify-between">
                <span className="text-text-muted">Risk Score</span>
                <span className="font-bold text-risk-high">0.68</span>
              </div>
              
              <div className="border-t border-surface-border pt-3 space-y-2">
                <div className="flex justify-between">
                  <span className="text-text-muted">Estimated Cost</span>
                  <span className="font-mono text-text-primary">$230</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-text-muted">Timeline (median)</span>
                  <span className="font-mono text-text-primary">4.2 hrs</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-text-muted">Rollback Time</span>
                  <span className="font-mono text-system-local"><30s</span>
                </div>
              </div>
            </div>
            
            <div className="flex gap-2 pt-2">
              <Button variant="primary" className="flex-1">Approve & Execute</Button>
              <Button variant="ghost">Adjust Constraints</Button>
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-3">Top Risks</h2>
            <ul className="space-y-2 text-sm">
              <li className="flex items-start gap-2 text-text-secondary">
                <span className="text-risk-high">⚠</span>
                <span>DB lock during migration (6%, 5 min)</span>
              </li>
              <li className="flex items-start gap-2 text-text-secondary">
                <span className="text-risk-medium">⚠</span>
                <span>Cache stampede on cutover (2%)</span>
              </li>
            </ul>
            <Button variant="ghost" className="w-full mt-2 text-xs">View all 47 failure modes</Button>
          </Card>
        </div>
      </div>
    </div>
  );
}