import React, { useState, useEffect, useCallback } from 'react';
import { TopBar } from './components/layout/TopBar';
import { Sidebar } from './components/layout/Sidebar';
import { LoadingScreen } from './components/LoadingScreen';
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
  const [isLoading, setIsLoading] = useState(true);
  const [loadingStatus, setLoadingStatus] = useState('Initializing...');

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
      setLoadingStatus('Connecting to daemon...');
      const [status, health, models] = await Promise.all([
        fetchRuntimeStatus(),
        fetchHealth(),
        fetchModels().catch(() => []),
      ]);
      setLoadingStatus('Checking Ollama connection...');
      setRuntimeStatus(status);
      setInstalledModels(models || []);
      if (health && health.vault) {
        setMemoryMode(health.vault.toLowerCase() === 'pro' ? 'Pro' : 'Lite');
      }
      setLoadingStatus('Loading interface...');
      // Give a brief moment to show the loading message before hiding
      setTimeout(() => setIsLoading(false), 500);
    } catch (error) {
      setLoadingStatus('Waiting for daemon to start...');
      console.error('Failed to fetch status:', error);
    }
  }, []);

  // Initial load and polling for runtime status and health
  useEffect(() => {
    const loadApp = async () => {
      // Retry logic with progressive backoff
      let retries = 0;
      const maxRetries = 60; // 60 seconds total
      
      while (retries < maxRetries && isLoading) {
        try {
          setLoadingStatus(`Initializing (attempt ${retries + 1}/${maxRetries})...`);
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
          
          setLoadingStatus('Ready!');
          setTimeout(() => setIsLoading(false), 300);
          break;
        } catch (error) {
          retries++;
          if (retries < maxRetries) {
            setLoadingStatus(`Waiting for Ollama to be ready... (${retries}/${maxRetries})`);
            await new Promise(resolve => setTimeout(resolve, 1000));
          } else {
            console.error('Failed to load after max retries:', error);
            setLoadingStatus('Failed to connect. Check if Ollama is running.');
            break;
          }
        }
      }
    };

    loadApp();
  }, []);

  // Polling for runtime status (after initial load)
  useEffect(() => {
    if (isLoading) return;

    const interval = setInterval(refreshStatus, 5000);
    return () => clearInterval(interval);
  }, [refreshStatus, isLoading]);

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

  // Show loading screen while initializing
  if (isLoading) {
    return <LoadingScreen isVisible={isLoading} status={loadingStatus} />;
  }

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
      </div>
    </div>
  );
}

export default App;
