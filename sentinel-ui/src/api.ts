const BASE = (window as any).electronAPI && typeof (window as any).electronAPI.getDaemonPort === 'function'
  ? `http://localhost:${(window as any).electronAPI.getDaemonPort()}`
  : 'http://localhost:8888';


// ─── Health ──────────────────────────────────────────────────────────────────
export const fetchHealth = () => fetch(`${BASE}/health`).then(r => r.json());

// ─── Runtime ─────────────────────────────────────────────────────────────────
export const fetchRuntimeStatus = () => fetch(`${BASE}/runtime/status`).then(r => r.json());
export const fetchModels = () => fetch(`${BASE}/runtime/models`).then(r => r.json());
export const switchModel = (name: string) =>
  fetch(`${BASE}/runtime/switch`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) }).then(r => r.json());
export const loadModel = (name: string) =>
  fetch(`${BASE}/runtime/load`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) }).then(r => r.json());
export const unloadModel = (name?: string) =>
  fetch(`${BASE}/runtime/unload`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) }).then(r => r.json());
export const pullModel = (name: string) =>
  fetch(`${BASE}/runtime/pull`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) }).then(r => r.json());
export const deleteModel = (name: string) =>
  fetch(`${BASE}/runtime/model/${encodeURIComponent(name)}`, { method: 'DELETE' }).then(r => r.json());

// ─── Chat ─────────────────────────────────────────────────────────────────────
export const fetchChatHistory = () => fetch(`${BASE}/chat/history`).then(r => r.json());
export const fetchAllChatHistory = () => fetch(`${BASE}/vault/chat/history`).then(r => r.json());
export const clearChatHistory = () => fetch(`${BASE}/chat/history`, { method: 'DELETE' }).then(r => r.json());
export const startNewSession = () => fetch(`${BASE}/chat/session/new`, { method: 'POST' }).then(r => r.json());

export const sendChat = (message: string, model: string | null, onToken: (token: string) => void, onDone?: () => void) => {
  const safeOnDone = onDone || (() => {});
  fetch(`${BASE}/chat/send`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ message, model })
  }).then(async res => {
    const reader = res.body?.getReader();
    const decoder = new TextDecoder();
    if (!reader) {
      safeOnDone();
      return;
    }
    let buf = '';
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() ?? '';
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          try {
            const data = JSON.parse(line.slice(6));
            if (data.token) onToken(data.token);
            if (data.error) onToken(`Error: ${data.error}`);
            if (data.done) safeOnDone();
          } catch {}
        }
      }
    }
    safeOnDone();
  }).catch(() => safeOnDone());
};

// ─── Guardian ─────────────────────────────────────────────────────────────────
export const fetchGuardianStatus = () => fetch(`${BASE}/guardian/status`).then(r => r.json());
export const fetchGuardianProcesses = () => fetch(`${BASE}/guardian/processes`).then(r => r.json());

// ─── Vault ────────────────────────────────────────────────────────────────────
export const fetchVaultNodes = () => fetch(`${BASE}/vault/nodes`).then(r => r.json());
export const fetchVaultEdges = () => fetch(`${BASE}/vault/edges`).then(r => r.json());
export const fetchVaultSummaries = () => fetch(`${BASE}/vault/summaries`).then(r => r.json());
export const deleteVaultSummary = (id: number) => fetch(`${BASE}/vault/summary/${id}`, { method: 'DELETE' }).then(r => r.json());
export const loadVaultSummary = (id: number) => fetch(`${BASE}/vault/summary/${id}/load`, { method: 'POST' }).then(r => r.json());
export const searchVault = (query: string) => fetch(`${BASE}/vault/search?q=${encodeURIComponent(query)}`).then(r => r.json());
export const deleteVaultNode = (id: number) => fetch(`${BASE}/vault/node/${id}`, { method: 'DELETE' }).then(r => r.json());
export const deleteVaultEdge = (id: number) => fetch(`${BASE}/vault/edge/${id}`, { method: 'DELETE' }).then(r => r.json());
export const saveMemory = (content: string) =>
  fetch(`${BASE}/vault/save`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ content }) }).then(r => r.json());
export const exportVault = () => fetch(`${BASE}/vault/export`).then(r => r.json());

// ─── Skills ───────────────────────────────────────────────────────────────────
export const fetchSkills = () => fetch(`${BASE}/skills`).then(r => r.json());
export const enableSkill = (name: string) =>
  fetch(`${BASE}/skills/${encodeURIComponent(name)}/enable`, { method: 'POST' }).then(r => r.json());
export const disableSkill = (name: string) =>
  fetch(`${BASE}/skills/${encodeURIComponent(name)}/disable`, { method: 'POST' }).then(r => r.json());
export const deleteSkill = (name: string) =>
  fetch(`${BASE}/skills/${encodeURIComponent(name)}`, { method: 'DELETE' }).then(r => r.json());

// ─── Settings ─────────────────────────────────────────────────────────────────
export const fetchSettings = () => fetch(`${BASE}/settings`).then(r => r.json());
export const updateSettings = (settings: Record<string, unknown>) =>
  fetch(`${BASE}/settings`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(settings) }).then(r => r.json());

export const searchOllamaModels = (query: string) => fetch(`${BASE}/runtime/models/search?q=${encodeURIComponent(query)}`).then(r => r.json());

/**
 * Lightweight real-time metrics poll — called every few seconds.
 * Backend should return:
 *   { ram_used_mb, ram_total_mb, vram_used_mb, vram_total_mb, cpu_percent }
 * If your daemon exposes this differently, adjust the endpoint path here.
 */
export const fetchHardwareMetrics = () =>
  fetch(`${BASE}/runtime/metrics`).then(r => r.json());

// ─── Ollama Library Scraping (proxied through backend to avoid CORS) ──────────

/**
 * Fetches the full Ollama library index via backend proxy.
 * Returns raw HTML text of https://ollama.com/library
 */
export const fetchOllamaLibraryIndex = (): Promise<string> =>
  fetch(`${BASE}/runtime/proxy/ollama?path=${encodeURIComponent('/library')}`)
    .then(r => r.text());

/**
 * Fetches a specific model page from Ollama library via backend proxy.
 * Returns raw HTML text of https://ollama.com/library/<modelBase>
 * e.g. fetchOllamaModelPage('gemma3') → HTML with all tags/variants
 */
export const fetchOllamaModelPage = (modelBase: string): Promise<string> =>
  fetch(`${BASE}/runtime/proxy/ollama?path=${encodeURIComponent(`/library/${modelBase}`)}`)
    .then(r => r.text());

// ─── Code Execution (IDE Features) ────────────────────────────────────────────
export const executeCode = (language: string, code: string) =>
  fetch(`${BASE}/code/execute`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ language, code })
  }).then(r => r.json());

export const analyzeCode = (language: string, code: string) =>
  fetch(`${BASE}/code/analyze`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ language, code })
  }).then(r => r.json());

export const formatCode = (language: string, code: string) =>
  fetch(`${BASE}/code/format`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ language, code })
  }).then(r => r.json());