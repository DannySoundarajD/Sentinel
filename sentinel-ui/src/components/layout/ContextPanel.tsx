import React from 'react';
import { X, Cpu, MemoryStick, Zap, BookOpen } from 'lucide-react';
import './ContextPanel.css';

interface ContextPanelProps {
  modelName: string;
  contextUsage: number;
  maxContext: number;
  memoryNodes: number;
  hasSummary: boolean;
  installedModels?: any[];
  onSwitchModel?: (name: string) => void;
  onClose?: () => void;
}

function formatTokens(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

function getBarColor(pct: number): string {
  if (pct > 85) return 'danger';
  if (pct > 60) return 'warning';
  return '';
}

export const ContextPanel: React.FC<ContextPanelProps> = ({
  modelName,
  contextUsage,
  maxContext,
  memoryNodes,
  hasSummary,
  installedModels = [],
  onSwitchModel,
  onClose
}) => {
  const usagePercentage = maxContext > 0 ? Math.min((contextUsage / maxContext) * 100, 100) : 0;
  const barColor = getBarColor(usagePercentage);
  const remaining = Math.max(maxContext - contextUsage, 0);

  return (
    <div className="context-panel">
      <div className="panel-header">
        <h3 className="panel-title">Context Window</h3>
        {onClose && (
          <button className="panel-close-btn" onClick={onClose} title="Hide Context Panel">
            <X size={14} />
          </button>
        )}
      </div>

      {/* Model selector */}
      <div className="panel-section">
        <div className="section-header model-section-header">
          <span className="section-label">
            <Cpu size={11} style={{ display: 'inline', marginRight: 4 }} />
            Model
          </span>
          {installedModels && installedModels.length > 0 ? (
            <select
              className="model-select"
              value={modelName}
              onChange={e => onSwitchModel?.(e.target.value)}
            >
              <option value="">No Model Selected</option>
              {installedModels.map(m => (
                <option key={m.name} value={m.name}>
                  {m.name}
                </option>
              ))}
            </select>
          ) : (
            <span className="section-value highlight">{modelName}</span>
          )}
        </div>
      </div>

      {/* Context usage */}
      <div className="panel-section">
        <div className="section-header">
          <span className="section-label">
            <MemoryStick size={11} style={{ display: 'inline', marginRight: 4 }} />
            Session Usage
          </span>
          <span className={`section-value ${barColor === 'danger' ? 'danger' : barColor === 'warning' ? 'warning-text' : ''}`}>
            {usagePercentage.toFixed(1)}%
          </span>
        </div>
        <div className="progress-bar-bg">
          <div
            className={`progress-bar-fill ${barColor}`}
            style={{ width: `${usagePercentage}%` }}
          />
        </div>
        <div className="context-token-row">
          <span className="context-token-used">{formatTokens(contextUsage)} used</span>
          <span className="context-token-sep">/</span>
          <span className="context-token-max">{formatTokens(maxContext)} max</span>
          <span className="context-token-remaining">({formatTokens(remaining)} free)</span>
        </div>
      </div>

      {/* Memory nodes */}
      <div className="panel-section">
        <div className="section-header">
          <span className="section-label">
            <Zap size={11} style={{ display: 'inline', marginRight: 4 }} />
            Memory Injected
          </span>
          <span className={`section-value ${memoryNodes > 0 ? 'highlight' : ''}`}>
            {memoryNodes} node{memoryNodes !== 1 ? 's' : ''}
          </span>
        </div>
      </div>

      {/* Summary status */}
      <div className="panel-section">
        <div className="section-header">
          <span className="section-label">
            <BookOpen size={11} style={{ display: 'inline', marginRight: 4 }} />
            Session Summary
          </span>
          <span className={`section-value ${hasSummary ? 'success' : ''}`}>
            {hasSummary ? 'Saved' : 'None'}
          </span>
        </div>
      </div>
    </div>
  );
};
