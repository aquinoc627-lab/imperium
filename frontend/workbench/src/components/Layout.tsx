import { Outlet } from 'react-router-dom';
import { SideNav } from './SideNav';
import { AmbientStatusBar } from './AmbientStatusBar';
import { VoiceHUD } from './VoiceHUD';

export function Layout() {
  return (
    <div className="h-screen w-screen flex flex-col bg-surface-base text-text-primary">
      <div className="flex-1 flex overflow-hidden">
        <SideNav />
        <main className="flex-1 overflow-auto p-4">
          <Outlet />
        </main>
      </div>
      <AmbientStatusBar />
      <VoiceHUD />
    </div>
  );
}