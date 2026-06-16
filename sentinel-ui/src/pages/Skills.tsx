import React, { useEffect, useState } from 'react';
import { Wrench, ToggleLeft, ToggleRight, Trash2, Package } from 'lucide-react';
import { fetchSkills, enableSkill, disableSkill, deleteSkill } from '../api';
import './Skills.css';

interface Skill {
  name: string;
  version: string;
  enabled: boolean;
  description?: string;
  permissions?: string[];
}

export const Skills: React.FC = () => {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    try {
      const data = await fetchSkills();
      setSkills(Array.isArray(data) ? data : []);
    } catch {}
    setLoading(false);
  };

  useEffect(() => { load(); }, []);

  const handleToggle = async (skill: Skill) => {
    if (skill.enabled) {
      await disableSkill(skill.name);
    } else {
      await enableSkill(skill.name);
    }
    setSkills(prev => prev.map(s => s.name === skill.name ? { ...s, enabled: !s.enabled } : s));
  };

  const handleDelete = async (name: string) => {
    await deleteSkill(name);
    setSkills(prev => prev.filter(s => s.name !== name));
  };

  if (loading) return <div className="page-content loading">Loading skills...</div>;

  return (
    <div className="page-content skills-page">
      <h2 className="page-title"><Wrench size={20} className="title-icon" /> Skills</h2>

      {skills.length === 0 ? (
        <div className="skills-empty">
          <Package size={48} />
          <p className="empty-title">No skills installed</p>
          <p className="empty-sub">Skills extend Sentinel with additional capabilities. Install via the Sentinel Store.</p>
        </div>
      ) : (
        <div className="skills-list">
          {skills.map(skill => (
            <div key={skill.name} className={`skill-card ${skill.enabled ? 'enabled' : 'disabled'}`}>
              <div className="skill-icon">
                <Package size={20} />
              </div>
              <div className="skill-info">
                <div className="skill-header">
                  <span className="skill-name">{skill.name}</span>
                  <span className="skill-version">v{skill.version}</span>
                </div>
                {skill.description && <p className="skill-desc">{skill.description}</p>}
                {skill.permissions && skill.permissions.length > 0 && (
                  <div className="skill-perms">
                    {skill.permissions.map(p => (
                      <span key={p} className="perm-tag">{p}</span>
                    ))}
                  </div>
                )}
              </div>
              <div className="skill-actions">
                <button
                  className={`toggle-btn ${skill.enabled ? 'on' : 'off'}`}
                  onClick={() => handleToggle(skill)}
                  title={skill.enabled ? 'Disable' : 'Enable'}
                >
                  {skill.enabled ? <ToggleRight size={24} /> : <ToggleLeft size={24} />}
                </button>
                <button className="skill-delete-btn" onClick={() => handleDelete(skill.name)}>
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
