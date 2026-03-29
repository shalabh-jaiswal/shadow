import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useState, useEffect } from 'react';
import { Layout } from './components/layout/Layout';
import { Dashboard } from './components/screens/Dashboard';
import { Folders } from './components/screens/Folders';
import { Providers } from './components/screens/Providers';
import { Settings } from './components/screens/Settings';
import { useFoldersStore } from './store/foldersStore';
import { useProviderStore } from './store/providerStore';
export default function App() {
    const [screen, setScreen] = useState('dashboard');
    const fetchFolders = useFoldersStore((s) => s.fetchFolders);
    const loadProviders = useProviderStore((s) => s.load);
    useEffect(() => {
        fetchFolders();
        loadProviders();
    }, [fetchFolders, loadProviders]);
    return (_jsxs(Layout, { current: screen, onNavigate: setScreen, children: [screen === 'dashboard' && _jsx(Dashboard, {}), screen === 'folders' && _jsx(Folders, {}), screen === 'providers' && _jsx(Providers, {}), screen === 'settings' && _jsx(Settings, {})] }));
}
