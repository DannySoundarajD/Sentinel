# Sentinel

> Local-first AI agent for Linux desktops. Private. Fast. Yours.

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux-orange)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange)
![Status](https://img.shields.io/badge/status-beta-yellow)

## Preview

Sentinel running on Arch Linux + XFCE:

![Chat Interface](screenshots/chat.png)
![Guardian Monitor](screenshots/guardian.png)
![Models Page](screenshots/models.png)

---

Sentinel is a minimal, privacy-first AI assistant that runs entirely
on your Linux machine using local Ollama models.
No cloud. No tracking. No subscriptions.

---

## Features

- Chat with local AI models via Ollama
- Vault memory system (Lite and Pro modes)
- Guardian resource monitor (CPU / RAM / GPU / Temperature)
- Super+Space global hotkey (open anywhere)
- System tray integration
- Telegram bridge (/chat, /status, /memory, /model)
- Skill plugin system
- Context window management for low-spec hardware
- Works on 4GB RAM with gemma4:e2b

---

## Requirements

- Linux (Arch, Ubuntu, Fedora, Debian, Mint)
- [Ollama](https://ollama.ai) installed and running
- Rust 1.70+
- Node.js 18+

---

## Quick Start

### Arch Linux
```bash
sudo pacman -S ollama nodejs npm xdotool libnotify
```

### Ubuntu / Debian
```bash
sudo apt install nodejs npm xdotool libnotify-bin
curl -fsSL https://ollama.ai/install.sh | sh
```

### Fedora
```bash
sudo dnf install ollama nodejs npm xdotool libnotify
```

### Installation & Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/DannySoundarajD/Sentinel.git
   cd Sentinel
   ```

2. **Install frontend UI dependencies**:
   ```bash
   cd sentinel-ui
   npm install
   cd ..
   ```

3. **Run in Development Mode**:
   This runs the Rust daemon and launching frontend in dev mode automatically:
   ```bash
   ./dev-launch.sh
   ```

4. **Production Build** (Optional):
   To compile the release target:
   ```bash
   cargo build --release
   ```
   You can then run the compiled binary `target/release/sentinel` or add it to your desktop's application startup (e.g., in XFCE Application Autostart).

---

## Usage

| Action | How |
|--------|-----|
| Open Sentinel | Super + Space |
| Hide Sentinel | Super + Space again |
| Pull a model | Models tab → type model name → Pull |
| View Hardware Profile | Models tab → Hardware Profile Dashboard |
| Save a memory | Type /save in chat |
| Search memory | Memory tab → search bar |
| View resource usage | Guardian tab |

### Keyboard Shortcut Setup (XFCE / Linux)

To configure `Super+Space` to launch and toggle Sentinel:

1. **Add Keyboard Shortcut**:
   - Open **Settings** → **Keyboard** → **Application Shortcuts**.
   - Click **Add** and specify the path to `dev-launch.sh` (e.g. `/home/danny/Desktop/Sentinel/dev-launch.sh`).
   - Press **Super + Space** to bind the shortcut.
   - *(Note: `dev-launch.sh` will automatically start the app if closed, or toggle visibility if it is already running).*

2. **Resolve Conflicts**:
   - Ensure no other default application (such as the Whisker Menu or Application Finder) intercepts **Super + Space**. If there's a conflict, remove or modify the default shortcut in Settings.

---

## Memory System — Vault

Sentinel uses a purpose-built memory system called **Vault**.
It is fundamentally different from how memory works in other agent systems like OpenClaw and Hermes Agent.

---

### How Other Systems Handle Memory

**OpenClaw**
- Similar vector-first approach
- Memory retrieval depends on embedding quality
- If embedding model is wrong or missing, retrieval breaks entirely
- No concept of memory priority or budget
- Injects everything it finds into context with no size limit
- On small models (2K context): context overflow causes silent truncation
  or hallucination

**Hermes Agent**
- Session-based memory only
- No persistence across restarts by default
- Relies on external Redis or vector store for persistence
- No awareness of model context window size
- Memory and model are tightly coupled —
  changing the model breaks the memory retrieval

---

### How Vault Works Differently

Vault is built on three principles that the above systems ignore:

**1. The model does not own memory. Vault does.**

In standard agent architectures like Hermes, memory is part of the agent runtime.
If you switch models, memory behavior changes.
In Sentinel, Vault is completely separate from the model.
You can swap Ollama models freely — Vault never changes.

**2. No vector database. No embedding model. No external services.**

Vault uses SQLite only. It runs in the same process as the backend.
Zero extra services to install or manage.
On a 4GB RAM laptop, this matters enormously.

| System        | Memory Backend       | Extra Services Needed      |
|---------------|---------------------|---------------------------|
| OpenClaw      | Vector store        | Embedding model required   |
| Hermes        | Redis / vector      | Redis server               |
| **Sentinel**  | **SQLite (Vault)**  | **None**                   |

**3. Context budget awareness.**

Every system above injects memory into context without checking if it will fit. On a model with 2048 tokens, this silently breaks things.

Vault knows the context window of the active model before it builds context. It assembles memory in strict priority order and stops when the budget runs out. Furthermore, in the `/frommemory <query>` command, Sentinel performs a keyword-relevance ranking and caps the memories block strictly to the model's remaining context window to prevent hallucinations and context-window overflows.

---

### Vault Priority System

Before every prompt, Vault runs this assembly in order. Lower priority layers are dropped automatically if context is full:

```
Priority 1 — Current user prompt         (never dropped)
Priority 2 — User preferences            (tiny, almost always fits)
Priority 3 — Recent chat (last 10 turns) (trimmed from front if needed)
Priority 4 — Relevant memory nodes       (top ranked matches by relevance score)
Priority 5 — Conversation summary        (most recent, truncated)
Priority 6 — Workflow context            (dropped first if tight)
```

On a model with 2048 token context window, the math looks like this:

```
Total context:      2048 tokens
System reserve:      200 tokens  (instructions)
Response reserve:    512 tokens  (model reply space)
Available for Vault: 1336 tokens

Allocated:
User prompt:        ~50 tokens  (depends on message)
Preferences:        ~80 tokens
Recent history:    ~600 tokens  (last turns, trimmed)
Memory nodes:      ~300 tokens  (ranked nodes fitting remaining budget)
Summary:           ~200 tokens  (truncated to fit)
Remaining:         ~106 tokens  (buffer)
```

The model never sees a context it cannot handle. No silent truncation. No hallucination from overflow.

---

### Budget-Aware Ranked Memory Retrieval (`/frommemory`)

To handle large memory databases without overflowing the context window or causing model hallucinations, `/frommemory` employs a robust ranked retrieval system:

1. **Stop-Word Filtering**: Common prepositions and pronouns (like "who", "is", "what", "are", "the", "in", "to") are stripped from the query to isolate search keywords.
2. **Exact Word Matching**: Implements boundary-checking (`is_exact_word_match`) to distinguish between distinct words (e.g. matching "leo" in "leo is a thief" but ignoring it in "leopold").
3. **Keyword-Relevance Scoring**:
   - Exact word match in name: **+15 points**
   - Exact word match in description: **+10 points**
   - Substring match in name: **+5 points**
   - Substring match in description: **+3 points**
4. **Relevance Ranking**: Memories are sorted by their relevance score descending, using database ID (recency) descending as a tie-breaker. Memories with 0 score are discarded if a search query is present.
5. **Dynamic Budget Capping**: The prompt assembler calculates the remaining token budget by subtracting the sizes of static instructions, current conversational history, and query prompt prefixes. It injects the highest-ranked memories into the prompt context, stopping immediately once the budget is exhausted.

---

### Vault Modes

**Lite Mode** — for 4 to 8GB RAM

Designed for daily use on modest hardware.

- Fresh session by default — no automatic memory injection
- Memory only stored when you use `/save`
- Memory only retrieved when you use `/frommemory`
- Zero overhead — raw prompt goes straight to Ollama
- Works on models as small as `tinyllama:1.1b` (2048 tokens)

```
User types: /save Danny prefers dark mode
Vault stores: key="Danny prefers dark mode" in SQLite

Later session:
User types: /frommemory preferences
Vault ranks, budgets, and injects matching nodes into next prompt
```

**Pro Mode** — for 16GB+ RAM

Designed for power users who want persistent context across sessions.

- After every conversation Vault extracts key entities automatically
- Builds a graph of nodes and relationships in SQLite
- Before every prompt searches the graph for relevant context
- Assembles and injects context within the token budget
- Conversation summaries stored and retrieved automatically

```
Graph example after a week of use:

DannySoundarajD
├── works on → SentinX OS
├── prefers → dark mode, Arch Linux, XFCE
├── building → Sentinel, SecureCall, AI SoleMate
└── uses → gemma:2b, phi3:mini

Before next prompt about "SentinX":
Vault finds: SentinX node + relationships + past summary
Injects only what fits in the token budget
Model receives rich context without overflow
```

---

### Database Schema

Everything lives in a single SQLite file at:
`~/.local/share/sentinx/sentinel/vault.db`

```sql
-- Memory graph nodes
CREATE TABLE memory_nodes (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    type        TEXT NOT NULL,
    description TEXT
);

-- Relationships between nodes
CREATE TABLE memory_edges (
    id        INTEGER PRIMARY KEY,
    source_id INTEGER REFERENCES memory_nodes(id),
    target_id INTEGER REFERENCES memory_nodes(id),
    relation  TEXT NOT NULL,
    weight    REAL DEFAULT 1.0
);

-- Conversation summaries
CREATE TABLE conversation_summaries (
    id        INTEGER PRIMARY KEY,
    title     TEXT,
    summary   TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

-- User preferences
CREATE TABLE preferences (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Chat history (persists across restarts)
CREATE TABLE chat_history (
    id        TEXT PRIMARY KEY,
    role      TEXT NOT NULL,
    content   TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);
```

No Qdrant. No Redis. No PostgreSQL. No embedding server.
One file. Always available. Instant startup.

---

### Why This Matters on Low-Spec Hardware

A typical vector-database-based setup on a 4GB laptop:

```
Qdrant server:        ~300 MB RAM
Embedding model:      ~500 MB RAM
Agent runtime:        ~200 MB RAM
Ollama + model:      ~1600 MB RAM
Total:               ~2600 MB RAM  →  system starts swapping
```

Sentinel on the same 4GB laptop:

```
Sentinel daemon:       ~30 MB RAM
Ollama + gemma:2b:  ~1600 MB RAM
Total:              ~1630 MB RAM  →  970 MB still free
```

Vault adds zero meaningful overhead because SQLite
runs inside the Sentinel process with no separate service.

---

### Vault vs Others — Summary Table

| Feature                        | OpenClaw | Hermes | Sentinel Vault |
|-------------------------------|----------|--------|----------------|
| Persistent memory             | ✅       | ⚠️     | ✅             |
| Survives model switch         | ❌       | ❌     | ✅             |
| Works without extra services  | ❌       | ❌     | ✅             |
| Context window awareness      | ❌       | ❌     | ✅             |
| Works on 4GB RAM              | ❌       | ⚠️     | ✅             |
| Manual memory control         | ❌       | ⚠️     | ✅ (/save)     |
| Graph relationships           | ✅       | ❌     | ✅ (Pro mode)  |
| Zero config setup             | ❌       | ❌     | ✅             |
| Offline capable               | ✅       | ⚠️     | ✅             |

⚠️ = partial or requires specific configuration

---

## Model Recommendations & Hardware Profile

Sentinel features a built-in **Hardware Profile Dashboard** inside the **Models** tab that actively scans your local system configuration:
- **Processor**: Detects logical cores and architecture
- **System Memory**: Shows total RAM vs available RAM
- **Graphics Card**: Detects VRAM, utilization, and vendor (Nvidia, AMD, Intel)
- **Capability Tier**: Automatically categorizes your hardware (e.g. Minimal, Low, Medium, High, Ultra)

Based on this hardware profile, Sentinel automatically scores and recommends installed models. No manual configuration needed.

General guidance:

| Available RAM | Suggested Model Type      | Why                          |
|---------------|--------------------------|------------------------------|
| Under 4 GB    | tinyllama:1.1b, gemma:2b | Fits in RAM, fast startup    |
| 4–8 GB        | gemma:2b, phi3:mini      | Good quality, manageable RAM |
| 8–16 GB       | gemma4:e2b, mistral:7b   | Strong output, GPU hybrid    |
| 16 GB+        | qwen3.6, mistral:7b      | Full quality, Pro mode safe  |

Cloud models (e.g. gemma4:31b-cloud) are detected automatically 
and shown separately — they need zero local RAM but require 
internet connection.

---

## Architecture

```
User
  ↓
Electron UI  (React + Vite on :51793+, dynamic)
  ↓
Rust API     (Axum on :8888+, dynamic)
  ├── /chat        SSE streaming
  ├── /runtime     Ollama model management
  ├── /vault       SQLite memory
  └── /guardian    System metrics
  ↓
Ollama       (localhost:11434)
```

---

## Project Structure

```
Sentinel/
├── src/                  Rust backend
│   ├── api/              25 HTTP endpoints
│   ├── vault/            Memory system + token budget
│   ├── runtime/          Ollama integration
│   ├── guardian/         Resource monitoring
│   ├── bridge/           Telegram bot
│   ├── hotkey/           Display server detection module
│   └── notifications/    Desktop notifications
├── sentinel-ui/          Electron + React frontend
│   ├── electron/         Main process + preload
│   ├── wait-and-launch.cjs Dynamic port resolver and launcher
│   └── src/pages/        Chat, Models, Guardian, Memory, Skills, Settings
├── dev-launch.sh         Development launcher
└── README.md
```

---

## License

MIT — see [LICENSE](LICENSE)


