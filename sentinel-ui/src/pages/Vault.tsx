import React, { useEffect, useState } from 'react';
import { Database, Search, Trash2, Download, Plus, Layers, Network, FileText } from 'lucide-react';
import { fetchVaultNodes, fetchVaultEdges, fetchVaultSummaries, searchVault, deleteVaultNode, saveMemory, exportVault, deleteVaultEdge, deleteVaultSummary } from '../api';
import './Vault.css';

interface VaultNode {
  id: number;
  name: string;
  type: string;
  description?: string;
}

interface VaultEdge {
  id: number;
  source_id: number;
  target_id: number;
  relation: string;
}

interface VaultSummary {
  id: number;
  title?: string;
  summary: string;
  timestamp: number;
}

export const Vault: React.FC = () => {
  const [nodes, setNodes] = useState<VaultNode[]>([]);
  const [edges, setEdges] = useState<VaultEdge[]>([]);
  const [summaries, setSummaries] = useState<VaultSummary[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<VaultNode[] | null>(null);
  const [saveContent, setSaveContent] = useState('');
  const [saving, setSaving] = useState(false);
  const [tab, setTab] = useState<'nodes' | 'summaries' | 'graph'>('nodes');

  const load = async () => {
    try {
      const [n, e, s] = await Promise.all([
        fetchVaultNodes(),
        fetchVaultEdges(),
        fetchVaultSummaries(),
      ]);
      setNodes(Array.isArray(n) ? n : []);
      setEdges(Array.isArray(e) ? e : []);
      setSummaries(Array.isArray(s) ? s : []);
    } catch {}
  };

  useEffect(() => { load(); }, []);

  const handleSearch = async () => {
    if (!searchQuery.trim()) { setSearchResults(null); return; }
    const res = await searchVault(searchQuery);
    setSearchResults(Array.isArray(res) ? res : []);
  };

  const handleDelete = async (id: number) => {
    await deleteVaultNode(id);
    setNodes(prev => prev.filter(n => n.id !== id));
  };

  const handleDeleteEdge = async (id: number) => {
    await deleteVaultEdge(id);
    setEdges(prev => prev.filter(e => e.id !== id));
  };

  const handleDeleteSummary = async (id: number) => {
    await deleteVaultSummary(id);
    setSummaries(prev => prev.filter(s => s.id !== id));
  };

  const handleSave = async () => {
    if (!saveContent.trim()) return;
    setSaving(true);
    await saveMemory(saveContent.trim());
    setSaveContent('');
    setSaving(false);
    load();
  };

  const handleExport = async () => {
    const data = await exportVault();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'vault-export.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  const displayNodes = searchResults !== null ? searchResults : nodes;

  return (
    <div className="page-content vault-page">
      <div className="vault-header">
        <h2 className="page-title"><Database size={20} className="title-icon" /> Vault Memory</h2>
        <button className="vault-export-btn" onClick={handleExport}>
          <Download size={14} /> Export
        </button>
      </div>

      {/* Stats Row */}
      <div className="vault-stats">
        <div className="stat-card">
          <Layers size={16} />
          <span className="stat-num">{nodes.length}</span>
          <span className="stat-lbl">Nodes</span>
        </div>
        <div className="stat-card">
          <Network size={16} />
          <span className="stat-num">{edges.length}</span>
          <span className="stat-lbl">Relationships</span>
        </div>
        <div className="stat-card">
          <FileText size={16} />
          <span className="stat-num">{summaries.length}</span>
          <span className="stat-lbl">Summaries</span>
        </div>
      </div>

      {/* Save Memory */}
      <div className="vault-save">
        <input
          className="vault-input"
          placeholder="/save Danny prefers dark mode..."
          value={saveContent}
          onChange={e => setSaveContent(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSave()}
        />
        <button className="vault-save-btn" onClick={handleSave} disabled={saving}>
          <Plus size={16} /> Save
        </button>
      </div>

      {/* Search */}
      <div className="vault-search">
        <Search size={14} className="search-icon" />
        <input
          className="vault-search-input"
          placeholder="Search memory nodes..."
          value={searchQuery}
          onChange={e => setSearchQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSearch()}
        />
        {searchResults && (
          <button className="clear-search" onClick={() => { setSearchResults(null); setSearchQuery(''); }}>✕</button>
        )}
      </div>

      {/* Tabs */}
      <div className="vault-tabs">
        {(['nodes', 'summaries', 'graph'] as const).map(t => (
          <button key={t} className={`vault-tab ${tab === t ? 'active' : ''}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {/* Content */}
      {tab === 'nodes' && (
        <div className="vault-list">
          {displayNodes.length === 0 ? (
            <div className="vault-empty">
              <Database size={32} />
              <p>No memory nodes yet. Use /save to store memories.</p>
            </div>
          ) : displayNodes.map(node => (
            <div key={node.id} className="vault-node-card">
              <div className="node-type-badge">{node.type}</div>
              <div className="node-content">
                <div className="node-name">{node.name}</div>
                {node.description && <div className="node-desc">{node.description}</div>}
              </div>
              <button className="vault-delete-btn" onClick={() => handleDelete(node.id)}>
                <Trash2 size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      {tab === 'summaries' && (
        <div className="vault-list">
          {summaries.length === 0 ? (
            <div className="vault-empty">
              <FileText size={32} />
              <p>No conversation summaries yet.</p>
            </div>
          ) : summaries.map(s => (
            <div key={s.id} className="vault-summary-card">
              <div className="summary-title">{s.title || 'Conversation Summary'}</div>
              <div className="summary-text">{s.summary}</div>
              <div className="summary-time">{new Date(s.timestamp * 1000).toLocaleDateString()}</div>
              <button className="vault-delete-btn" onClick={() => handleDeleteSummary(s.id)}>
                <Trash2 size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      {tab === 'graph' && (
        <div className="vault-graph">
          {nodes.length === 0 ? (
            <div className="vault-empty">
              <Network size={32} />
              <p>No graph data yet.</p>
            </div>
          ) : (
            <div className="graph-view">
              {nodes.map(node => {
                const nodeEdges = edges.filter(e => e.source_id === node.id);
                return (
                  <div key={node.id} className="graph-node">
                    <div className="graph-node-label">{node.name}</div>
                    {nodeEdges.length > 0 && (
                      <div className="graph-edges">
                        {nodeEdges.map(edge => {
                          const target = nodes.find(n => n.id === edge.target_id);
                          return target ? (
                            <div key={edge.id} className="graph-edge">
                              <span className="edge-relation">{edge.relation}</span>
                              <span className="edge-arrow">→</span>
                              <span className="edge-target">{target.name}</span>
                              <button className="edge-delete-btn" onClick={() => handleDeleteEdge(edge.id)}>
                                <Trash2 size={12} />
                              </button>
                            </div>
                          ) : null;
                        })}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
