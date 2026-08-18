import { 
  Cpu, 
  HardDrive, 
  Shield, 
  Mic, 
  WifiOff, 
  CheckCircle,
  AlertTriangle 
} from 'lucide-react';

const statusItems = [
  { 
    label: 'Routing', 
    status: 'local-only', 
    icon: WifiOff, 
    color: 'text-system-local',
    tooltip: 'ALLOW_CLOUD_ROUTING=false'
  },
  { 
    label: 'Models', 
    status: '3/4 loaded', 
    icon: Cpu, 
    color: 'text-text-secondary',
    tooltip: 'gemma4:12b, qwen3.5:9b, qwen3.5:2b, qwen3-emb:4b'
  },
  { 
    label: 'Memory', 
    status: '12.3/16 GB', 
    icon: HardDrive, 
    color: 'text-text-secondary',
    tooltip: 'KV cache: 8.2GB, Embeddings: 3.1GB, OS: 1.0GB'
  },
  { 
    label: 'Privacy', 
    status: 'air-gap', 
    icon: Shield, 
    color: 'text-system-airgap',
    tooltip: 'REDACT_PII=true, zero cloud calls this session'
  },
  { 
    label: 'Voice', 
    status: 'active', 
    icon: Mic, 
    color: 'text-system-local',
    tooltip: 'Whisper: ready, Kokoro: ready, Wake word: listening'
  },
];

export function AmbientStatusBar() {
  return (
    <footer className="h-10 px-4 bg-surface-raised border-t border-surface-border flex items-center justify-between">
      <div className="flex items-center gap-4">
        {statusItems.map((item, i) => (
          <div key={i} className="flex items-center gap-2" title={item.tooltip}>
            <item.icon className={`${item.color} ${item.status === 'active' || item.status === 'local-only' || item.status === 'air-gap' ? 'text-lg' : ''}`} size={14} />
            {!item.status.includes('/') && (
              <span className="text-xs text-text-muted">{item.label}</span>
            )}
            <span className={`text-xs font-mono ${item.color}`}>{item.status}</span>
          </div>
        ))}
      </div>
      
      <div className="flex items-center gap-2">
        <button className="p-1.5 rounded hover:bg-surface-overlay text-text-secondary hover:text-text-primary" title="New Intent">
          <span className="text-lg">⌘N</span>
        </button>
        <button className="p-1.5 rounded hover:bg-surface-overlay text-text-secondary hover:text-text-primary" title="Simulate">
          <span className="text-lg">⌘S</span>
        </button>
        <button className="p-1.5 rounded hover:bg-surface-overlay text-text-secondary hover:text-text-primary" title="Memory">
          <span className="text-lg">⌘M</span>
        </button>
        <button className="p-1.5 rounded hover:bg-surface-overlay text-text-secondary hover:text-text-primary" title="Policy">
          <span className="text-lg">⌘P</span>
        </button>
      </div>
    </footer>
  );
}