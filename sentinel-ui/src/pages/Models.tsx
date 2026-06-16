import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Cpu, Plus, Trash2, Play, Square, Star, Zap, RefreshCw, Server, ShieldCheck, HelpCircle } from 'lucide-react';
import { fetchModels, fetchRuntimeStatus, loadModel, unloadModel, deleteModel, pullModel, switchModel, searchOllamaModels, fetchOllamaModelPage, fetchHardwareMetrics } from '../api';
import './Models.css';

interface Model {
  name: string;
  size: number;
  modified_at: string;
  loaded?: boolean;
  context_length: number;
  param_count: number | null;
  quantization: string | null;
  architecture: string | null;
  embedding_length: number | null;
  estimated_ram_mb: number;
  recommended: boolean;
  recommendation_reason: string;
  is_cloud?: boolean;
  cloud_provider?: string | null;
}

interface RecommendedModel {
  name: string;
  context_length: number;
}

interface OllamaVariant {
  tag: string;       // e.g. "qwen3.5:9b"
  sizeMb: number | null;   // actual file size in MB from ollama.com
  context: string | null;  // e.g. "256K"
  input: string | null;    // e.g. "Text, Image"
  isLatest: boolean;
}

interface HardwareInfo {
  ram_total_mb: number;
  ram_available_mb: number;
  cpu_cores: number;
  cpu_model: string;
  gpu_vendor: string;
  gpu_name: string | null;
  vram_total_mb: number | null;
  vram_available_mb: number | null;
  tier: string;
  recommended_memory_mode: string;
  memory_mode_reason: string;
}

interface LiveMetrics {
  ram_used_mb: number;
  ram_total_mb: number;
  vram_used_mb: number | null;
  vram_total_mb: number | null;
  cpu_percent: number;
  gpu_percent: number | null;
}

