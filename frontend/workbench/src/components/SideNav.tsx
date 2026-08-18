import { NavLink } from 'react-router-dom';
import { 
  Target, 
  BarChart3, 
  Brain, 
  Wrench, 
  Package, 
  Mic, 
  Settings,
  ChevronLeft 
} from 'lucide-react';
import { useState } from 'react';

const navItems = [
  { path: '/', label: 'Dashboard', icon: Target },
  { path: '/intent/new', label: 'Intents', icon: Target },
  { path: '/simulate', label: 'Simulate', icon: BarChart3 },
  { path: '/world', label: 'World Model', icon: Brain },
  { path: '/vault', label: 'Memory', icon: Package },
  { path: '/tools', label: 'Tools', icon: Wrench },
  { path: '/voice', label: 'Voice', icon: Mic },
  { path: '/policy', label: 'Policy', icon: Settings },
];

export function SideNav() {
  const [collapsed, setCollapsed] = useState(false);
  
  return (
    <aside className={`${collapsed ? 'w-16' : 'w-64'} h-full bg-surface-raised border-r border-surface-border flex flex-col transition-all duration-200`}>
      <div className="p-4 border-b border-surface-border">
        {!collapsed && (
          <h1 className="font-display font-bold text-xl text-text-primary">IMPERIUM</h1>
        )}
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="mt-2 w-full flex justify-center p-2 rounded-lg hover:bg-surface-overlay"
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          <ChevronLeft className={`${collapsed ? 'rotate-180' : ''} transition-transform`} size={20} />
        </button>
      </div>
      
      <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
        {navItems.map(({ path, label, icon: Icon }) => (
          <NavLink
            key={path}
            to={path}
            className={({ isActive }) => `
              flex items-center gap-3 px-3 py-2 rounded-lg transition-colors
              ${isActive 
                ? 'bg-intent-simulating/20 text-intent-simulating' 
                : 'text-text-secondary hover:bg-surface-overlay hover:text-text-primary'
              }
              ${collapsed ? 'justify-center' : ''}
            `}
            title={collapsed ? label : undefined}
          >
            <Icon size={20} aria-hidden="true" />
            {!collapsed && <span>{label}</span>}
          </NavLink>
        ))}
      </nav>
      
      <div className="p-4 border-t border-surface-border">
        {!collapsed && (
          <div className="text-xs text-text-muted">
            v0.1.0-dev • Local • Sovereign
          </div>
        )}
      </div>
    </aside>
  );
}