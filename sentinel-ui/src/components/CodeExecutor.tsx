import React, { useState } from 'react';
import { Play, Code2, Sparkles } from 'lucide-react';
import { executeCode, analyzeCode, formatCode } from '../api';
import './CodeExecutor.css';

interface CodeExecutorProps {
  onClose: () => void;
}

export const CodeExecutor: React.FC<CodeExecutorProps> = ({ onClose }) => {
  const [language, setLanguage] = useState('python');
  const [code, setCode] = useState('print("Hello, World!")');
  const [output, setOutput] = useState('');
  const [executing, setExecuting] = useState(false);
  const [mode, setMode] = useState<'execute' | 'analyze' | 'format'>('execute');

  const handleExecute = async () => {
    setExecuting(true);
    setOutput('');
    
    try {
      if (mode === 'execute') {
        const result = await executeCode(language, code);
        setOutput(result.output || 'No output');
      } else if (mode === 'analyze') {
        const result = await analyzeCode(language, code);
        setOutput(JSON.stringify(result, null, 2));
      } else if (mode === 'format') {
        const result = await formatCode(language, code);
        setCode(result.formatted || code);
        setOutput('Code formatted successfully!');
      }
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setExecuting(false);
    }
  };

  return (
    <div className="code-executor-overlay" onClick={onClose}>
      <div className="code-executor-panel" onClick={(e) => e.stopPropagation()}>
        <div className="code-executor-header">
          <h3>Code Executor</h3>
          <button className="close-btn" onClick={onClose}>×</button>
        </div>

        <div className="code-executor-toolbar">
          <select 
            value={language} 
            onChange={(e) => setLanguage(e.target.value)}
            className="language-select"
          >
            <option value="python">Python</option>
            <option value="javascript">JavaScript</option>
            <option value="rust">Rust</option>
            <option value="bash">Bash</option>
            <option value="html">HTML</option>
          </select>

          <div className="mode-buttons">
            <button 
              className={mode === 'execute' ? 'active' : ''} 
              onClick={() => setMode('execute')}
            >
              <Play size={14} /> Execute
            </button>
            <button 
              className={mode === 'analyze' ? 'active' : ''} 
              onClick={() => setMode('analyze')}
            >
              <Code2 size={14} /> Analyze
            </button>
            <button 
              className={mode === 'format' ? 'active' : ''} 
              onClick={() => setMode('format')}
            >
              <Sparkles size={14} /> Format
            </button>
          </div>

          <button 
            className="run-btn" 
            onClick={handleExecute}
            disabled={executing}
          >
            {executing ? 'Running...' : 'Run'}
          </button>
        </div>

        <div className="code-executor-body">
          <div className="code-input-section">
            <label>Code</label>
            <textarea
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder="Enter your code here..."
              spellCheck={false}
            />
          </div>

          <div className="code-output-section">
            <label>Output</label>
            <pre className="output-pre">{output || 'No output yet'}</pre>
          </div>
        </div>
      </div>
    </div>
  );
};
