import { Card } from '@/ui/components/Card';
import { Button } from '@/ui/components/Button';
import { Input } from '@/ui/components/Input';
import { Select } from '@/ui/components/Select';
import { Mic, MicOff, Volume2, Download, CheckCircle, AlertCircle } from 'lucide-react';

export function VoiceSettings() {
  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="font-display text-2xl font-bold text-text-primary">Voice Settings</h1>
        <p className="text-text-secondary">Configure STT, TTS, and wake word detection</p>
      </div>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <h2 className="font-medium text-text-primary mb-4 flex items-center gap-2">
            <Mic className="text-intent-simulating" size={20} />
            Speech-to-Text (Whisper)
          </h2>
          
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-text-secondary mb-2">Model</label>
              <Select defaultValue="base.en" className="w-full">
                <option value="tiny.en">Tiny (39 MB, fastest)</option>
                <option value="base.en">Base (74 MB, balanced)</option>
                <option value="small.en">Small (244 MB, accurate)</option>
                <option value="medium.en">Medium (769 MB, very accurate)</option>
              </Select>
            </div>
            
            <div className="flex items-center gap-4">
              <div className="flex-1">
                <label className="block text-sm font-medium text-text-secondary mb-1">Language</label>
                <Select defaultValue="en" className="w-full">
                  <option value="en">English</option>
                  <option value="auto">Auto-detect</option>
                </Select>
              </div>
              <div className="flex-1">
                <label className="block text-sm font-medium text-text-secondary mb-1">Compute</label>
                <Select defaultValue="cpu" className="w-full">
                  <option value="cpu">CPU</option>
                  <option value="cuda">CUDA</option>
                  <option value="metal">Metal (macOS)</option>
                </Select>
              </div>
            </div>
            
            <div className="p-3 bg-surface-base border border-surface-border rounded-lg">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="p-2 bg-system-local/20 rounded">
                    <CheckCircle className="text-system-local" size={18} />
                  </div>
                  <span className="font-medium text-text-primary">Whisper.cpp Ready</span>
                </div>
                <Button variant="ghost" size="sm">Test STT</Button>
              </div>
            </div>
          </div>
        </Card>
        
        <Card>
          <h2 className="font-medium text-text-primary mb-4 flex items-center gap-2">
            <Volume2 className="text-intent-simulating" size={20} />
            Text-to-Speech (Kokoro)
          </h2>
          
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-text-secondary mb-2">Voice</label>
              <Select defaultValue="af" className="w-full">
                <option value="af">Aria (Female, US)</option>
                <option value="am">Adam (Male, US)</option>
                <option value="bf">Bella (Female, UK)</option>
                <option value="bm">Brian (Male, UK)</option>
              </Select>
            </div>
            
            <div className="flex items-center gap-4">
              <div className="flex-1">
                <label className="block text-sm font-medium text-text-secondary mb-1">Speed</label>
                <Input type="range" min="0.5" max="2" step="0.1" defaultValue="1.1" className="w-full" />
              </div>
              <div className="flex-1">
                <label className="block text-sm font-medium text-text-secondary mb-1">Volume</label>
                <Input type="range" min="0" max="1" step="0.1" defaultValue="0.8" className="w-full" />
              </div>
            </div>
            
            <div className="p-3 bg-surface-base border border-surface-border rounded-lg">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="p-2 bg-system-local/20 rounded">
                    <CheckCircle className="text-system-local" size={18} />
                  </div>
                  <span className="font-medium text-text-primary">Kokoro ONNX Ready</span>
                </div>
                <Button variant="ghost" size="sm">Test TTS</Button>
              </div>
            </div>
          </div>
        </Card>
        
        <Card>
          <h2 className="font-medium text-text-primary mb-4 flex items-center gap-2">
            <Mic className="text-intent-simulating" size={20} />
            Wake Word (Porcupine)
          </h2>
          
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-text-secondary mb-2">Wake Word</label>
              <Select defaultValue="hey-os" className="w-full">
                <option value="hey-os">Hey OS</option>
                <option value="computer">Computer</option>
                <option value="jarvis">Jarvis</option>
                <option value="custom">Custom (train your own)</option>
              </Select>
            </div>
            
            <div className="flex items-center gap-4">
              <div className="flex-1">
                <label className="block text-sm font-medium text-text-secondary mb-1">Sensitivity</label>
                <Input type="range" min="0.1" max="1" step="0.05" defaultValue="0.5" className="w-full" />
              </div>
              <div className="flex-1">
                <label className="block text-sm font-medium text-text-secondary mb-1">Endpoint Duration (ms)</label>
                <Input type="number" min="100" max="2000" step="100" defaultValue="800" className="w-full" />
              </div>
            </div>
            
            <div className="p-3 bg-surface-base border border-surface-border rounded-lg">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="p-2 bg-system-local/20 rounded">
                    <CheckCircle className="text-system-local" size={18} />
                  </div>
                  <span className="font-medium text-text-primary">Porcupine Ready</span>
                </div>
                <Button variant="ghost" size="sm" onClick={() => alert('Listening for "Hey OS"...')}>
                  <Mic size={16} className="mr-1" />
                  Test Wake Word
                </Button>
              </div>
            </div>
          </div>
        </Card>
        
        <Card>
          <h2 className="font-medium text-text-primary mb-4">Model Assets</h2>
          <div className="space-y-3 text-sm">
            <div className="flex items-center justify-between p-3 bg-surface-base border border-surface-border rounded">
              <div className="flex items-center gap-2">
                <Download className="text-text-muted" size={18} />
                <span className="font-medium">whisper-base.en.bin</span>
              </div>
              <span className="text-system-local text-sm">74 MB ✓</span>
            </div>
            <div className="flex items-center justify-between p-3 bg-surface-base border border-surface-border rounded">
              <div className="flex items-center gap-2">
                <Download className="text-text-muted" size={18} />
                <span className="font-medium">kokoro-v1.0.int8.onnx</span>
              </div>
              <span className="text-system-local text-sm">82 MB ✓</span>
            </div>
            <div className="flex items-center justify-between p-3 bg-surface-base border border-surface-border rounded">
              <div className="flex items-center gap-2">
                <Download className="text-text-muted" size={18} />
                <span className="font-medium">voices-v1.0.bin</span>
              </div>
              <span className="text-system-local text-sm">4.2 MB ✓</span>
            </div>
            <div className="flex items-center justify-between p-3 bg-surface-base border border-surface-border rounded">
              <div className="flex items-center gap-2">
                <Download className="text-text-muted" size={18} />
                <span className="font-medium">porcupine-hey-os.ppn</span>
              </div>
              <span className="text-system-local text-sm">18 KB ✓</span>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}