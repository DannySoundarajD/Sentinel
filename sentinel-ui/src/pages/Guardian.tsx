import React, { useEffect, useState } from 'react';
import { fetchGuardianStatus } from '../api';
import { Activity, Cpu, HardDrive, MemoryStick } from 'lucide-react';
import './Guardian.css';

export const Guardian: React.FC = () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [status, setStatus] = useState<any>(null);

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        const data = await fetchGuardianStatus();
        if (mounted) setStatus(data);
      } catch (err) {
        console.error('Failed to fetch guardian status', err);
      }
    };
    load();
    const interval = setInterval(load, 2000);
    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  if (!status) {
    return <div className="page-content loading">Loading Guardian...</div>;
  }

  return (
    <div className="page-content guardian-page">
      <h2 className="page-title"><Activity className="icon" /> Guardian</h2>
      
      <div className="metrics-grid">
        <div className="metric-card">
          <div className="metric-header">
            <Cpu size={18} /> CPU Usage
          </div>
          <div className="metric-value">{status.cpu_pct?.toFixed(1) ?? 0}%</div>
          <div className="metric-sub">{status.cpu_temp_c}°C</div>
        </div>
        
        <div className="metric-card">
          <div className="metric-header">
            <MemoryStick size={18} /> RAM Usage
          </div>
          <div className="metric-value">{status.ram_pct?.toFixed(1) ?? 0}%</div>
          <div className="metric-sub">{status.ram_used_mb} MB / {status.ram_total_mb} MB</div>
        </div>

        <div className="metric-card">
          <div className="metric-header">
            <HardDrive size={18} /> GPU Usage
          </div>
          <div className="metric-value">{status.gpu_pct ?? 'N/A'}</div>
          <div className="metric-sub">{status.vram_used_mb ? `${status.vram_used_mb} MB` : 'Integrated / None'}</div>
        </div>
      </div>

      <div className="process-list">
        <h3>Top Memory Consumers</h3>
        <table className="process-table">
          <thead>
            <tr>
              <th>Process Name</th>
              <th>PID</th>
              <th className="align-right">Memory</th>
            </tr>
          </thead>
          <tbody>
            {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
            {status.top_processes.map((p: any) => (
              <tr key={p.pid}>
                <td>{p.name}</td>
                <td className="text-muted">{p.pid}</td>
                <td className="align-right">{p.ram_mb} MB</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
