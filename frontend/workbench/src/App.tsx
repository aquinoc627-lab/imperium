import { Routes, Route } from 'react-router-dom';
import { Dashboard } from './pages/Dashboard';
import { IntentWorkspace } from './pages/IntentWorkspace';
import { SimulationExplorer } from './pages/SimulationExplorer';
import { WorldModelInspector } from './pages/WorldModelInspector';
import { VaultBrowser } from './pages/VaultBrowser';
import { ToolsManager } from './pages/ToolsManager';
import { VoiceSettings } from './pages/VoiceSettings';
import { PolicyEditor } from './pages/PolicyEditor';
import { Layout } from './components/Layout';

export function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/intent/:id" element={<IntentWorkspace />} />
        <Route path="/simulate" element={<SimulationExplorer />} />
        <Route path="/world" element={<WorldModelInspector />} />
        <Route path="/vault" element={<VaultBrowser />} />
        <Route path="/tools" element={<ToolsManager />} />
        <Route path="/voice" element={<VoiceSettings />} />
        <Route path="/policy" element={<PolicyEditor />} />
      </Routes>
    </Layout>
  );
}