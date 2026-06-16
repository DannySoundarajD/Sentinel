import React from 'react';
import { MessageSquare, Database, Cpu, Settings, Menu } from 'lucide-react';
import './Sidebar.css';

interface SidebarProps {
  activePage: string;
  onNavigate: (page: string) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  width?: number;
}

export const Sidebar: React.FC<SidebarProps> = ({ activePage, onNavigate, collapsed, onToggleCollapse, width }) => {
  const navItems = [
    { id: 'chat', icon: <MessageSquare size={20} />, label: 'Chat' },
    { id: 'vault', icon: <Database size={20} />, label: 'Vault' },
    { id: 'models', icon: <Cpu size={20} />, label: 'Models' },
  ];

  return (
    <div 
      className={`sidebar ${collapsed ? 'collapsed' : ''}`}
      style={!collapsed && width ? { width: `${width}px` } : undefined}
    >
      <div className="sidebar-nav">
        <button className="nav-item collapse-toggle" onClick={onToggleCollapse} title="Toggle Sidebar">
          <span className="nav-icon"><Menu size={20} /></span>
          {!collapsed && <span className="nav-label">Collapse Menu</span>}
        </button>
        <div style={{ height: '1px', background: 'var(--border)', margin: '4px 0' }} />

        {navItems.map(item => (
          <button
            key={item.id}
            className={`nav-item ${activePage === item.id ? 'active' : ''}`}
            onClick={() => onNavigate(item.id)}
            title={item.label}
          >
            <span className="nav-icon">{item.icon}</span>
            {!collapsed && <span className="nav-label">{item.label}</span>}
          </button>
        ))}
      </div>
      <div className="sidebar-bottom">
        <button
          className={`nav-item ${activePage === 'settings' ? 'active' : ''}`}
          onClick={() => onNavigate('settings')}
          title="Settings"
        >
          <span className="nav-icon"><Settings size={20} /></span>
          {!collapsed && <span className="nav-label">Settings</span>}
        </button>
      </div>
    </div>
  );
};
