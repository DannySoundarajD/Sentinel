import React from 'react';
import { Sun, Moon } from 'lucide-react';
import './TopBar.css';

interface TopBarProps {
  modelName: string;
  mode: 'Lite' | 'Pro';
  theme: 'light' | 'dark';
  onToggleTheme: () => void;
}

export const TopBar: React.FC<TopBarProps> = ({ modelName, mode, theme, onToggleTheme }) => {
  return (
    <div className="top-bar drag-region">
      <div className="top-bar-left">
        <div className="window-controls no-drag">
          <button className="win-btn close" onClick={() => (window as any).electronAPI?.close()} title="Close" />
          <button className="win-btn maximize" onClick={() => (window as any).electronAPI?.maximize()} title="Zoom" />
          <button className="win-btn minimize" onClick={() => (window as any).electronAPI?.minimize()} title="Minimize" />
        </div>
        <span className="logo">⬡ Sentinel</span>
      </div>

      <div className="top-bar-center">
        <span className="model-badge">{modelName}</span>
      </div>

      <div className="top-bar-right no-drag">
        <span className={`mode-badge ${mode.toLowerCase()}`}>{mode}</span>
        <button className="theme-toggle" onClick={onToggleTheme} title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`}>
          {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
        </button>
      </div>
    </div>
  );
};
