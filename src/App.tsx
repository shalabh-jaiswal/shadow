import { useState, useEffect } from 'react';
import { Layout } from './components/layout/Layout';
import { Dashboard } from './components/screens/Dashboard';
import { Folders } from './components/screens/Folders';
import { Providers } from './components/screens/Providers';
import { Settings } from './components/screens/Settings';
import { useFoldersStore } from './store/foldersStore';
import { useProviderStore } from './store/providerStore';

type Screen = 'dashboard' | 'folders' | 'providers' | 'settings';

export default function App() {
  const [screen, setScreen] = useState<Screen>('dashboard');
  const fetchFolders = useFoldersStore((s) => s.fetchFolders);
  const loadProviders = useProviderStore((s) => s.load);

  useEffect(() => {
    fetchFolders();
    loadProviders();
  }, [fetchFolders, loadProviders]);

  return (
    <Layout current={screen} onNavigate={setScreen}>
      {screen === 'dashboard' && <Dashboard />}
      {screen === 'folders' && <Folders />}
      {screen === 'providers' && <Providers />}
      {screen === 'settings' && <Settings />}
    </Layout>
  );
}
