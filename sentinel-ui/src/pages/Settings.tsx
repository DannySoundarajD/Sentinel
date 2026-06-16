import React, { useEffect, useState } from 'react';
import { Settings as SettingsIcon, Save, Key, Cpu, Shield, ToggleLeft, Moon, Sun, Info } from 'lucide-react';
import { fetchSettings, updateSettings, fetchModels } from '../api';
import './Settings.css';

interface SettingsData {
  vault_mode?: string;
  memory_mode?: string;
  default_model?: string;
  fallback_model?: string;
  ollama_host?: string;
  telegram_token?: string;
  telegram_chat_id?: string;
  context_window?: number;
  resource_profile?: string;
  telegram_enabled?: boolean;
}

interface Model {
  name: string;
}

interface SettingsProps {
  theme: 'light' | 'dark';
  onThemeChange: (theme: 'light' | 'dark') => void;
}

export const Settings: React.FC<SettingsProps> = ({ theme, onThemeChange }) => {
  const [settings, setSettings] = useState<SettingsData>({});
  const [models, setModels] = useState<Model[]>([]);
  const [saved, setSaved] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      fetchSettings(),
      fetchModels()
    ]).then(([settingsData, modelsData]) => {
      setSettings(settingsData || {});
      setModels(Array.isArray(modelsData) ? modelsData : []);
    }).catch(() => {}).finally(() => setLoading(false));
  }, []);

  const handleChange = (key: keyof SettingsData, value: string | number | boolean) => {
    setSettings(prev => ({ ...prev, [key]: value }));
  };

  const handleSave = async () => {
    const payload = {
      ollama_host: settings.ollama_host,
      default_model: settings.default_model,
      fallback_model: settings.fallback_model,
      memory_mode: settings.memory_mode,
      resource_profile: settings.resource_profile || 'balanced',
      telegram_token: settings.telegram_token,
      telegram_chat_id: settings.telegram_chat_id,
      telegram_enabled: settings.telegram_enabled
    };
    await updateSettings(payload);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  if (loading) {
    return (
      <div className="page-content loading">
        <div className="spinner-container">
          <SettingsIcon className="spinner-icon spin" size={32} />
          <p>Loading configuration settings...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="page-content settings-page">
      <div className="settings-header">
        <div className="title-area">
          <h2 className="page-title"><SettingsIcon size={20} className="title-icon" /> Settings</h2>
          <p className="page-subtitle">Configure runtime parameters, model selections, system resource allocations, and Telegram bridge integrations.</p>
        </div>
        <button className={`save-btn ${saved ? 'saved' : ''}`} onClick={handleSave}>
          <Save size={14} /> {saved ? 'Configuration Saved!' : 'Save Configuration'}
        </button>
      </div>

      {/* General Settings */}
      <section className="settings-section">
        <h3 className="settings-section-title"><Cpu size={14} /> General Runtime Options</h3>
        <div className="settings-group">
          <div className="setting-row">
            <label className="setting-label">
              Default Model
              <span className="setting-hint">Loaded automatically when launching Sentinel.</span>
            </label>
            <select
              className="setting-select wide"
              value={settings.default_model || ''}
              onChange={e => handleChange('default_model', e.target.value)}
            >
              <option value="" disabled>Select a default model...</option>
              {models.map(m => (
                <option key={m.name} value={m.name}>{m.name}</option>
              ))}
              {models.length === 0 && <option value="gemma:2b">gemma:2b (Not Installed)</option>}
            </select>
          </div>

          <div className="setting-row">
            <label className="setting-label">
              Fallback Model
              <span className="setting-hint">Used automatically if the default model fails or is missing.</span>
            </label>
            <select
              className="setting-select wide"
              value={settings.fallback_model || ''}
              onChange={e => handleChange('fallback_model', e.target.value)}
            >
              <option value="" disabled>Select a fallback model...</option>
              {models.map(m => (
                <option key={m.name} value={m.name}>{m.name}</option>
              ))}
              {models.length === 0 && <option value="gemma:2b">gemma:2b (Not Installed)</option>}
            </select>
          </div>

          <div className="setting-row">
            <label className="setting-label">
              Ollama Host Endpoint
              <span className="setting-hint">HTTP endpoint where local Ollama daemon is running.</span>
            </label>
            <input
              className="setting-input"
              value={settings.ollama_host || ''}
              onChange={e => handleChange('ollama_host', e.target.value)}
              placeholder="http://localhost:11434"
            />
          </div>

          <div className="setting-row">
            <label className="setting-label">
              Resource Optimization Profile
              <span className="setting-hint">Applies CPU/thread nice priorities to LLM daemon processes.</span>
            </label>
            <select
              className="setting-select wide"
              value={settings.resource_profile || 'balanced'}
              onChange={e => handleChange('resource_profile', e.target.value)}
            >
              <option value="lite">Lite (Low CPU priority, saves energy)</option>
              <option value="balanced">Balanced (Standard priority)</option>
              <option value="performance">Performance (Real-time threads, high usage)</option>
            </select>
          </div>
        </div>
      </section>

      {/* Memory Mode Selection */}
      <section className="settings-section">
        <h3 className="settings-section-title"><Shield size={14} /> Memory Vault Configuration</h3>
        <div className="settings-group">
          <div className="setting-row">
            <label className="setting-label">
              Vault Memory Mode
              <span className="setting-hint">Pro mode enables deep context generation using database vector indexing. Lite is passive recall.</span>
            </label>
            <select
              className="setting-select wide"
              value={settings.memory_mode || 'lite'}
              onChange={e => handleChange('memory_mode', e.target.value)}
            >
              <option value="lite">Lite Mode (Low Memory Overhead)</option>
              <option value="pro">Pro Mode (Dynamic Vector Knowledge Base)</option>
            </select>
          </div>

          <div className="setting-row info-row">
            <div className="info-box">
              <Info size={14} />
              <span>Current System Context Window limit is: <strong>{settings.context_window || 4096} tokens</strong> (dynamically scaled).</span>
            </div>
          </div>
        </div>
      </section>

      {/* Telegram Bridge Settings */}
      <section className="settings-section">
        <h3 className="settings-section-title"><Key size={14} /> Telegram API Bridge</h3>
        <div className="settings-group">
          <div className="setting-row">
            <label className="setting-label">
              Enable Telegram Bridge
              <span className="setting-hint">Run a background bot to query Sentinel from Telegram.</span>
            </label>
            <input
              type="checkbox"
              className="setting-checkbox"
              checked={!!settings.telegram_enabled}
              onChange={e => handleChange('telegram_enabled', e.target.checked)}
            />
          </div>

          {settings.telegram_enabled && (
            <>
              <div className="setting-row">
                <label className="setting-label">
                  Bot Father Token
                  <span className="setting-hint">Obtain token from @BotFather.</span>
                </label>
                <input
                  className="setting-input"
                  type="password"
                  value={settings.telegram_token || ''}
                  onChange={e => handleChange('telegram_token', e.target.value)}
                  placeholder="e.g. 123456789:ABCDefGhI..."
                />
              </div>

              <div className="setting-row">
                <label className="setting-label">
                  Authorized Chat ID
                  <span className="setting-hint">Whitelist specific Chat ID to prevent unauthorized access.</span>
                </label>
                <input
                  className="setting-input"
                  value={settings.telegram_chat_id || ''}
                  onChange={e => handleChange('telegram_chat_id', e.target.value)}
                  placeholder="e.g. 987654321"
                />
              </div>
            </>
          )}
        </div>
      </section>

      {/* Appearance Settings */}
      <section className="settings-section">
        <h3 className="settings-section-title"><ToggleLeft size={14} /> Appearance & Themes</h3>
        <div className="settings-group">
          <div className="setting-row">
            <label className="setting-label">
              Theme Style
              <span className="setting-hint">Switch between dark mode and light mode interfaces.</span>
            </label>
            <div className="theme-toggle-group">
              <button 
                className={`theme-btn ${theme === 'dark' ? 'active' : ''}`}
                onClick={() => onThemeChange('dark')}
              >
                <Moon size={14} /> Dark
              </button>
              <button 
                className={`theme-btn ${theme === 'light' ? 'active' : ''}`}
                onClick={() => onThemeChange('light')}
              >
                <Sun size={14} /> Light
              </button>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
};
