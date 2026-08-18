import { Target, BarChart3, Brain, Package, Wrench, Mic, Settings, Plus } from 'lucide-react';
import { Card } from '@/ui/components/Card';

const quickActions = [
  { path: '/intent/new', label: 'New Intent', icon: Plus, description: 'Create a new intent from natural language', shortcut: '⌘N' },
  { path: '/simulate', label: 'Run Simulation', icon: BarChart3, description: 'Explore counterfactuals and what-if scenarios', shortcut: '⌘S' },
  { path: '/world', label: 'World Model', icon: Brain, description: 'Inspect the causal graph of your system', shortcut: '⌘W' },
  { path: '/vault', label: 'Vault Browser', icon: Package, description: 'Search and explore your knowledge vault', shortcut: '⌘V' },
  { path: '/tools', label: 'Tools & Plugins', icon: Wrench, description: 'Manage MCP servers and capabilities', shortcut: '⌘T' },
  { path: '/voice', label: 'Voice Settings', icon: Mic, description: 'Configure STT, TTS, and wake word', shortcut: '⌘O' },
];

export function Dashboard() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-3xl font-bold text-text-primary">IMPERIUM Workbench</h1>
        <p className="text-text-secondary mt-1">The Self-Synthesizing Intent Runtime</p>
      </div>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {quickActions.map((action) => (
          <Card key={action.path} className="cursor-pointer hover:border-intent-simulating/50 transition-colors" as="a" href={action.path}>
            <div className="flex items-start gap-3">
              <div className="p-2 bg-surface-overlay rounded-lg">
                <action.icon className="text-intent-simulating" size={24} />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between">
                  <h3 className="font-medium text-text-primary">{action.label}</h3>
                  <kbd className="text-xs text-text-muted px-1.5 py-0.5 bg-surface-base rounded">{action.shortcut}</kbd>
                </div>
                <p className="text-sm text-text-muted mt-1">{action.description}</p>
              </div>
            </div>
          </Card>
        ))}
      </div>
      
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <h3 className="font-medium text-text-primary mb-3">System Status</h3>
          <dl className="space-y-2 text-sm">
            <div className="flex justify-between">
              <dt className="text-text-muted">Routing</dt>
              <dd className="text-system-local font-mono">Local Only</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-text-muted">Models Loaded</dt>
              <dd className="font-mono">3/4</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-text-muted">Memory</dt>
              <dd className="font-mono">12.3/16 GB</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-text-muted">Privacy</dt>
              <dd className="text-system-airgap font-mono">Air-Gap</dd>
            </div>
          </dl>
        </Card>
        
        <Card>
          <h3 className="font-medium text-text-primary mb-3">Recent Intents</h3>
          <div className="space-y-2 text-sm">
            <p className="text-text-muted">No recent intents</p>
            <a href="/intent/new" className="text-intent-simulating hover:underline">Create your first intent →</a>
          </div>
        </Card>
        
        <Card>
          <h3 className="font-medium text-text-primary mb-3">Quick Commands</h3>
          <ul className="space-y-1 text-sm font-mono text-text-muted">
            <li>⌘N — New Intent</li>
            <li>⌘S — Simulate</li>
            <li>⌘W — World Model</li>
            <li>⌘V — Vault</li>
            <li>⌘T — Tools</li>
            <li>⌘O — Voice</li>
            <li>⌘P — Policy</li>
          </ul>
        </Card>
      </div>
    </div>
  );
}