import { Sidebar } from './Sidebar';

type Screen = 'dashboard' | 'folders' | 'providers' | 'settings' | 'about';

interface LayoutProps {
  current: Screen;
  onNavigate: (screen: Screen) => void;
  children: React.ReactNode;
}

export function Layout({ current, onNavigate, children }: LayoutProps) {
  return (
    <div className="flex h-screen bg-white dark:bg-gray-900">
      <Sidebar current={current} onNavigate={onNavigate} />
      <main className="flex-1 overflow-y-auto">
        <div className="p-6">
          {children}
        </div>
      </main>
    </div>
  );
}