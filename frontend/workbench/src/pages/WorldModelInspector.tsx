import { Card } from '@/ui/components/Card';

export function WorldModelInspector() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-2xl font-bold text-text-primary">World Model Inspector</h1>
        <p className="text-text-secondary">Inspect the live causal graph of your system</p>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <div className="lg:col-span-3">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Causal Graph</h2>
            <div className="bg-surface-base border border-surface-border rounded-lg h-[600px] flex items-center justify-center text-text-muted">
              Interactive force-directed graph (React Flow)
            </div>
          </Card>
        </div>
        
        <div className="space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Node Inspector</h2>
            <div className="space-y-3 text-sm">
              <p className="text-text-muted">Click a node in the graph to inspect</div>
              <div className="p-3 bg-surface-base border border-surface-border rounded">
                <div className="font-medium text-text-primary">auth-service</div>
                <div className="text-xs text-text-muted">Type: Code | Health: 0.85</div>
              </div>
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Filters</h2>
            <div className="space-y-2">
              {['Code', 'Infra', 'Data', 'Team', 'Traffic'].map(layer => (
                <label key={layer} className="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" defaultChecked className="accent-intent-simulating" />
                  <span className="text-sm text-text-secondary">{layer}</span>
                </label>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}