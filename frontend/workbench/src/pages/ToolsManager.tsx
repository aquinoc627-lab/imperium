import { Card } from '@/ui/components/Card';
import { Button } from '@/ui/components/Button';
import { Input } from '@/ui/components/Input';
import { Plus, Search, Server, Cpu, Plugin, CheckCircle, XCircle, Settings } from 'lucide-react';

const mockServers = [
  { id: 'github', name: 'GitHub Integration', status: 'connected', tools: 12, type: 'NPX' },
  { id: 'jira', name: 'Jira Cloud', status: 'connected', tools: 8, type: 'NPX' },
  { id: 'postgres', name: 'PostgreSQL Analyzer', status: 'disconnected', tools: 5, type: 'Docker' },
  { id: 'k8s', name: 'Kubernetes Operator', status: 'connected', tools: 15, type: 'NPX' },
  { id: 'slack', name: 'Slack Bot', status: 'error', tools: 6, type: 'NPX' },
];

export function ToolsManager() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-2xl font-bold text-text-primary">Tools & Capabilities</h1>
          <p className="text-text-secondary">Manage MCP servers and synthesized capabilities</p>
        </div>
        <div className="flex gap-2">
          <Button variant="secondary">
            <Search size={16} className="mr-2" />
            Discover APIs
          </Button>
          <Button variant="primary">
            <Plus size={16} className="mr-2" />
            Add Server
          </Button>
        </div>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <h2 className="font-medium text-text-primary mb-4">MCP Servers</h2>
          <div className="space-y-3">
            {mockServers.map(server => (
              <div key={server.id} className="p-4 border border-surface-border/50 rounded-lg flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-surface-overlay rounded-lg">
                    <Server className="text-text-secondary" size={20} />
                  </div>
                  <div>
                    <h3 className="font-medium text-text-primary">{server.name}</h3>
                    <div className="flex items-center gap-2 text-sm text-text-muted">
                      <span className="px-2 py-0.5 bg-surface-base rounded">{server.type}</span>
                      <span>{server.tools} tools</span>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                    server.status === 'connected' ? 'bg-system-local/20 text-system-local' :
                    server.status === 'error' ? 'bg-risk-critical/20 text-risk-critical' :
                    'bg-text-muted/20 text-text-muted'
                  }`}>
                    {server.status}
                  </span>
                  <Button variant="ghost" size="sm" title="Configure">
                    <Settings size={16} />
                  </Button>
                  <Button variant="ghost" size="sm" title={server.status === 'connected' ? 'Disconnect' : 'Connect'}>
                    {server.status === 'connected' ? <XCircle size={16} /> : <CheckCircle size={16} />}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </Card>
        
        <Card>
          <h2 className="font-medium text-text-primary mb-4">Synthesized Capabilities</h2>
          <div className="space-y-3">
            {[
              { name: 'github.query_issues', source: 'Synthesized from OpenAPI', calls: 47 },
              { name: 'jira.create_ticket', source: 'Synthesized from GraphQL', calls: 23 },
              { name: 'postgres.explain_query', source: 'Synthesized from pg_catalog', calls: 12 },
              { name: 'k8s.get_pods', source: 'Synthesized from K8s API', calls: 89 },
            ].map((cap, i) => (
              <div key={i} className="p-3 border border-surface-border/50 rounded-lg flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-surface-overlay rounded-lg">
                    <Plugin className="text-intent-simulating" size={18} />
                  </div>
                  <div>
                    <code className="font-mono text-sm text-text-primary">{cap.name}</code>
                    <div className="text-xs text-text-muted">{cap.source}</div>
                  </div>
                </div>
                <span className="text-xs text-text-muted font-mono">{cap.calls} calls</span>
              </div>
            ))}
          </div>
        </Card>
      </div>
      
      <Card>
        <h2 className="font-medium text-text-primary mb-4">Capability Marketplace</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {[
            { name: 'AWS Cost Explorer', description: 'Query AWS costs and usage', category: 'Cloud' },
            { name: 'Datadog Dashboards', description: 'Create and manage dashboards', category: 'Monitoring' },
            { name: 'Terraform Plan Parser', description: 'Analyze Terraform plans', category: 'IaC' },
            { name: 'Notion Sync', description: 'Sync notes to Notion', category: 'Productivity' },
            { name: 'Figma to Code', description: 'Generate React from Figma', category: 'Design' },
            { name: 'Linear Integration', description: 'Manage Linear issues', category: 'Project Mgmt' },
          ].map((cap, i) => (
            <div key={i} className="p-4 border border-surface-border/50 rounded-lg hover:border-intent-simulating/50 transition-colors">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs px-2 py-0.5 bg-surface-overlay text-text-muted rounded">{cap.category}</span>
              </div>
              <h3 className="font-medium text-text-primary mb-1">{cap.name}</h3>
              <p className="text-sm text-text-muted mb-3">{cap.description}</p>
              <Button variant="ghost" size="sm" className="w-full">Install</Button>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}