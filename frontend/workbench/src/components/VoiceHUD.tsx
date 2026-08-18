import { Mic, MicOff, Volume2, X } from 'lucide-react';
import { useState } from 'react';

export function VoiceHUD() {
  const [listening, setListening] = useState(false);
  const [transcript, setTranscript] = useState('');
  
  return (
    <div className="fixed bottom-16 right-4 z-50">
      <div className="bg-surface-raised border border-surface-border rounded-xl p-3 w-80 shadow-xl animate-slideIn">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <button
              onClick={() => setListening(!listening)}
              className={`p-2 rounded-lg transition-colors ${listening 
                ? 'bg-intent-simulating/20 text-intent-simulating' 
                : 'hover:bg-surface-overlay text-text-secondary'
              }`}
              aria-label={listening ? 'Stop listening' : 'Start listening'}
            >
              {listening ? <MicOff size={20} /> : <Mic size={20} />}
            </button>
            <span className="text-sm font-medium text-text-primary">
              {listening ? 'Listening...' : 'Voice HUD'}
            </span>
            {listening && (
              <span className="text-xs text-intent-simulating animate-pulse">● REC</span>
            )}
          </div>
          <button 
            className="p-1 hover:bg-surface-overlay rounded text-text-muted"
            onClick={() => setTranscript('')}
          >
            <X size={16} />
          </button>
        </div>
        
        {transcript && (
          <div className="text-sm text-text-secondary mb-2 p-2 bg-surface-base rounded">
            {transcript}
          </div>
        )}
        
        <div className="flex items-center gap-2 text-xs text-text-muted">
          <Volume2 size={12} />
          <input type="range" min="0" max="100" defaultValue={80} className="flex-1 accent-intent-simulating" />
          <select className="text-xs bg-surface-base border border-surface-border rounded px-1 py-0.5 text-text-primary" defaultValue="1.1">
            <option value="1.0">1.0x</option>
            <option value="1.1">1.1x</option>
            <option value="1.25">1.25x</option>
            <option value="1.5">1.5x</option>
          </select>
        </div>
      </div>
    </div>
  );
}