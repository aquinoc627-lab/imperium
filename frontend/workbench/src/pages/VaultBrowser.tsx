import { Card } from '@/ui/components/Card';
import { Input } from '@/ui/components/Input';
import { Button } from '@/ui/components/Button';
import { Search, Folder, FileText, Plus, Filter } from 'lucide-react';

export function VaultBrowser() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-2xl font-bold text-text-primary">Vault Browser</h1>
          <p className="text-text-secondary">Search, explore, and manage your knowledge vault</p>
        </div>
        <Button variant="primary">
          <Plus size={16} className="mr-2" />
          New Note
        </Button>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <div className="lg:col-span-3 space-y-4">
          <Card>
            <div className="flex items-center gap-3 mb-4">
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" size={18} />
                <Input
                  placeholder="Search vault... (semantic search enabled)"
                  className="pl-10"
                />
              </div>
              <Filter className="p-2 hover:bg-surface-overlay rounded-lg text-text-secondary" size={20} />
            </div>
            
            <div className="space-y-2 max-h-[600px] overflow-y-auto">
              {[
                { title: 'Project Alpha - Architecture', tags: ['architecture', 'decisions'], updated: '2h ago' },
                { title: 'Passkey Migration Plan', tags: ['migration', 'auth', 'security'], updated: '1d ago' },
                { title: 'Daily Standup - 2024-01-15', tags: ['standup', 'team'], updated: '3d ago' },
                { title: 'API Design Guidelines', tags: ['api', 'guidelines'], updated: '1w ago' },
              ].map((note, i) => (
                <div key={i} className="p-3 hover:bg-surface-overlay rounded-lg border border-surface-border/50 cursor-pointer transition-colors">
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex-1 min-w-0">
                      <h4 className="font-medium text-text-primary truncate">{note.title}</h4>
                      <div className="flex items-center gap-2 mt-1 flex-wrap">
                        {note.tags.map(tag => (
                          <span key={tag} className="text-xs px-2 py-0.5 bg-surface-overlay text-text-muted rounded">
                            #{tag}
                          </span>
                        ))}
                      </div>
                    </div>
                    <span className="text-xs text-text-muted whitespace-nowrap">{note.updated}</span>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </div>
        
        <div className="space-y-4">
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Folders</h2>
            <ul className="space-y-1">
              {['Projects', 'Personal', 'Team', 'Archive'].map(folder => (
                <li key={folder} className="flex items-center gap-2 p-2 hover:bg-surface-overlay rounded cursor-pointer">
                  <Folder className="text-text-muted" size={16} />
                  <span className="text-sm text-text-secondary">{folder}</span>
                </li>
              ))}
            </ul>
          </Card>
          
          <Card>
            <h2 className="font-medium text-text-primary mb-4">Quick Stats</h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between">
                <dt className="text-text-muted">Total Notes</dt>
                <dd className="font-mono text-text-primary">1,247</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-text-muted">Indexed</dt>
                <dd className="font-mono text-system-local">1,247</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-text-muted">Graph Nodes</dt>
                <dd className="font-mono">3,891</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-text-muted">Git Commits</dt>
                <dd className="font-mono">42</dd>
              </div>
            </dl>
          </Card>
        </div>
      </div>
    </div>
  );
}