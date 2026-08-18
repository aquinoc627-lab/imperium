import { Card } from '@/ui/components/Card';
import { Button } from '@/ui/components/Button';
import { Input } from '@/ui/components/Input';
import { Textarea } from '@/ui/components/Textarea';
import { Shield, CheckCircle, AlertCircle, Plus, Edit, Trash2, Play, Download } from 'lucide-react';

const builtinPolicies = [
  { id: 'air-gap', name: 'Air-Gap Enforcement', description: 'Block all cloud routing without explicit approval', status: 'active', risk: 'critical' },
  { id: 'pii-redaction', name: 'PII Redaction', description: 'Scrub PII from all outbound requests', status: 'active', risk: 'critical' },
  { id: 'capability-authz', name: 'Capability Authorization', description: 'Only granted capabilities can be invoked', status: 'active', risk: 'high' },
  { id: 'resource-limits', name: 'Resource Limits', description: 'Enforce declared CPU/memory/network limits', status: 'active', risk: 'high' },
  { id: 'intent-approval', name: 'Intent Approval', description: 'High-risk intents require human approval', status: 'active', risk: 'medium' },
  { id: 'data-residency', name: 'Data Residency', description: 'Enforce data locality constraints', status: 'active', risk: 'medium' },
  { id: 'audit-logging', name: 'Audit Logging', description: 'Log all policy decisions with reasoning', status: 'active', risk: 'low' },
];

export function PolicyEditor() {
  return (
    <div className="space-y-6 max-w-5xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-2xl font-bold text-text-primary">Policy Editor</h1>
          <p className="text-text-secondary">Manage OPA/Rego policies for intent routing, privacy, and security</p>
        </div>
        <Button variant="primary">
          <Plus size={16} className="mr-2" />
          Create Policy
        </Button>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Built-in Policies</h2>
            <div className="space-y-3">
              {builtinPolicies.map(policy => (
                <div key={policy.id} className="p-4 border border-surface-border/50 rounded-lg flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className={`p-2 rounded-lg ${policy.status === 'active' ? 'bg-system-local/20' : 'bg-text-muted/20'}`}>
                      <Shield className={`text-system-local`} size={20} />
                    </div>
                    <div>
                      <h3 className="font-medium text-text-primary">{policy.name}</h3>
                      <p className="text-sm text-text-muted">{policy.description}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                      policy.risk === 'critical' ? 'bg-risk-critical/20 text-risk-critical' :
                      policy.risk === 'high' ? 'bg-risk-high/20 text-risk-high' :
                      policy.risk === 'medium' ? 'bg-risk-medium/20 text-risk-medium' :
                      'bg-risk-low/20 text-risk-low'
                    }`}>
                      {policy.risk}
                    </span>
                    <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                      policy.status === 'active' ? 'bg-system-local/20 text-system-local' :
                      'bg-text-muted/20 text-text-muted'
                    }`}>
                      {policy.status}
                    </span>
                    <Button variant="ghost" size="sm" title="Edit"><Edit size={14} /></Button>
                    <Button variant="ghost" size="sm" title="Delete"><Trash2 size={14} /></Button>
                  </div>
                </div>
              ))}
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Policy Simulation</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-2">Test Input (JSON)</label>
                <Textarea
                  placeholder='{
  "actor": "user",
  "intent_id": "01HXK3JQ9V...",
  "action": "cloud_route",
  "resource": "anthropic"
}'
                  rows={8}
                  className="font-mono"
                />
              </div>
              <div className="flex gap-2">
                <Button variant="primary">
                  <Play size={16} className="mr-2" />
                  Evaluate All Policies
                </Button>
                <Button variant="secondary">Export Decisions</Button>
              </div>
              
              <div className="bg-surface-base border border-surface-border rounded-lg p-4">
                <h3 className="font-medium text-text-primary mb-3">Simulation Results</h3>
                <div className="space-y-2 text-sm font-mono">
                  <div className="flex items-center justify-between p-2 bg-system-local/10 rounded">
                    <span>air-gap</span>
                    <span className="text-system-local font-medium">DENY</span>
                    <span className="text-text-muted">cloud routing blocked</span>
                  </div>
                  <div className="flex items-center justify-between p-2 bg-system-local/10 rounded">
                    <span>pii-redaction</span>
                    <span className="text-system-local font-medium">ALLOW</span>
                    <span className="text-text-muted">no PII detected</span>
                  </div>
                  <div className="flex items-center justify-between p-2 bg-system-local/10 rounded">
                    <span>capability-authz</span>
                    <span className="text-system-local font-medium">ALLOW</span>
                    <span className="text-text-muted">token valid</span>
                  </div>
                  <div className="flex items-center justify-between p-2 bg-risk-critical/10 rounded">
                    <span>intent-approval</span>
                    <span className="text-risk-critical font-medium">REQUIRE_APPROVAL</span>
                    <span className="text-text-muted">risk_score=0.68 > 0.5</span>
                  </div>
                </div>
              </div>
            </div>
          </Card>
        </div>
        
        <div className="space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Custom Policy</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">Policy ID</label>
                <Input placeholder="custom-policy-name" />
              </div>
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">Description</label>
                <Input placeholder="What does this policy do?" />
              </div>
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">Rego Source</label>
                <Textarea
                  placeholder='package imperium.custom

default allow = false

allow {
  input.action == "local_only"
  not input.force_cloud
}

deny {
  input.action == "cloud_route"
  not input.approved
}'
                  rows={15}
                  className="font-mono"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1">Test Data (JSON)</label>
                <Textarea
                  placeholder='{"action": "cloud_route", "approved": false}'
                  rows={5}
                  className="font-mono"
                />
              </div>
              <div className="flex gap-2">
                <Button variant="primary">Validate & Save</Button>
                <Button variant="secondary">Test</Button>
              </div>
            </div>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Policy Bundle</h2>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between">
                <span className="text-text-muted">Version</span>
                <span className="font-mono">v2.1.0</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Policies</span>
                <span className="font-mono">7 built-in + 0 custom</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Last Updated</span>
                <span className="font-mono">2024-01-15 10:30 UTC</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Hash</span>
                <span className="font-mono text-xs">a1b2c3d4...</span>
              </div>
              <div className="border-t border-surface-border pt-3 flex gap-2">
                <Button variant="secondary" className="flex-1">
                  <Download size={16} className="mr-2" />
                  Export Bundle
                </Button>
                <Button variant="secondary" className="flex-1">
                  <Download size={16} className="mr-2" />
                  Export SBOM
                </Button>
              </div>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}