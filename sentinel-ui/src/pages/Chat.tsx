import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Send, Trash2, Copy, RefreshCw, PanelLeftOpen, PanelLeftClose, PanelRightOpen, PanelRightClose, Plus, AlertCircle, Code2 } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeRaw from 'rehype-raw';
import { sendChat, fetchChatHistory, fetchAllChatHistory, clearChatHistory, fetchVaultSummaries, fetchGuardianStatus, startNewSession, deleteVaultSummary, loadVaultSummary, fetchRuntimeStatus } from '../api';
import { ContextPanel } from '../components/layout/ContextPanel';
import { CodeExecutor } from '../components/CodeExecutor';
import './Chat.css';

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

interface Summary {
  id: number;
  title?: string;
  summary: string;
  timestamp: number;
  [key: string]: any;
}

interface ChatProps {
  showContextPanel: boolean;
  setShowContextPanel: (val: boolean) => void;
  activeModel: string;
  installedModels: { name: string; [key: string]: any }[];
  onSwitchModel: (name: string) => Promise<void>;
  onStatusRefresh?: () => void;
}

export const Chat: React.FC<ChatProps> = ({
  showContextPanel,
  setShowContextPanel,
  activeModel,
  installedModels,
  onSwitchModel,
  onStatusRefresh,
}) => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [streaming, setStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [summaries, setSummaries] = useState<Summary[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [activeSuggestion, setActiveSuggestion] = useState(0);
  const [showHistoryPane, setShowHistoryPane] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [contextUsage, setContextUsage] = useState(0);
  const [maxContext, setMaxContext] = useState(4096);
  const [memoryNodesCount, setMemoryNodesCount] = useState(0);
  const [hasSummary, setHasSummary] = useState(false);
  const [showCodeExecutor, setShowCodeExecutor] = useState(false);
  const isStartingSession = useRef(false);

  // History pane resize
  const [historyWidth, setHistoryWidth] = useState(240);
  const [isResizingHistory, setIsResizingHistory] = useState(false);
  const historyRef = useRef<HTMLDivElement>(null);

  const messagesAreaRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Scroll tracking state
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);

  const COMMANDS = [
    { name: '/new', desc: 'Start a fresh chat session' },
    { name: '/reset', desc: 'Clear all history and context' },
    { name: '/status', desc: 'Show CPU/RAM/GPU metrics' },
    { name: '/model', desc: 'Switch active Ollama model' },
    { name: '/save', desc: 'Save a key-value fact to memory Vault' },
    { name: '/frommemory', desc: 'Retrieve matching memories from Vault' },
    { name: '/memory', desc: 'List all manual memories in Vault' },
    { name: '/explain', desc: 'Explain code or a concept' },
    { name: '/fix', desc: 'Fix bugs in code or text' },
    { name: '/review', desc: 'Review code quality' },
    { name: '/doc', desc: 'Generate documentation' },
    { name: '/exec', desc: 'Execute a terminal command' },
    { name: '/help', desc: 'List all commands' },
  ];

  // ─── Data Loading ────────────────────────────────────────────
  const refreshHistory = useCallback(() => {
    // Fetch ALL chat history including Telegram messages
    fetchAllChatHistory()
      .then((data: Message[]) => {
        if (Array.isArray(data)) {
          setMessages(data);
        }
      })
      .catch(() => {
        // Fallback to regular chat history if vault endpoint fails
        fetchChatHistory()
          .then((data: Message[]) => {
            if (Array.isArray(data)) setMessages(data);
          })
          .catch(() => {});
      });
  }, []);

  const refreshSummaries = useCallback(() => {
    fetchVaultSummaries()
      .then((data) => {
        if (Array.isArray(data)) {
          // Sort by timestamp desc
          const sorted = [...data].sort((a, b) => b.timestamp - a.timestamp);
          setSummaries(sorted);
        }
      })
      .catch(() => {});
  }, []);

  // Estimate tokens from text (rough approximation: ~4 chars per token)
  const estimateTokens = useCallback((text: string): number => {
    return Math.ceil(text.length / 4);
  }, []);

  // Calculate real-time context usage based on current messages
  const updateContextUsage = useCallback(() => {
    try {
      let totalTokens = 0;
      
      // System prompt (~200 tokens)
      totalTokens += 200;
      
      // Message history (last 10 messages)
      const recentMessages = messages.slice(-10);
      recentMessages.forEach(msg => {
        totalTokens += estimateTokens(msg.content);
      });
      
      // Current input
      totalTokens += estimateTokens(input);
      
      // Buffer for response (~500 tokens)
      totalTokens += 500;
      
      setContextUsage(totalTokens);
    } catch (e) {
      // Fallback
      setContextUsage(0);
    }
  }, [messages, input, estimateTokens]);

  useEffect(() => {
    refreshHistory();
    refreshSummaries();
  }, [refreshHistory, refreshSummaries]);

  // Update context usage whenever messages or input changes
  useEffect(() => {
    updateContextUsage();
  }, [messages, input, updateContextUsage]);

  // Poll runtime status for max context and memory count
  useEffect(() => {
    const pollStatus = async () => {
      try {
        const status = await fetchRuntimeStatus();
        if (status?.active_model_context) {
          setMaxContext(status.active_model_context);
        }
        if (status?.memory_nodes_injected !== undefined) {
          setMemoryNodesCount(status.memory_nodes_injected);
        }
        if (status?.has_summary !== undefined) {
          setHasSummary(status.has_summary);
        }
      } catch (e) {
        // Ignore errors
      }
    };

    pollStatus();
    const interval = setInterval(pollStatus, 2000); // Update every 2 seconds for real-time feel
    return () => clearInterval(interval);
  }, []);

  // Handle scroll event
  const handleScroll = () => {
    const container = messagesAreaRef.current;
    if (!container) return;
    // If the user scrolls up, disable auto-scroll. If they scroll to bottom, re-enable it.
    const isAtBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 120;
    setShouldAutoScroll(isAtBottom);
  };

  // Scroll to bottom effect
  useEffect(() => {
    if (shouldAutoScroll && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages, streamingContent, shouldAutoScroll]);

  // Auto-grow textarea height
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = 'auto';
    textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
  }, [input]);

  // ─── History Pane Resize ─────────────────────────────────────
  useEffect(() => {
    if (!isResizingHistory) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (historyRef.current) {
        const rect = historyRef.current.getBoundingClientRect();
        const newWidth = Math.min(Math.max(e.clientX - rect.left, 160), 400);
        setHistoryWidth(newWidth);
      }
    };
    const handleMouseUp = () => setIsResizingHistory(false);

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingHistory]);

  // ─── Session Helpers ─────────────────────────────────────────
  const handleNewSession = async () => {
    // Debounce: prevent double-click racing
    if (isStartingSession.current) return;
    isStartingSession.current = true;
    try {
      await startNewSession();
      setMessages([]);
      setActiveSessionId(null);
      refreshSummaries();
      onStatusRefresh?.();
    } catch { /* ignore */ } finally {
      isStartingSession.current = false;
    }
  };

  const handleLoadSession = async (id: number) => {
    try {
      const res = await loadVaultSummary(id);
      if (res && res.messages) {
        setMessages(res.messages);
        setActiveSessionId(id);
        setShouldAutoScroll(true);
        onStatusRefresh?.();
      }
    } catch { /* ignore */ }
  };

  const handleDeleteSession = async (id: number) => {
    try {
      await deleteVaultSummary(id);
      refreshSummaries();
    } catch { /* ignore */ }
  };

  // ─── Clipboard Helper ────────────────────────────────────────
  const copyToClipboard = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  // ─── Send Chat ───────────────────────────────────────────────
  const handleSend = async () => {
    const text = input.trim();
    if (!text || streaming) return;
    setInput('');
    setShowSuggestions(false);
    setShouldAutoScroll(true); // Always force scroll on send

    // ── Local slash commands intercept ──
    if (text.startsWith('/')) {
      const parts = text.split(' ');
      const cmd = parts[0].toLowerCase();

      // Clear/Reset chat
      if (cmd === '/clear' || cmd === '/reset') {
        try {
          await clearChatHistory();
          setMessages([]);
        } catch (e) {
          const errMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `Failed to clear: ${e}`,
            timestamp: Date.now(),
          };
          setMessages((prev) => [...prev, errMsg]);
        }
        return;
      }

      // New Session
      if (cmd === '/new') {
        await handleNewSession();
        return;
      }

      // Switching active model locally
      if (cmd === '/model' && parts.length > 1) {
        const targetModel = parts.slice(1).join(' ');
        try {
          await onSwitchModel(targetModel);
          const feedback: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `Switched active model to: **${targetModel}**`,
            timestamp: Date.now(),
          };
          setMessages((prev) => [...prev, feedback]);
        } catch (e) {
          const feedback: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `Failed to switch model: ${e}`,
            timestamp: Date.now(),
          };
          setMessages((prev) => [...prev, feedback]);
        }
        return;
      }

      // System metrics status command
      if (cmd === '/status') {
        try {
          const s = await fetchGuardianStatus();
          const responseText = `**System Metrics Dashboard**\n\n- **CPU**: ${s.cpu_pct?.toFixed(1)}% (Temp: ${s.cpu_temp_c?.toFixed(1)}°C)\n- **RAM**: ${s.ram_pct?.toFixed(1)}% (${s.ram_used_mb}MB / ${s.ram_total_mb}MB)\n- **GPU**: ${s.gpu_pct?.toFixed(1)}% (VRAM: ${s.vram_used_mb}MB)`;
          const feedback: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: responseText,
            timestamp: Date.now(),
          };
          setMessages((prev) => [...prev, feedback]);
        } catch (e) {
          const feedback: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `Failed to get status: ${e}`,
            timestamp: Date.now(),
          };
          setMessages((prev) => [...prev, feedback]);
        }
        return;
      }

      // Help command
      if (cmd === '/help') {
        const helpText = COMMANDS.map((c) => `\`${c.name}\` — ${c.desc}`).join('\n');
        const feedback: Message = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `### Available Commands\n\n${helpText}`,
          timestamp: Date.now(),
        };
        setMessages((prev) => [...prev, feedback]);
        return;
      }
    }

    // ── Standard Chat or Backend Commands (e.g. /save, /frommemory, /memory) ──
    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setStreaming(true);
    setStreamingContent('');

    sendChat(
      text,
      selectedModel,
      (token) => setStreamingContent((prev) => prev + token),
      () => {
        setStreaming(false);
        setStreamingContent('');
        refreshHistory();
        // Refresh runtime status so context window shows live token count
        onStatusRefresh?.();
      }
    );
  };

  // ─── Input Suggestions Auto-complete ─────────────────────────
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);
    if (val === '/') {
      setShowSuggestions(true);
      setActiveSuggestion(0);
    } else if (val.startsWith('/')) {
      setShowSuggestions(true);
    } else {
      setShowSuggestions(false);
    }
  };

  const filteredCommands = input.startsWith('/')
    ? COMMANDS.filter((c) => c.name.startsWith(input.split(' ')[0]))
    : COMMANDS;

  const selectSuggestion = (name: string) => {
    setInput(name + ' ');
    setShowSuggestions(false);
    textareaRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (showSuggestions && filteredCommands.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveSuggestion((prev) => (prev + 1) % filteredCommands.length);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveSuggestion((prev) => (prev - 1 + filteredCommands.length) % filteredCommands.length);
      } else if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        selectSuggestion(filteredCommands[activeSuggestion].name);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        setShowSuggestions(false);
      }
    } else if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // ─── Formatting Utilities ────────────────────────────────────
  const formatSeparatorDate = (timestamp: number) => {
    const msgDate = new Date(timestamp);
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(today.getDate() - 1);

    if (msgDate.toDateString() === today.toDateString()) {
      return 'Today';
    } else if (msgDate.toDateString() === yesterday.toDateString()) {
      return 'Yesterday';
    } else {
      return msgDate.toLocaleDateString([], {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        year: 'numeric',
      });
    }
  };

  const formatTime = (ts: number) =>
    new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  // ─── ReactMarkdown Custom Code Renderer ───────────────────────
  const renderCode = ({ className, children, ...props }: any) => {
    const match = /language-(\w+)/.exec(className || '');
    const codeString = String(children).replace(/\n$/, '');
    const isInline = !match && !codeString.includes('\n');

    if (!isInline) {
      const copyId = `code-${Math.random().toString(36).substring(2, 9)}`;
      return (
        <div className="code-block-container">
          <div className="code-block-header">
            <span className="code-block-lang">{match ? match[1] : 'text'}</span>
            <button
              className="code-block-copy"
              onClick={() => copyToClipboard(codeString, copyId)}
            >
              <Copy size={11} />
              <span>{copiedId === copyId ? 'Copied' : 'Copy'}</span>
            </button>
          </div>
          <pre className="code-block-pre">
            <code className={className} {...props}>
              {children}
            </code>
          </pre>
        </div>
      );
    }
    return (
      <code className="inline-code" {...props}>
        {children}
      </code>
    );
  };

  // Render Date Separator groups
  let lastDateStr = '';

  const isModelMissing = !activeModel || activeModel === 'No Model';

  return (
    <div className="chat-layout">
      {/* Sessions Sidebar */}
      {showHistoryPane && (
        <>
          <div ref={historyRef} className="history-pane" style={{ width: `${historyWidth}px` }}>
            <div className="history-pane-header">
              <span className="history-pane-title">Conversations</span>
              <button className="history-new-btn" onClick={handleNewSession} title="New chat session">
                <Plus size={13} />
              </button>
            </div>
            <div className="history-pane-list">
              {summaries.length === 0 ? (
                <div className="history-pane-empty">Empty sandbox.</div>
              ) : (
                summaries.map((s) => (
                  <div
                    key={s.id}
                    className={`history-item ${activeSessionId === s.id ? 'active' : ''}`}
                    onClick={() => handleLoadSession(s.id)}
                  >
                    <div className="history-item-header">
                      <div className="history-item-title">{s.title || 'Untitled Session'}</div>
                      <button
                        className="history-item-delete-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteSession(s.id);
                        }}
                        title="Delete session"
                      >
                        <Trash2 size={11} />
                      </button>
                    </div>
                    <div className="history-item-summary">{s.summary}</div>
                    <div className="history-item-date">
                      {new Date(s.timestamp * 1000).toLocaleDateString([], {
                        month: 'short',
                        day: 'numeric',
                      })}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
          <div
            className="history-resize-handle"
            onMouseDown={(e) => {
              e.preventDefault();
              setIsResizingHistory(true);
            }}
          />
        </>
      )}

      {/* Main Chat Frame with Context Panel */}
      <div style={{ display: 'flex', flex: 1, minWidth: 0 }}>
        {/* Chat Main */}
      <div className="chat-main">
        {/* Header */}
        <div className="chat-header">
          <div className="chat-header-left">
            <button
              className="header-btn"
              onClick={() => {
                setShowHistoryPane((prev) => {
                  if (!prev) refreshSummaries(); // refresh when opening
                  return !prev;
                });
              }}
              title={showHistoryPane ? 'Hide sessions list' : 'Show sessions list'}
            >
              {showHistoryPane ? <PanelLeftClose size={15} /> : <PanelLeftOpen size={15} />}
            </button>
            <span className="chat-header-title">Sentinel Console</span>
            <div className="chat-header-model-selector">
              <select 
                value={selectedModel || activeModel || ''} 
                onChange={(e) => setSelectedModel(e.target.value)}
                className="model-select-dropdown"
              >
                {installedModels.map(m => (
                  <option key={m.name} value={m.name}>{m.name}</option>
                ))}
                {installedModels.length === 0 && <option value="">No Models Installed</option>}
              </select>
            </div>
          </div>
          <div className="chat-header-actions">
            <button className="header-btn" onClick={() => setShowCodeExecutor(true)} title="Open code executor">
              <Code2 size={15} /> <span>Code</span>
            </button>
            <button className="header-btn accent" onClick={handleNewSession} title="New chat session">
              <Plus size={13} /> <span>New Chat</span>
            </button>
            <button
              className={`header-btn ${showContextPanel ? 'active' : ''}`}
              onClick={() => setShowContextPanel(!showContextPanel)}
              title={showContextPanel ? 'Hide context metrics' : 'Show context metrics'}
            >
              {showContextPanel ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}
            </button>
          </div>
        </div>

        {/* Warning Banner (if no active model loaded) */}
        {isModelMissing && (
          <div className="warning-banner">
            <div className="warning-banner-left">
              <AlertCircle size={14} />
              <span className="warning-banner-text">
                Ollama service disconnected or no active LLM model is loaded.
              </span>
            </div>
            {installedModels.length > 0 && (
              <div className="warning-banner-actions">
                <select
                  className="warning-select-model"
                  defaultValue=""
                  onChange={(e) => {
                    if (e.target.value) onSwitchModel(e.target.value);
                  }}
                >
                  <option value="" disabled>
                    Switch to model...
                  </option>
                  {installedModels.map((m) => (
                    <option key={m.name} value={m.name}>
                      {m.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>
        )}

        {/* Message Sandbox */}
        <div ref={messagesAreaRef} className="messages-area" onScroll={handleScroll}>
          <div className="messages-inner">
          {messages.length === 0 && !streaming && (
            <div className="empty-state">
              <div className="empty-title">SENTINEL CONSOLE</div>
              <div className="empty-sub">
                Sandbox ready. Type a message to begin. Slash commands are intercepted locally.
              </div>
              <div className="empty-state-console">
                <div className="console-header">
                  <span className="console-title">sentinel-cli v0.1.0</span>
                  <span>localhost:{(window as any).electronAPI && typeof (window as any).electronAPI.getDaemonPort === 'function' ? (window as any).electronAPI.getDaemonPort() : '8888'}</span>
                </div>
                <div className="console-grid">
                  <span className="console-cmd">/new</span>
                  <span className="console-desc">Start a fresh conversation sandbox</span>

                  <span className="console-cmd">/save name: desc</span>
                  <span className="console-desc">Persist a manual fact into knowledge vault</span>

                  <span className="console-cmd">/frommemory query</span>
                  <span className="console-desc">Fetch nodes matching context from vault</span>

                  <span className="console-cmd">/memory</span>
                  <span className="console-desc">List all manual facts currently saved</span>

                  <span className="console-cmd">/status</span>
                  <span className="console-desc">Poll hardware controller and system metrics</span>

                  <span className="console-cmd">/model name</span>
                  <span className="console-desc">Switch active model dynamically</span>

                  <span className="console-cmd">/reset</span>
                  <span className="console-desc">Wipe all local console logs and histories</span>
                </div>
              </div>
            </div>
          )}

          {messages.map((msg) => {
            const dateStr = formatSeparatorDate(msg.timestamp);
            const showSeparator = dateStr !== lastDateStr;
            lastDateStr = dateStr;

          return (
              <React.Fragment key={msg.id}>
                {showSeparator && (
                  <div className="date-separator">
                    <span className="date-separator-text">{dateStr}</span>
                  </div>
                )}

                <div className={`msg-wrapper ${msg.role}`}>
                  <div className={`msg ${msg.role}`}>
                    <div className="msg-avatar">
                      {msg.role === 'user' ? '👤' : '🤖'}
                    </div>
                    <div className="msg-content-wrapper">
                      <div className="msg-header">
                        <span className="msg-role-name">
                          {msg.role === 'user' ? 'You' : 'Sentinel'}
                        </span>
                        <span className="msg-time-stamp">{formatTime(msg.timestamp)}</span>
                      </div>

                      <div className="msg-body">
                        {msg.role === 'user' ? (
                          <div className="msg-content">{msg.content}</div>
                        ) : (
                          <div className="markdown-body">
                            <ReactMarkdown
                              remarkPlugins={[remarkGfm]}
                              rehypePlugins={[rehypeRaw]}
                              components={{
                                code: renderCode,
                              }}
                            >
                              {msg.content}
                            </ReactMarkdown>
                          </div>
                        )}
                      </div>
                    </div>

                    <button
                      className="msg-copy-btn"
                      onClick={() => copyToClipboard(msg.content, msg.id)}
                      title="Copy message"
                    >
                      <Copy size={14} />
                    </button>
                  </div>
                </div>
              </React.Fragment>
            );

          })}

          {streaming && (
            <div className="msg-wrapper assistant">
              <div className="msg assistant">
                <div className="msg-avatar">🤖</div>
                <div className="msg-content-wrapper">
                  <div className="msg-header">
                    <span className="msg-role-name">Sentinel</span>
                    <span className="streaming-indicator">⚡ generating...</span>
                  </div>
                  <div className="msg-body">
                    <div className="markdown-body">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        rehypePlugins={[rehypeRaw]}
                        components={{
                          code: renderCode,
                        }}
                      >
                        {streamingContent}
                      </ReactMarkdown>
                      <span className="cursor-blink">│</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}


          <div ref={bottomRef} />
          </div>{/* end .messages-inner */}
        </div>

        {/* Input Bar */}
        <div className="chat-input-area">
          {showSuggestions && filteredCommands.length > 0 && (
            <div className="cmd-palette">
              {filteredCommands.map((cmd, idx) => (
                <div
                  key={cmd.name}
                  className={`cmd-option ${idx === activeSuggestion ? 'active' : ''}`}
                  onClick={() => selectSuggestion(cmd.name)}
                >
                  <span className="cmd-name">{cmd.name}</span>
                  <span className="cmd-desc">{cmd.desc}</span>
                </div>
              ))}
            </div>
          )}

          <div className="input-container-box">
            <div className="input-row">
              <textarea
                ref={textareaRef}
                className="chat-textarea"
                value={input}
                onChange={handleInputChange}
                onKeyDown={handleKeyDown}
                placeholder="Type a message or press '/' for commands..."
                rows={1}
                disabled={streaming}
              />
              <button
                className={`send-btn ${streaming ? 'disabled' : ''}`}
                onClick={handleSend}
                disabled={streaming || !input.trim()}
              >
                {streaming ? (
                  <RefreshCw size={14} className="spin" />
                ) : (
                  <Send size={14} />
                )}
              </button>
            </div>
            <div className="input-footer">
              <div className="input-model-indicator">
                <span>Active Model:</span>
                <strong className={isModelMissing ? 'streaming-indicator' : ''}>
                  {activeModel || 'No Model Selected'}
                </strong>
              </div>
              <div className="input-tips">
                <span>Enter to send • Shift+Enter for new line</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Context Panel (Real-time) */}
      {showContextPanel && (
        <ContextPanel
          modelName={activeModel || 'No Model'}
          contextUsage={contextUsage}
          maxContext={maxContext}
          memoryNodes={memoryNodesCount}
          hasSummary={hasSummary}
          installedModels={installedModels}
          onSwitchModel={onSwitchModel}
          onClose={() => setShowContextPanel(false)}
        />
      )}
    </div>

      {/* Code Executor Modal */}
      {showCodeExecutor && <CodeExecutor onClose={() => setShowCodeExecutor(false)} />}
    </div>
  );
};
