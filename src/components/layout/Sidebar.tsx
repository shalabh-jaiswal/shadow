type Screen = 'dashboard' | 'folders' | 'providers' | 'settings' | 'about';

interface SidebarProps {
  current: Screen;
  onNavigate: (screen: Screen) => void;
}

const navItems = [
  { id: 'dashboard' as const, label: 'Dashboard', icon: '📊' },
  { id: 'folders' as const, label: 'Folders', icon: '📁' },
  { id: 'providers' as const, label: 'Providers', icon: '☁️' },
  { id: 'settings' as const, label: 'Settings', icon: '⚙️' },
  { id: 'about' as const, label: 'About', icon: 'ℹ️' },
];

export function Sidebar({ current, onNavigate }: SidebarProps) {
  return (
    <div className="w-64 h-screen bg-gray-100 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700">
      <div className="p-4">
        <h1 className="text-xl font-bold text-gray-900 dark:text-white">
          Shadow
        </h1>
      </div>
      <nav className="mt-8">
        <ul className="space-y-1 px-2">
          {navItems.map((item) => (
            <li key={item.id}>
              <button
                onClick={() => onNavigate(item.id)}
                className={`w-full flex items-center gap-3 px-3 py-2 text-left text-sm font-medium rounded-md transition-colors ${
                  current === item.id
                    ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                    : 'text-gray-700 hover:bg-gray-200 dark:text-gray-300 dark:hover:bg-gray-800'
                }`}
              >
                <span className="text-base">{item.icon}</span>
                {item.label}
              </button>
            </li>
          ))}
        </ul>
      </nav>
    </div>
  );
}