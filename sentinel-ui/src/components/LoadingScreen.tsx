import { useState, useEffect } from 'react';
import './LoadingScreen.css';

interface LoadingScreenProps {
  isVisible: boolean;
  status: string;
}

export function LoadingScreen({ isVisible, status }: LoadingScreenProps) {
  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState(0);

  const phases = [
    'Initializing neural pathways',
    'Loading knowledge vault',
    'Connecting to Ollama runtime',
    'Calibrating AI engine',
    'Preparing interface'
  ];

  useEffect(() => {
    if (!isVisible) return;

    // Smooth progress animation
    const progressInterval = setInterval(() => {
      setProgress(prev => {
        if (prev >= 100) return 100;
        return prev + Math.random() * 3;
      });
    }, 100);

    // Phase transitions
    const phaseInterval = setInterval(() => {
      setPhase(prev => (prev + 1) % phases.length);
    }, 2000);

    return () => {
      clearInterval(progressInterval);
      clearInterval(phaseInterval);
    };
  }, [isVisible]);

  if (!isVisible) return null;

  return (
    <div className="loading-screen-cinematic">
      <div className="loading-particles">
        {[...Array(30)].map((_, i) => (
          <div 
            key={i} 
            className="particle" 
            style={{
              left: `${Math.random() * 100}%`,
              top: `${Math.random() * 100}%`,
              animationDelay: `${Math.random() * 3}s`,
              animationDuration: `${3 + Math.random() * 4}s`
            }}
          />
        ))}
      </div>

      <div className="loading-container-cinematic">
        <div className="loading-logo-cinematic">
          <svg width="150" height="150" viewBox="0 0 150 150" fill="none" xmlns="http://www.w3.org/2000/svg">
            <defs>
              <linearGradient id="gradientRing" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#667eea" />
                <stop offset="50%" stopColor="#764ba2" />
                <stop offset="100%" stopColor="#f093fb" />
              </linearGradient>
              <filter id="glow">
                <feGaussianBlur stdDeviation="4" result="coloredBlur"/>
                <feMerge>
                  <feMergeNode in="coloredBlur"/>
                  <feMergeNode in="SourceGraphic"/>
                </feMerge>
              </filter>
            </defs>
            
            {/* Outer rotating ring */}
            <circle 
              cx="75" 
              cy="75" 
              r="65" 
              stroke="url(#gradientRing)" 
              strokeWidth="3" 
              fill="none" 
              strokeDasharray="20 10"
              filter="url(#glow)"
              className="rotating-ring"
            />
            
            {/* Inner pulsing ring */}
            <circle 
              cx="75" 
              cy="75" 
              r="50" 
              stroke="url(#gradientRing)" 
              strokeWidth="2" 
              fill="none" 
              opacity="0.6"
              className="pulsing-ring"
            />
            
            {/* Neural nodes */}
            <circle cx="75" cy="25" r="8" fill="#667eea" className="node-pulse" />
            <circle cx="110" cy="50" r="8" fill="#764ba2" className="node-pulse" style={{animationDelay: '0.2s'}} />
            <circle cx="110" cy="100" r="8" fill="#f093fb" className="node-pulse" style={{animationDelay: '0.4s'}} />
            <circle cx="75" cy="125" r="8" fill="#667eea" className="node-pulse" style={{animationDelay: '0.6s'}} />
            <circle cx="40" cy="100" r="8" fill="#764ba2" className="node-pulse" style={{animationDelay: '0.8s'}} />
            <circle cx="40" cy="50" r="8" fill="#f093fb" className="node-pulse" style={{animationDelay: '1s'}} />
            
            {/* Center core */}
            <circle cx="75" cy="75" r="12" fill="url(#gradientRing)" className="core-pulse" />
            <circle cx="75" cy="75" r="6" fill="#fff" opacity="0.9" />
          </svg>
        </div>

        <h1 className="loading-title-cinematic">
          <span className="title-gradient">SENTINEL</span>
        </h1>

        <p className="loading-tagline">Next-Generation AI Platform</p>

        <div className="loading-phase-text">{phases[phase]}</div>

        <div className="loading-progress-container">
          <div className="loading-progress-bar">
            <div 
              className="progress-fill-cinematic"
              style={{ width: `${Math.min(progress, 100)}%` }}
            />
          </div>
          <div className="progress-percentage">{Math.floor(Math.min(progress, 100))}%</div>
        </div>

        <div className="loading-status-text">{status}</div>

        <div className="loading-system-info">
          <div className="sys-info-item">
            <div className="sys-dot"></div>
            <span>Neural Engine</span>
          </div>
          <div className="sys-info-item">
            <div className="sys-dot"></div>
            <span>Memory Vault</span>
          </div>
          <div className="sys-info-item">
            <div className="sys-dot"></div>
            <span>Ollama Runtime</span>
          </div>
        </div>
      </div>
    </div>
  );
}