export const Models: React.FC = () => {
  const [models, setModels] = useState<Model[]>([]);
  const [recommended, setRecommended] = useState<RecommendedModel[]>([]);
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [liveMetrics, setLiveMetrics] = useState<LiveMetrics | null>(null);
  const [activeModel, setActiveModel] = useState('');
  const [pullName, setPullName] = useState('');
  const [pulling, setPulling] = useState(false);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [searchResults, setSearchResults] = useState<string[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [showDropdown, setShowDropdown] = useState(false);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [variants, setVariants] = useState<OllamaVariant[]>([]);
  const [loadingVariants, setLoadingVariants] = useState(false);
  const searchTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const dropdownRef = useRef<HTMLDivElement | null>(null);
  const metricsIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const load = async (isSilent = false) => {
    if (!isSilent) setLoading(true);
    else setRefreshing(true);
    try {
      const [mods, status] = await Promise.all([fetchModels(), fetchRuntimeStatus()]);
      setModels(Array.isArray(mods) ? mods : []);
      setActiveModel(status?.active_model || '');
      setRecommended(status?.recommended_models || []);
      if (status?.hardware) {
        const hw: HardwareInfo = status.hardware;
        setHardware(hw);
        // Seed live metrics — VRAM/GPU left null until first real poll
        setLiveMetrics({
          ram_used_mb: hw.ram_total_mb - hw.ram_available_mb,
          ram_total_mb: hw.ram_total_mb,
          vram_used_mb: null,
          vram_total_mb: hw.vram_total_mb ?? null,
          cpu_percent: 0,
          gpu_percent: null,
        });
      }
    } catch {}
    setLoading(false);
    setRefreshing(false);
  };

  // Poll live metrics: try /runtime/metrics first, fall back to /runtime/status
  const pollMetrics = useCallback(async () => {
    // Primary: dedicated metrics endpoint (fast, lightweight)
    try {
      const m = await fetchHardwareMetrics();
      if (m && typeof m.ram_used_mb === 'number') {
        setLiveMetrics(m as LiveMetrics);
        return;
      }
    } catch {}

    // Fallback: re-use runtime/status which always exists
    try {
      const status = await fetchRuntimeStatus();
      if (status?.hardware) {
        const hw: HardwareInfo = status.hardware;
        setHardware(hw);
        setLiveMetrics({
          ram_used_mb: hw.ram_total_mb - hw.ram_available_mb,
          ram_total_mb: hw.ram_total_mb,
          vram_used_mb: null,
          vram_total_mb: hw.vram_total_mb ?? null,
          cpu_percent: 0,
          gpu_percent: null,
        });
      }
    } catch {}
  }, []);

  useEffect(() => {
    load();
  }, []);

  // Start polling immediately on mount, every 3 s
  useEffect(() => {
    const id = setInterval(pollMetrics, 3000);
    metricsIntervalRef.current = id;
    return () => clearInterval(id);
  }, [pollMetrics]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
        setSelectedModel(null);
        setVariants([]);
      }
    };
    document.addEventListener('mousedown', handleOutside);
    return () => document.removeEventListener('mousedown', handleOutside);
  }, []);

  // Parse size string from ollama.com like "6.6GB" or "1.0GB" → MB
  const parseOllamaSize = (sizeStr: string): number | null => {
    if (!sizeStr || sizeStr.trim() === '') return null;
    const m = sizeStr.trim().match(/^([\d.]+)\s*(GB|MB|KB)?$/i);
    if (!m) return null;
    const val = parseFloat(m[1]);
    const unit = (m[2] || 'GB').toUpperCase();
    if (unit === 'GB') return Math.round(val * 1024);
    if (unit === 'MB') return Math.round(val);
    if (unit === 'KB') return Math.round(val / 1024);
    return null;
  };

  /**
   * Scrape all variants for a model from ollama.com via backend proxy.
   *
   * Strategy:
   *  1. Fetch https://ollama.com/library/<modelBase> through the backend proxy
   *     (avoids browser CORS restrictions).
   *  2. Parse the returned HTML with DOMParser.
   *  3. Primary: find every <a href="/library/modelBase:tag"> anchor and walk
   *     up to its containing row to extract size, context-window, and input type.
   *  4. Fallback: scan Next.js __NEXT_DATA__ / application/json script tags for
   *     a structured tags array.
   *  5. Deduplicate by full tag string; sort so "latest" aliases come first,
   *     then by param count ascending, then alphabetically.
   */
  const fetchOllamaVariants = async (modelBase: string): Promise<OllamaVariant[]> => {
    let html = '';
    try {
      // Use backend proxy to bypass CORS
      html = await fetchOllamaModelPage(modelBase);
    } catch {
      return [];
    }

    try {
      const parser = new DOMParser();
      const doc = parser.parseFromString(html, 'text/html');
      const seen = new Set<string>();
      const variants: OllamaVariant[] = [];

      // ── Primary: anchor-based scraping ────────────────────────────────────
      // Ollama renders each tag as a link: /library/gemma3:2b, /library/gemma3:27b-instruct-q4_K_M …
      const tagLinks = Array.from(
        doc.querySelectorAll(`a[href*="/library/${modelBase}:"]`)
      ) as HTMLAnchorElement[];

      for (const link of tagLinks) {
        const href = link.getAttribute('href') || link.href || '';
        // Match  /library/modelBase:tagname  (no further slashes)
        const tagMatch = href.match(new RegExp(`/library/(${modelBase}:[^\\s/?#]+)`));
        if (!tagMatch) continue;
        const fullTag = tagMatch[1];
        if (seen.has(fullTag)) continue;
        seen.add(fullTag);

        // Walk up the DOM to the nearest row-like container
        const row =
          link.closest('tr') ||
          link.closest('[class*="row"]') ||
          link.closest('[class*="item"]') ||
          link.parentElement?.parentElement ||
          link.parentElement;

        let sizeMb: number | null = null;
        let context: string | null = null;
        let input: string | null = null;
        let isLatest = false;

        if (row) {
          const text = row.textContent || '';

          // "latest" marker
          isLatest = /\blatest\b/i.test(text);

          // Size: "6.6 GB", "17GB", "830 MB" etc.
          const sizeMatch = text.match(/\b([\d]+(?:\.\d+)?\s*(?:GB|MB))\b/i);
          if (sizeMatch) sizeMb = parseOllamaSize(sizeMatch[1]);

          // Context window: "256K", "128K", "8K", "1M" etc.
          const ctxMatch = text.match(/\b(\d+(?:\.\d+)?[KM])\b/i);
          if (ctxMatch) context = ctxMatch[1].toUpperCase();

          // Input modality
          if (/\bvision\b|\bimage\b|\bmultimodal\b/i.test(text)) input = 'Text, Image';
          else if (/\btext\b/i.test(text)) input = 'Text';
        }

        variants.push({ tag: fullTag, sizeMb, context, input, isLatest });
      }

      // ── Fallback: Next.js / JSON-LD structured data ───────────────────────
      if (variants.length === 0) {
        const scripts = Array.from(
          doc.querySelectorAll('script[type="application/json"], script#__NEXT_DATA__, script[type="application/ld+json"]')
        ) as HTMLScriptElement[];

        for (const s of scripts) {
          try {
            const json = JSON.parse(s.textContent || '');
            // Support multiple possible shapes
            const tags: any[] =
              json?.props?.pageProps?.model?.tags ||
              json?.pageProps?.model?.tags ||
              json?.model?.tags ||
              [];

            for (const t of tags) {
              const tagName = t.name || t.tag || t.id || '';
              if (!tagName) continue;
              const fullTag = tagName.includes(':') ? tagName : `${modelBase}:${tagName}`;
              if (seen.has(fullTag)) continue;
              seen.add(fullTag);

              // Size may be in bytes
              const rawSize = t.size ?? t.file_size ?? null;
              const sizeMb = rawSize ? Math.round(rawSize / (1024 * 1024)) : null;

              variants.push({
                tag: fullTag,
                sizeMb,
                context: t.context_length ? String(t.context_length) : null,
                input: t.input_modalities || null,
                isLatest: !!(t.latest || t.is_latest || /\blatest\b/i.test(tagName)),
              });
            }
          } catch {}
        }
      }

      // ── Regex fallback on raw HTML ─────────────────────────────────────────
      // If DOM parsing found nothing, scan the raw HTML for href patterns
      if (variants.length === 0) {
        const re = new RegExp(`href=["']/library/(${modelBase}:[^"'\\s/?#]+)["']`, 'g');
        let m: RegExpExecArray | null;
        while ((m = re.exec(html)) !== null) {
          const fullTag = m[1];
          if (seen.has(fullTag)) continue;
          seen.add(fullTag);
          variants.push({ tag: fullTag, sizeMb: null, context: null, input: null, isLatest: /\blatest\b/.test(fullTag) });
        }
      }

      // ── Sort: latest aliases first, then by param count, then alpha ───────
      const paramCount = (tag: string): number => {
        const m = tag.match(/:.*?(\d+(?:\.\d+)?)b/i);
        return m ? parseFloat(m[1]) : 999;
      };

      variants.sort((a, b) => {
        // "latest" tag alias always first
        if (a.tag.endsWith(':latest')) return -1;
        if (b.tag.endsWith(':latest')) return 1;
        // Then sort by param size
        const pa = paramCount(a.tag);
        const pb = paramCount(b.tag);
        if (pa !== pb) return pa - pb;
        // Then alphabetically within same param size (e.g. q4_K_M before q8_0)
        return a.tag.localeCompare(b.tag);
      });

      return variants;
    } catch {
      return [];
    }
  };

  // Estimate RAM needed (MB) from actual file size (sizeMb) — add ~15% overhead for KV cache etc.
  const estimateRamFromSize = (sizeMb: number): number => Math.round(sizeMb * 1.15 + 512);

  // Fallback estimate from tag string when no real size available
  const estimateVariantRamMb = (tag: string): number | null => {
    const lower = tag.toLowerCase();
    const paramMatch = lower.match(/[:\-_](\d+(?:\.\d+)?)b/);
    if (!paramMatch) return null;
    const paramB = parseFloat(paramMatch[1]);
    let bpw = 4.5;
    if (/fp16|f16/.test(lower))      bpw = 16;
    else if (/fp32|f32/.test(lower)) bpw = 32;
    else if (/q8/.test(lower))       bpw = 8;
    else if (/q6/.test(lower))       bpw = 6;
    else if (/q5_k_l/.test(lower))   bpw = 5.5;
    else if (/q5/.test(lower))       bpw = 5;
    else if (/q4_k_l/.test(lower))   bpw = 4.8;
    else if (/q4_k_m/.test(lower))   bpw = 4.5;
    else if (/q4_k_s/.test(lower))   bpw = 4.3;
    else if (/q4/.test(lower))       bpw = 4;
    else if (/q3/.test(lower))       bpw = 3;
    else if (/q2/.test(lower))       bpw = 2;
    const weightBytes = paramB * 1e9 * (bpw / 8);
    return Math.round((weightBytes / 1e6) * 1.10 + 1024);
  };

  const getVariantCompatibility = (v: OllamaVariant): 'vram' | 'ok' | 'tight' | 'toolarge' | 'unknown' => {
    if (!hardware) return 'unknown';
    const ramNeeded = v.sizeMb
      ? estimateRamFromSize(v.sizeMb)
      : estimateVariantRamMb(v.tag);
    if (ramNeeded === null) return 'unknown';
    if (hardware.vram_total_mb && ramNeeded <= hardware.vram_total_mb) return 'vram';
    if (ramNeeded <= hardware.ram_available_mb) return 'ok';
    if (ramNeeded <= hardware.ram_total_mb) return 'tight';
    return 'toolarge';
  };

  const formatSizeMb = (mb: number | null): string => {
    if (mb === null) return '';
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;
  };

  const formatSize = (bytes: number) => {
    if (!bytes) return 'Unknown';
    const gb = bytes / 1e9;
    return gb >= 1 ? `${gb.toFixed(1)} GB` : `${(bytes / 1e6).toFixed(0)} MB`;
  };

  const formatParams = (params: number | null) => {
    if (!params) return 'N/A';
    const billion = params / 1e9;
    return billion >= 1 ? `${billion.toFixed(1)}B` : `${(params / 1e6).toFixed(0)}M`;
  };

  const handleLoad = async (name: string) => {
    await loadModel(name);
    await switchModel(name);
    setActiveModel(name);
    load(true);
  };

  const handleUnload = async () => {
    await unloadModel();
    setActiveModel('');
    load(true);
  };

  const handleDelete = async (name: string) => {
    if (confirm(`Are you sure you want to delete ${name}?`)) {
      await deleteModel(name);
      load(true);
    }
  };

  const handlePull = async (targetName?: string) => {
    const nameToPull = targetName || pullName.trim();
    if (!nameToPull) return;
    setPulling(true);
    setShowDropdown(false);
    setSelectedModel(null);
    setVariants([]);
    await pullModel(nameToPull);
    setPullName('');
    setSearchResults([]);
    setPulling(false);
    alert(`Pull request for ${nameToPull} started in background.`);
    // Poll for status updates
    setTimeout(() => load(true), 2000);
    setTimeout(() => load(true), 5000);
  };

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setPullName(val);
    setSelectedModel(null);
    setVariants([]);

    if (searchTimeoutRef.current) clearTimeout(searchTimeoutRef.current);

    if (!val.trim()) {
      setSearchResults([]);
      setIsSearching(false);
      setShowDropdown(false);
      return;
    }

    setIsSearching(true);
    setShowDropdown(true);
    searchTimeoutRef.current = setTimeout(async () => {
      try {
        const res = await searchOllamaModels(val);
        setSearchResults(res.results || []);
      } catch (e) {
        setSearchResults([]);
      } finally {
        setIsSearching(false);
      }
    }, 400);
  };

  const handleSelectModel = async (modelBase: string) => {
    setSelectedModel(modelBase);
    setPullName(modelBase);
    setLoadingVariants(true);
    setVariants([]);
    try {
      const scraped = await fetchOllamaVariants(modelBase);
      if (scraped.length > 0) {
        setVariants(scraped);
      } else {
        // Fallback: use backend search results as plain tags
        const res = await searchOllamaModels(modelBase + ':');
        const found: string[] = (res.results || []).filter((r: string) =>
          r === modelBase || r.startsWith(modelBase + ':')
        );
        const fallback: OllamaVariant[] = found.length > 0
          ? found.map(tag => ({ tag, sizeMb: null, context: null, input: null, isLatest: tag.includes('latest') }))
          : [{ tag: `${modelBase}:latest`, sizeMb: null, context: null, input: null, isLatest: true }];
        setVariants(fallback);
      }
    } catch {
      setVariants([{ tag: `${modelBase}:latest`, sizeMb: null, context: null, input: null, isLatest: true }]);
    } finally {
      setLoadingVariants(false);
    }
  };

  const handleSearchFocus = () => {
    if (pullName.trim()) setShowDropdown(true);
  };

  // Suitability check
  const getSuitabilityScore = (model: Model) => {
    if (model.is_cloud) {
      return {
        color: 'green',
        label: '✓ Available',
        desc: `Runs on ${model.cloud_provider || 'remote'} servers, zero local RAM required.`,
      };
    }
    if (!hardware) return { color: 'gray', label: 'Unknown', desc: 'No hardware profile loaded.' };
    const neededMb = model.estimated_ram_mb;
    
    // Fit fully in VRAM?
    if (hardware.vram_total_mb && neededMb <= hardware.vram_total_mb) {
      return {
        color: 'green',
        label: 'VRAM Accelerated',
        desc: `Fits fully in GPU VRAM (${hardware.vram_total_mb} MB available). Maximum performance.`,
      };
    }
    
    // Fits in available RAM?
    if (neededMb <= hardware.ram_available_mb) {
      return {
        color: 'yellow',
        label: 'Runs in RAM',
        desc: `Fits in available RAM. Will run on CPU or hybrid, which might be slower.`,
      };
    }
    
    // Fits in total RAM?
    if (neededMb <= hardware.ram_total_mb) {
      return {
        color: 'orange',
        label: 'Tight Fit',
        desc: `Fits in total RAM, but may compete with system applications causing slowdown.`,
      };
    }
    
    // Too large
    return {
      color: 'red',
      label: 'Insufficient RAM',
      desc: `Model needs ~${(neededMb / 1024).toFixed(1)} GB but system total is ${(hardware.ram_total_mb / 1024).toFixed(1)} GB. Run at risk of heavy lag or crash.`,
    };
  };

  if (loading) {
    return (
      <div className="page-content loading">
        <div className="spinner-container">
          <RefreshCw className="spinner-icon spin" size={32} />
          <p>Analyzing system hardware & loading models...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="page-content models-page">
      {/* Header */}
      <div className="models-header">
        <div className="title-area">
          <h2 className="page-title"><Server size={20} className="title-icon" /> System & Models</h2>
          <p className="page-subtitle">Manage local model parameters, check hardware compatibility, and scale your configurations.</p>
        </div>
        <div className="action-bar">
          <button className="icon-btn" onClick={() => load(true)} disabled={refreshing} title="Refresh Hardware Info">
            <RefreshCw size={16} className={refreshing ? 'spin' : ''} />
          </button>
          <div className="pull-bar" ref={dropdownRef} style={{ position: 'relative' }}>
            <input
              className="pull-input"
              placeholder="e.g. gemma2:2b, qwen2.5:7b..."
              value={pullName}
              onChange={handleSearchChange}
              onFocus={handleSearchFocus}
              onKeyDown={e => e.key === 'Enter' && handlePull()}
            />
            {showDropdown && (
              <div className="search-dropdown">
                {!selectedModel ? (
                  // Model list panel
                  <>
                    {isSearching ? (
                      <div className="search-dropdown-item loading">
                        <RefreshCw size={12} className="spin" style={{marginRight: 6}} />
                        Searching Ollama registry...
                      </div>
                    ) : searchResults.length > 0 ? (
                      searchResults.map(result => (
                        <div
                          key={result}
                          className="search-dropdown-item"
                          onClick={() => handleSelectModel(result)}
                        >
                          <Server size={12} style={{marginRight: 8, opacity: 0.5}} />
                          <span className="sdi-name">{result}</span>
                          <span className="sdi-chevron">›</span>
                        </div>
                      ))
                    ) : (
                      <div className="search-dropdown-item loading">No models found — try a different name</div>
                    )}
                  </>
                ) : (
                  // Variant panel
                  <div className="variant-panel">
                    <div className="variant-panel-header">
                      <button className="variant-back-btn" onClick={() => { setSelectedModel(null); setVariants([]); }}>
                        ‹ Back
                      </button>
                      <span className="variant-panel-title">{selectedModel}</span>
                      {hardware && (
                        <span className="variant-hw-hint">
                          {hardware.vram_total_mb
                            ? `${(hardware.vram_total_mb / 1024).toFixed(1)} GB VRAM · `
                            : ''}
                          {(hardware.ram_available_mb / 1024).toFixed(1)} GB RAM free
                        </span>
                      )}
                    </div>
                    {loadingVariants ? (
                      <div className="search-dropdown-item loading">
                        <RefreshCw size={12} className="spin" style={{marginRight: 6}} />
                        Fetching variants from ollama.com...
                      </div>
                    ) : variants.length === 0 ? (
                      <div className="search-dropdown-item loading">No variants found — try pulling directly</div>
                    ) : (
                      (() => {
                        // Group variants by param-size prefix (e.g. "2b", "7b", "70b")
                        // Tags that are just ":latest" or don't match a param go into "other"
                        const groups: Record<string, OllamaVariant[]> = {};
                        for (const v of variants) {
                          const m = v.tag.match(/:.*?(\d+(?:\.\d+)?)b/i);
                          const key = m ? `${m[1]}B` : (v.tag.endsWith(':latest') ? 'Latest' : 'Other');
                          if (!groups[key]) groups[key] = [];
                          groups[key].push(v);
                        }

                        // Sort group keys: Latest first, then numeric ascending, then Other
                        const sortedKeys = Object.keys(groups).sort((a, b) => {
                          if (a === 'Latest') return -1;
                          if (b === 'Latest') return 1;
                          if (a === 'Other') return 1;
                          if (b === 'Other') return -1;
                          return parseFloat(a) - parseFloat(b);
                        });

                        return sortedKeys.map(groupKey => (
                          <div key={groupKey} className="variant-group">
                            {sortedKeys.length > 1 && (
                              <div className="variant-group-label">{groupKey} params</div>
                            )}
                            {groups[groupKey].map(v => {
                              const compat = getVariantCompatibility(v);
                              const sizeLabel = formatSizeMb(v.sizeMb);
                              // Extract the quant part from the tag (everything after the param size)
                              const quantPart = v.tag.replace(/^[^:]+:/, ''); // e.g. "7b-instruct-q4_K_M"
                              return (
                                <div key={v.tag} className={`variant-item ${compat === 'toolarge' ? 'variant-item--incompatible' : ''}`}>
                                  <div className="variant-item-left">
                                    <div className="variant-name-row">
                                      <span className="variant-name" title={v.tag}>{quantPart}</span>
                                      {v.isLatest && <span className="variant-latest-badge">default</span>}
                                      {compat === 'vram'     && <span className="compat-tag vram">⚡ GPU</span>}
                                      {compat === 'ok'       && <span className="compat-tag green">✓ Fits</span>}
                                      {compat === 'tight'    && <span className="compat-tag yellow">⚠ Tight</span>}
                                      {compat === 'toolarge' && <span className="compat-tag red">✗ Too large</span>}
                                      {compat === 'unknown'  && <span className="compat-tag gray">?</span>}
                                    </div>
                                    <div className="variant-meta-row">
                                      {sizeLabel && <span className="variant-size">{sizeLabel}</span>}
                                      {v.sizeMb && <span className="variant-ram-est">~{((v.sizeMb * 1.15 + 512) / 1024).toFixed(1)} GB RAM</span>}
                                      {v.context && <span className="variant-ctx">{v.context} ctx</span>}
                                      {v.input && v.input.includes('Image') && <span className="variant-input">👁 Vision</span>}
                                    </div>
                                  </div>
                                  <button
                                    className="variant-pull-btn"
                                    onClick={() => handlePull(v.tag)}
                                    disabled={pulling}
                                  >
                                    {pulling ? <RefreshCw size={11} className="spin" /> : <><Plus size={11} /> Pull</>}
                                  </button>
                                </div>
                              );
                            })}
                          </div>
                        ));
                      })()
                    )}
                  </div>
                )}
              </div>
            )}
            <button className="pull-btn" onClick={() => handlePull()} disabled={pulling || !pullName.trim()}>
              {pulling ? <RefreshCw size={14} className="spin" /> : <><Plus size={14} /> Pull</>}
            </button>
          </div>
        </div>
      </div>

      {/* Hardware Profile Dashboard */}
      {hardware && (
        <section className="hardware-dashboard">
          <div className="hardware-dashboard-header">
            <h3 className="section-title"><Cpu size={14} /> Hardware Profile Dashboard</h3>
            <span className="live-indicator live-indicator--active">
              <span className="live-dot" />
              Live
            </span>
          </div>
          <div className="hardware-grid">

            {/* CPU */}
            <div className="hw-card spec-cpu">
              <div className="hw-card-icon"><Cpu size={20} /></div>
              <div className="hw-card-info">
                <span className="hw-label">Processor</span>
                <span className="hw-value">{hardware.cpu_model}</span>
                <div className="hw-util-row">
                  <div className="progress-container" style={{ flex: 1 }}>
                    <div
                      className="progress-bar cpu"
                      style={{ width: `${Math.min(100, Math.round(liveMetrics?.cpu_percent ?? 0))}%` }}
                    />
                  </div>
                  <span className="hw-util-pct">{Math.round(liveMetrics?.cpu_percent ?? 0)}%</span>
                </div>
                <span className="hw-sub">{hardware.cpu_cores} cores · CPU utilisation</span>
              </div>
            </div>

            {/* RAM */}
            <div className="hw-card spec-ram">
              <div className="hw-card-icon"><Server size={20} /></div>
              <div className="hw-card-info">
                <span className="hw-label">System Memory</span>
                <span className="hw-value">
                  {liveMetrics
                    ? `${(liveMetrics.ram_used_mb / 1024).toFixed(1)} / ${(liveMetrics.ram_total_mb / 1024).toFixed(1)} GB`
                    : `${(hardware.ram_total_mb / 1024).toFixed(1)} GB RAM`}
                </span>
                <div className="progress-container">
                  <div
                    className="progress-bar"
                    style={{
                      width: liveMetrics
                        ? `${Math.min(100, Math.round((liveMetrics.ram_used_mb / liveMetrics.ram_total_mb) * 100))}%`
                        : `${Math.round(((hardware.ram_total_mb - hardware.ram_available_mb) / hardware.ram_total_mb) * 100)}%`
                    }}
                  />
                </div>
                <span className="hw-sub">
                  {liveMetrics
                    ? `${(( liveMetrics.ram_total_mb - liveMetrics.ram_used_mb) / 1024).toFixed(1)} GB free · ${Math.round((liveMetrics.ram_used_mb / liveMetrics.ram_total_mb) * 100)}% used`
                    : `${(hardware.ram_available_mb / 1024).toFixed(1)} GB available`}
                </span>
              </div>
            </div>

            {/* GPU / VRAM */}
            <div className="hw-card spec-gpu">
              <div className="hw-card-icon"><Zap size={20} /></div>
              <div className="hw-card-info">
                <span className="hw-label">Graphics Card</span>
                <span className="hw-value">{hardware.gpu_name || 'No Dedicated GPU'}</span>

                {hardware.vram_total_mb ? (
                  <>
                    {/* Single row: GPU util % + VRAM label combined */}
                    <div className="hw-gpu-meta-row">
                      {liveMetrics?.gpu_percent != null && (
                        <span className="hw-gpu-util-badge">
                          {Math.round(liveMetrics.gpu_percent)}% GPU
                        </span>
                      )}
                      <span className="hw-vram-label">
                        {liveMetrics?.vram_used_mb != null
                          ? `${(liveMetrics.vram_used_mb / 1024).toFixed(1)} / ${(hardware.vram_total_mb / 1024).toFixed(1)} GB VRAM`
                          : `${(hardware.vram_total_mb / 1024).toFixed(1)} GB VRAM`}
                      </span>
                    </div>

                    {/* Single VRAM progress bar */}
                    <div className="progress-container gpu">
                      <div
                        className="progress-bar gpu"
                        style={{
                          width: liveMetrics?.vram_used_mb != null
                            ? `${Math.min(100, Math.round((liveMetrics.vram_used_mb / hardware.vram_total_mb) * 100))}%`
                            : '0%'
                        }}
                      />
                    </div>
                    <span className="hw-sub">
                      {liveMetrics?.vram_used_mb != null
                        ? `${((hardware.vram_total_mb - liveMetrics.vram_used_mb) / 1024).toFixed(1)} GB free · ${Math.round((liveMetrics.vram_used_mb / hardware.vram_total_mb) * 100)}% used`
                        : `${(hardware.vram_total_mb / 1024).toFixed(1)} GB total`}
                    </span>
                  </>
                ) : (
                  <span className="hw-sub">Vendor: {hardware.gpu_vendor}</span>
                )}
              </div>
            </div>

            {/* Capability Tier */}
            <div className="hw-card spec-tier">
              <div className="hw-card-icon"><ShieldCheck size={20} /></div>
              <div className="hw-card-info">
                <span className="hw-label">Capability Tier</span>
                <span className="hw-value tier-badge">{hardware.tier} Tier</span>
                <span className="hw-sub">{hardware.memory_mode_reason}</span>
              </div>
            </div>

          </div>
        </section>
      )}

      {/* Installed Models */}
      <section className="models-section">
        <h3 className="section-title">Installed Models</h3>
        {models.length === 0 ? (
          <div className="empty-models">
            <p>No models detected in Ollama. Pull a model using the search bar above to begin.</p>
          </div>
        ) : (
          <div className="model-grid">
            {models.map(model => {
              const suitability = getSuitabilityScore(model);
              return (
                <div key={model.name} className={`model-rich-card ${activeModel === model.name ? 'active' : ''}`}>
                  <div className="model-header-row">
                    <div className="model-title-group">
                      {activeModel === model.name && <span className="active-glow-dot" />}
                      <span className="model-rich-name">{model.name}</span>
                    </div>
                    <div className="model-loaded-badge">
                      {activeModel === model.name ? (
                        <span className="badge loaded">Active</span>
                      ) : (
                        <span className="badge idle">Standby</span>
                      )}
                    </div>
                  </div>

                  {model.is_cloud && (
                    <div className="model-cloud-badge">
                      <span className="cloud-icon">☁</span>
                      <span className="cloud-label">
                        Cloud — {model.cloud_provider || 'Remote API'}
                      </span>
                      <span className="cloud-warning">
                        ⚠ Requires internet
                      </span>
                    </div>
                  )}

                  {/* Spec grid */}
                  <div className="model-spec-table">
                    <div className="spec-item">
                      <span className="spec-lbl">PARAMETERS</span>
                      <span className="spec-val">{formatParams(model.param_count)}</span>
                    </div>
                    <div className="spec-item">
                      <span className="spec-lbl">QUANTIZATION</span>
                      <span className="spec-val">{model.quantization || 'Unknown'}</span>
                    </div>
                    <div className="spec-item">
                      <span className="spec-lbl">CONTEXT LIMIT</span>
                      <span className="spec-val">{(model.context_length || 2048).toLocaleString()} ctx</span>
                    </div>
                    {!model.is_cloud && (
                      <>
                        <div className="spec-item">
                          <span className="spec-lbl">FILE SIZE</span>
                          <span className="spec-val">{formatSize(model.size)}</span>
                        </div>
                        <div className="spec-item">
                          <span className="spec-lbl">EST. VRAM/RAM</span>
                          <span className="spec-val">~{(model.estimated_ram_mb / 1024).toFixed(1)} GB</span>
                        </div>
                      </>
                    )}
                    <div className="spec-item">
                      <span className="spec-lbl">ARCHITECTURE</span>
                      <span className="spec-val">{model.architecture || 'Unknown'}</span>
                    </div>
                  </div>

                  {/* Suitability Score Banner */}
                  <div className={`suitability-banner ${suitability.color}`}>
                    <div className="banner-title">
                      <HelpCircle size={12} />
                      <strong>{suitability.label}</strong>
                    </div>
                    <div className="banner-desc">{suitability.desc}</div>
                  </div>

                  {/* Recommendations */}
                  {model.recommended && (
                    <div className="recommendation-notice">
                      <Star size={12} className="star-icon" />
                      <span>{model.recommendation_reason}</span>
                    </div>
                  )}

                  {/* Actions */}
                  <div className="model-card-actions">
                    {activeModel === model.name ? (
                      <button className="model-act-btn unload" onClick={handleUnload}>
                        <Square size={12} /> Unload Model
                      </button>
                    ) : (
                      <button className="model-act-btn load" onClick={() => handleLoad(model.name)}>
                        <Play size={12} /> Load Model
                      </button>
                    )}
                    <button className="model-act-btn danger" onClick={() => handleDelete(model.name)}>
                      <Trash2 size={12} /> Delete
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>

      {/* Recommended Models */}
      {recommended.length > 0 && (
        <section className="models-section">
          <h3 className="section-title"><Star size={14} /> Recommended Models for your system</h3>
          <div className="recommended-grid">
            {recommended.map(rec => (
              <div key={rec.name} className="recommended-card-premium">
                <div className="rec-info">
                  <div className="rec-name-row">
                    <Star size={14} className="star-icon" />
                    <span className="rec-model-name">{rec.name}</span>
                  </div>
                  <div className="rec-details">
                    <span>Context Length: {rec.context_length.toLocaleString()} tokens</span>
                  </div>
                </div>
                <button className="rec-pull-btn" onClick={() => handlePull(rec.name)}>
                  <Plus size={12} /> Pull Model
                </button>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
};