import React, { useState, useEffect, useCallback } from 'react';
import { TopBar } from './components/layout/TopBar';
import { Sidebar } from './components/layout/Sidebar';
import { ContextPanel } from './components/layout/ContextPanel';
import { Chat } from './pages/Chat';
import { Vault } from './pages/Vault';
import { Models } from './pages/Models';
import { Settings } from './pages/Settings';
import { fetchRuntimeStatus, fetchHealth, fetchModels, switchModel } from './api';
import './App.css';

type Theme = 'light' | 'dark';

function App() {
  const [activePage, setActivePage] = useState('chat');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [runtimeStatus, setRuntimeStatus] = useState<any>(null);
  const [memoryMode, setMemoryMode] = useState<'Lite' | 'Pro'>('Lite');
  const [showContextPanel, setShowContextPanel] = useState(true);
  const [installedModels, setInstalledModels] = useState<any[]>([]);

  // Theme state — persisted in localStorage
  const [theme, setTheme] = useState<Theme>(() => {
    const saved = localStorage.getItem('sentinel-theme');
    return (saved === 'light' || saved === 'dark') ? saved : 'dark';
  });

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('sentinel-theme', theme);
  }, [theme]);

  const toggleTheme = () => setTheme(prev => prev === 'dark' ? 'light' : 'dark');

  const refreshStatus = useCallback(async () => {
    try {
      const [status, health, models] = await Promise.all([
        fetchRuntimeStatus(),
        fetchHealth(),
        fetchModels().catch(() => []),
      ]);
      setRuntimeStatus(status);
      setInstalledModels(models || []);
      if (health && health.vault) {
        setMemoryMode(health.vault.toLowerCase() === 'pro' ? 'Pro' : 'Lite');
      }
    } catch { /* backend might not be running yet */ }
  }, []);

  // Polling for runtime status and health
  useEffect(() => {
    refreshStatus();
    const interval = setInterval(refreshStatus, 5000);
    return () => clearInterval(interval);
  }, [refreshStatus]);

  const handleSwitchModel = async (name: string) => {
    try {
      await switchModel(name);
      const status = await fetchRuntimeStatus();
      setRuntimeStatus(status);
    } catch { /* ignore */ }
  };

  const renderPage = () => {
    switch (activePage) {
      case 'chat':     return (
        <Chat
          showContextPanel={showContextPanel}
          setShowContextPanel={setShowContextPanel}
          activeModel={modelName}
          installedModels={installedModels}
          onSwitchModel={handleSwitchModel}
          onStatusRefresh={refreshStatus}
        />
      );
      case 'vault':    return <Vault />;
      case 'models':   return <Models />;
      case 'settings': return <Settings theme={theme} onThemeChange={setTheme} />;
      default:         return (
        <Chat
          showContextPanel={showContextPanel}
          setShowContextPanel={setShowContextPanel}
          activeModel={modelName}
          installedModels={installedModels}
          onSwitchModel={handleSwitchModel}
          onStatusRefresh={refreshStatus}
        />
      );
    }
  };

  // Sidebar resize
  const [sidebarWidth, setSidebarWidth] = useState(220);
  const [isResizingSidebar, setIsResizingSidebar] = useState(false);

  const startResizingSidebar = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizingSidebar(true);
  };

  useEffect(() => {
    if (!isResizingSidebar) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = Math.min(Math.max(e.clientX, 140), 360);
      setSidebarWidth(newWidth);
    };

    const handleMouseUp = () => setIsResizingSidebar(false);

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingSidebar]);

  const modelName = runtimeStatus?.active_model || 'No Model';
  const contextUsage = runtimeStatus?.context_used || 0;
  const maxContext = runtimeStatus?.active_model_context || 4096;

  return (
    <div className={`app-container ${isResizingSidebar ? 'is-resizing' : ''}`}>
      <TopBar
        modelName={modelName}
        mode={memoryMode}
        theme={theme}
        onToggleTheme={toggleTheme}
      />

      <div className="main-layout">
        <Sidebar
          activePage={activePage}
          onNavigate={setActivePage}
          collapsed={sidebarCollapsed}
          onToggleCollapse={() => setSidebarCollapsed(prev => !prev)}
          width={sidebarWidth}
        />
        {!sidebarCollapsed && (
          <div
            className="sidebar-resize-handle"
            onMouseDown={startResizingSidebar}
          />
        )}

        <main className="content-area">
          {renderPage()}
        </main>

        {activePage === 'chat' && showContextPanel && (
          <ContextPanel
            modelName={modelName}
            contextUsage={contextUsage}
            maxContext={maxContext}
            memoryNodes={runtimeStatus?.memory_nodes_injected || 0}
            hasSummary={runtimeStatus?.has_summary || false}
            installedModels={installedModels}
            onSwitchModel={handleSwitchModel}
            onClose={() => setShowContextPanel(false)}
          />
        )}
      </div>
    </div>
  );
}

export default App;
