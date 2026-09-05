# DEYANA System Architecture Specification (v2.0 Full-Stack AI Edition)

This document provides the definitive architectural specification, subsystem contracts, mathematical execution models, and data security boundaries of **Deyana** — an on-device, privacy-enforced, local-first personal and workspace AI assistant.

Deyana is implemented as a single compiled native Rust binary (`apps/desktop/src-tauri`) with a high-performance React 18 / TypeScript floating holographic HUD.

---

## 1. Executive Architectural Overview

Deyana operates directly over sensitive user context — local source code repositories, personal markdown notes, encrypted chat transcripts, active desktop window focus states, and authorized workplace connectors (Gmail, GitHub, Slack, Notion, Jira, Linear) — without transmitting private data to cloud AI services.

### 1.1 Non-Negotiable System Invariants

1. **Zero External AI Egress:** Private context (files, chat history, vector embeddings, vault notes, voice transcripts, and active window titles) is strictly blocked from external cloud AI APIs.
2. **Single-Binary Process Lifecycle:** All business logic, state management, retrieval indexing, background daemons, and protocol handlers reside in a single compiled Rust binary (`deyana-desktop.exe`). There are no Python runtimes, Node.js background services, or loopback HTTP servers for internal communication.
3. **Hardware-Aware Execution:** Subsystems automatically detect host hardware capabilities, maintaining a sub-100MB idle RAM footprint and adapting context windows from 2,048 tokens on low-spec laptops up to 128,000 tokens on multi-GPU workstations.
4. **Hardware-Backed Encryption:** All sensitive database records (chat history, personal notes, semantic cache) are encrypted at rest using AES-256-GCM with keys protected by the OS (Windows DPAPI).
5. **Continuous Fine-Tuning Flywheel:** High-quality local interactions are automatically logged with regex PII masking and can be exported with 1 click into ShareGPT or DPO preference JSONL for local model training.

### 1.2 High-Level System Architecture

```mermaid
flowchart TD
    subgraph UI["React 18 Desktop HUD (WebView2 Shell)"]
        HUD["Holographic Glass Floating Panel<br/>(Framer Motion + TailwindCSS)"]
        ChatUI["LocalChat & Expandable Scratchpad<br/>(AgentStepBadge.tsx)"]
        ModelUI["Unified ModelSetupPanel<br/>(Runtimes | GGUF Hub | Flywheel)"]
        VoiceViz["Voice Waveform & Status Ring"]
        ProactiveUI["Proactive Context Cards"]
    end

    subgraph Rust["Tauri 2 Rust Native Core Process (deyana-desktop.exe)"]
        ReAct["agent::react<br/>Autonomous ReAct Engine<br/>(Thought ➔ Action ➔ Observation)"]
        Grammar["agent::grammar<br/>GBNF Grammar Engine<br/>Token-Level Tool Constraints"]
        Dispatcher["agent::dispatcher<br/>Native Tool Dispatcher<br/>(Memory, Git, Web, Notes)"]
        Retrieval["agent::retrieval<br/>Hybrid Dual-Mode RAG<br/>FTS5 BM25 + Cosine Similarity"]
        Cache["agent::cache<br/>Semantic LRU Cache<br/>Embedding + Context Hash"]
        
        LlamaCpp["models::llamacpp<br/>Managed llama-server Supervisor<br/>(SSE Streaming & Subprocess Control)"]
        Ollama["models::ollama<br/>Ollama Native Client<br/>(Port 11434 HTTP Bridge)"]
        Downloader["models::downloader<br/>HuggingFace GGUF Downloader<br/>RAM Estimator & Streamer"]
        Flywheel["models::dataset<br/>Continuous Fine-Tuning Flywheel<br/>PII Masker & JSONL Exporter"]
        Hardware["models::hardware<br/>Hardware Auto-Detector<br/>4-Tier Profile Mapper (sysinfo)"]
        
        Privacy["privacy::firewall<br/>Deterministic Firewall<br/>SHA-256 Audit Trail"]
        Crypto["privacy::crypto & vault_key<br/>Windows DPAPI + AES-256-GCM<br/>Field-Level Encryption"]
        
        DB["db/<br/>SQLite WAL Mode Engine<br/>FTS5 + Vector + Encrypted Repos"]
        Daemon["daemon/<br/>Proactive Sentinel Daemon<br/>Win32 Active Window Poller"]
        Voice["voice/<br/>Always-On Voice Pipeline<br/>rustpotter Wake + VAD + Windows TTS"]
        MCP["mcp/<br/>Dual MCP Client & Axum Server<br/>Tool Registry Bridge"]
    end

    subgraph HostEnv["Host Operating System & Services"]
        LlamaServerBin["llama-server.exe<br/>(Spawned Subprocess)"]
        OllamaService["Ollama Service<br/>(System Background Service)"]
        Win32API["Windows APIs<br/>(DPAPI, Win32 Window, SAPI TTS)"]
        McpServers["External MCP Servers<br/>(stdio / SSE transports)"]
        Connectors["OAuth External APIs<br/>(Gmail, GitHub, etc.)"]
    end

    UI <-->|"Tauri IPC (Commands & agent:step Events)"| ReAct
    UI <-->|"Tauri IPC (model management & flywheel)"| ModelUI
    
    ReAct --> Grammar
    ReAct --> Dispatcher
    ReAct --> Retrieval
    ReAct --> Cache
    ReAct --> LlamaCpp
    ReAct --> Ollama
    
    LlamaCpp <-->|"HTTP / SSE"| LlamaServerBin
    Ollama <-->|"HTTP API"| OllamaService
    Downloader -->|"HTTPS (HuggingFace Hub)"| Privacy
    
    Flywheel --> DB
    Retrieval --> DB
    Crypto <-->|"CryptProtectData / CryptUnprotectData"| Win32API
    DB --> Crypto
    
    Voice <-->|"WASAPI / SAPI"| Win32API
    Daemon <-->|"GetForegroundWindow"| Win32API
    MCP <--> McpServers
    Dispatcher -.->|"Approved Calls"| Connectors
```

---

## 2. Process Topology & Concurrency Model

Deyana utilizes a multi-threaded, asynchronous runtime architecture powered by Tokio and Tauri 2:

```mermaid
flowchart TD
    subgraph Process["OS Process Space: deyana-desktop.exe"]
        subgraph MainThread["Main OS Thread"]
            WinMain["Tauri GUI Event Loop<br/>Frameless Transparent Window Management<br/>System Tray & Win32 Window Snapping"]
        end

        subgraph AudioThread["Dedicated Real-Time Audio Thread (std::thread)"]
            WASAPI["cpal WASAPI Stream Engine<br/>Continuous 16kHz Mono Ring Buffer"]
            VAD["rustpotter Wake Detector & SilenceEndpointer<br/>Energy RMS Voice Activity Detection"]
            WASAPI --> VAD
        end

        subgraph TokioPool["Tokio Asynchronous Multi-Threaded Runtime Pool"]
            ReActTask["agent::react<br/>ReAct Loop Task (Max 5 Steps)"]
            LlamaSupervisor["models::llamacpp<br/>llama-server Process Supervisor"]
            HFDownloader["models::downloader<br/>Chunked HTTP Streaming Task"]
            DaemonTask["daemon::poller<br/>60s Active Window Poller"]
            McpAxum["mcp::server<br/>Axum SSE Server Task"]
            DBPool["db::pool<br/>r2d2 SQLite WAL Connection Pool"]
        end
    end

    WinMain <-->|"IPC Messages"| TokioPool
    VAD -->|"mpsc::channel (wake/speech)"| ReActTask
    LlamaSupervisor <-->|"Stdio & HTTP Health"| LlamaServerBin
```

### Threading & Subsystem Isolation Guarantees
- **UI Responsiveness:** Heavy LLM inference, embedding generation, and GGUF downloads never block the main GUI thread.
- **Audio Protection:** Microphone sampling and neural wake word detection execute on a dedicated native OS thread (`std::thread`), completely insulated from Tokio runtime task preemption.
- **Database Concurrency:** SQLite operates in Write-Ahead Logging (`PRAGMA journal_mode=WAL;`) with an `r2d2` thread pool, allowing concurrent readers alongside writes.

---

## 3. Subsystem Specifications

### 3.1 Autonomous Multi-Turn ReAct Agent Engine (`agent/react.rs`)

Deyana features a full **Reasoning + Acting (ReAct)** autonomous agent engine that enables iterative problem solving:

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant HUD as React HUD (LocalChat)
    participant Engine as ReActEngine (Rust)
    participant Grammar as GBNF Grammar Generator
    participant Router as LLM Router (LlamaCpp / Ollama)
    participant Dispatcher as Tool Dispatcher
    participant DB as Encrypted SQLite Store

    User->>HUD: "Check git status and summarize recent memory notes"
    HUD->>Engine: agent_chat(prompt, context)
    
    loop ReAct Loop (Up to MAX_REACT_STEPS = 5)
        Engine->>Grammar: Build tool whitelist grammar
        Engine->>Router: Generate with Scratchpad context
        Router-->>Engine: LLM Output (Thought + ToolCall OR FinalAnswer)
        
        alt Model Emits Tool Call
            Engine->>HUD: emit("agent:step", { type: "tool_call", name, args })
            Engine->>Dispatcher: dispatch(tool_name, args)
            Dispatcher-->>Engine: ToolRunResponse (Observation)
            Engine->>HUD: emit("agent:step", { type: "observation", content })
            Engine->>Engine: Append Thought + Action + Observation to Scratchpad
        else Model Emits Final Answer OR Max Steps (5) Exceeded
            Engine->>HUD: emit("agent:step", { type: "final_answer", content })
            Engine->>DB: Log conversation turn to training_dataset (PII masked)
            Engine-->>HUD: Return synthesized Final Answer
        end
    end
```

#### ReAct Execution Invariants
- **Step Bound:** Hard-capped at `MAX_REACT_STEPS = 5` to prevent infinite execution loops.
- **Grammar-Constrained Schema:** The GBNF engine ensures that tool calls strictly adhere to the schema:
  ```json
  {"thought": "reasoning...", "tool_call": {"name": "tool_name", "arguments": {}}}
  ```
- **Robust Recovery:** If the LLM generates malformed JSON or invokes an invalid tool, the engine synthesizes an error observation (e.g. `"Tool 'xyz' not found. Available: [...]"`) and feeds it back to the model for self-correction.

---

### 3.2 Dual-Mode Memory & Context Engineering (`agent/retrieval.rs`, `agent/cache.rs`, `db/`)

Memory retrieval balances speed and semantic depth through hybrid rank fusion:

```mermaid
flowchart LR
    Query["User Query"] --> FTS["SQLite FTS5<br/>BM25 Lexical Scoring"]
    Query --> Embed["Ollama / llama.cpp<br/>384-dim Embedding"]
    Embed --> Vector["SQLite Vector Store<br/>Cosine Similarity Scan"]
    
    FTS --> Fusion["Reciprocal Rank Fusion<br/>Score = RRF(BM25) + RRF(Vector)"]
    Vector --> Fusion
    Fusion --> Context["Compressed Context Payload<br/>(budgeted to active model n_ctx)"]
```

#### Mathematical Retrieval Models
1. **Vector Cosine Similarity:**
   $$\text{sim}(\mathbf{u}, \mathbf{v}) = \frac{\mathbf{u} \cdot \mathbf{v}}{\|\mathbf{u}\|_2 \|\mathbf{v}\|_2} = \frac{\sum_{i=1}^d u_i v_i}{\sqrt{\sum_{i=1}^d u_i^2} \sqrt{\sum_{i=1}^d v_i^2}}$$
2. **Semantic Cache Lookup:**
   In-memory LRU + SQLite cache keyed by query vector and context hash. If cosine distance $\ge 0.96$, the cached answer returns with zero LLM inference latency.

---

### 3.3 Inference Engine & Hardware Auto-Detection (`models/`)

Deyana supports dual inference runtimes:
1. **Managed llama.cpp Engine (`models/llamacpp.rs`):** Launches and monitors `llama-server.exe` as a supervised child process with real-time SSE streaming.
2. **Ollama Client (`models/ollama.rs`):** HTTP connection to local Ollama on port `11434`.

#### Hardware Auto-Detection Profiles (`models/hardware.rs`)
Host RAM is probed via `sysinfo` on startup and mapped to four operational profiles:

| Profile | RAM Threshold | Recommended Models | Max Context Window | Quantization |
|---|---|---|---|---|
| **Balanced** | 8 – 16 GB | `qwen3:1.7b`, `llama3.2:3b` | 4,096 tokens | `Q4_K_M` |
| **Standard** | 16 – 32 GB | `llama3.3:8b`, `qwen2.5-coder:7b` | 8,192 tokens | `Q4_K_M` / `Q5_K_M` |
| **Power** | 32 – 64 GB | `qwen2.5:14b`, `deepseek-r1:14b` | 16,384 tokens | `Q5_K_M` / `Q8_0` |
| **Ultra** | 64+ GB / Multi-GPU | `qwen2.5:32b`, `llama3.3:70b` | 32,768 – 128,000 tokens | `Q4_K_M` / `Q8_0` |

#### KV-Cache & Host RAM Estimation Formula (`models/downloader.rs`)
Before downloading GGUF files from HuggingFace Hub, Deyana calculates the exact memory required to run the model safely:

$$\text{RAM}_{\text{req}} = \text{Model Weight Size (Bytes)} + \text{KV Cache Size} \times 1.25$$

Where the KV-Cache memory is computed as:
$$\text{KV Cache Size} = 2 \times n_{\text{layers}} \times n_{\text{heads}} \times d_{\text{head}} \times n_{\text{ctx}} \times \text{bytes per element}$$

---

### 3.4 Hardware-Backed Cryptographic Security (`privacy/`)

To protect user confidentiality against physical disk theft and unauthorized local process inspection:

```mermaid
flowchart TD
    Raw["Plaintext Record<br/>(Chat Message / Memory Note)"]
    DPAPI["Windows DPAPI<br/>(CryptProtectData via user logon credentials)"]
    MasterKey["32-Byte Master Key<br/>(in-memory VaultKeyState)"]
    AES["AES-256-GCM Cipher<br/>(Unique 12-byte CSPRNG Nonce)"]
    Envelope["ENC::<nonce_hex>::<ciphertext_hex>"]
    Disk["SQLite Database File<br/>(deyana.db)"]

    DPAPI -->|Unwraps on startup| MasterKey
    Raw --> AES
    MasterKey --> AES
    AES --> Envelope
    Envelope --> Disk
```

#### Cryptographic Specifications
- **Master Key Security:** Wrapped via Windows Data Protection API (DPAPI) and persisted to disk. Unwrapping requires the active user's Windows logon session.
- **Cipher:** AES-256-GCM with a fresh, cryptographically secure random 12-byte nonce per encryption operation.
- **Envelope Format:** `ENC::<12_BYTE_HEX_NONCE>::<CIPHERTEXT_AND_TAG_HEX>`. Unencrypted legacy records pass through seamlessly during migration.

---

### 3.5 Continuous Fine-Tuning Flywheel (`models/dataset.rs`)

Deyana includes a built-in data flywheel that logs interactions to create custom fine-tuning datasets:

```mermaid
flowchart LR
    Interaction["Chat Turn / ReAct Step"] --> PII["Regex PII Scrubber<br/>([EMAIL], [PHONE], [API_KEY], [JWT])"]
    PII --> DB["training_dataset Table"]
    
    DB --> ExportShareGPT["1-Click Export:<br/>ShareGPT Multi-Turn JSONL"]
    DB --> ExportDPO["1-Click Export:<br/>DPO Chosen/Rejected JSONL"]
    
    ExportShareGPT --> Training["Local Fine-Tuning<br/>(Unsloth / LoRA / Ollama Modelfile)"]
    ExportDPO --> Training
```

#### Supported Dataset Formats
1. **ShareGPT Format:** Multi-turn conversational JSONL (`conversations: [{from: "human", value: ...}, {from: "gpt", value: ...}]`).
2. **DPO / ORPO Format:** Direct Preference Optimization pairs (`prompt`, `chosen`, `rejected`) using user-edited responses as chosen targets.

---

## 4. Desktop Frontend Architecture

The user interface is a frameless, holographic floating overlay built in React 18, TailwindCSS, and Framer Motion:

```mermaid
flowchart TD
    subgraph Frontend["React 18 Desktop HUD (apps/desktop/src/)"]
        App["App.tsx<br/>Shell & MotionConfig"]
        FloatingPanel["components/floating/FloatingPanel.tsx<br/>Holographic Glass Surface"]
        
        LocalChat["components/floating/LocalChat.tsx<br/>Streaming Chat Stream"]
        StepBadge["components/chat/AgentStepBadge.tsx<br/>Animated ReAct Step Badges"]
        
        ModelSetup["components/floating/ModelSetupPanel.tsx<br/>Segmented 3-Tab Controller"]
        RuntimesTab["Tab 1: Local Runtimes<br/>Ollama / llama.cpp status"]
        GgufHubTab["Tab 2: HuggingFace GGUF Hub<br/>RAM Gauge & Chunked Downloader"]
        FlywheelTab["Tab 3: Fine-Tuning Flywheel<br/>Stats, PII Preview & 1-Click Export"]
        
        Store["stores/assistantStore.ts<br/>Zustand-like Reactive State Manager"]
    end

    App --> FloatingPanel
    FloatingPanel --> LocalChat
    LocalChat --> StepBadge
    FloatingPanel --> ModelSetup
    ModelSetup --> RuntimesTab
    ModelSetup --> GgufHubTab
    ModelSetup --> FlywheelTab
    Store <-->|"Tauri IPC Events (agent:step, llm:stream_chunk)"| Frontend
```

---

## 5. Data Persistence & SQLite WAL Schema

Primary relational storage is managed by SQLite in WAL mode:

```sql
-- Memory Items (AES-256-GCM encrypted content)
CREATE TABLE memory_items (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    category TEXT NOT NULL,
    content_markdown TEXT NOT NULL,  -- ENC::<nonce>::<ciphertext>
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Full-Text Search Virtual Table (FTS5)
CREATE VIRTUAL TABLE memory_fts USING fts5(
    title,
    content,
    content='memory_items',
    content_rowid='rowid'
);

-- Vector Embeddings for Cosine Similarity
CREATE TABLE memory_embeddings (
    memory_id TEXT PRIMARY KEY,
    embedding_json TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

-- Encrypted Chat History
CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    role TEXT NOT NULL,
    content TEXT NOT NULL,           -- ENC::<nonce>::<ciphertext>
    metadata_json TEXT,
    created_at TEXT NOT NULL
);

-- Continuous Fine-Tuning Dataset
CREATE TABLE training_dataset (
    id TEXT PRIMARY KEY,
    instruction TEXT NOT NULL,
    input_context TEXT NOT NULL,
    output TEXT NOT NULL,
    user_edit TEXT,
    quality_score REAL DEFAULT 1.0,
    created_at TEXT NOT NULL
);

-- Encrypted OAuth Connector Tokens
CREATE TABLE connector_tokens (
    connector_id TEXT PRIMARY KEY,
    encrypted_token TEXT NOT NULL,
    nonce TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Semantic Cache Entries (Encrypted)
CREATE TABLE semantic_cache (
    id TEXT PRIMARY KEY,
    query_hash TEXT NOT NULL,
    embedding_json TEXT NOT NULL,
    response_content TEXT NOT NULL,  -- ENC::<nonce>::<ciphertext>
    created_at TEXT NOT NULL
);
```

---

## 6. IPC Interface Specification

### 6.1 Core Tauri Commands

| Command Signature | Subsystem | Return Type | Description |
|---|---|---|---|
| `agent_chat(prompt, session_id)` | agent | `Result<ChatMessageResponse, String>` | Executes multi-turn ReAct loop with `agent:step` streaming |
| `get_hardware_profile()` | models | `Result<HardwareProfile, String>` | Returns system RAM tier, CPU cores, and model profile |
| `list_hf_gguf_files(repo_id)` | models | `Result<Vec<HfGgufFile>, String>` | Queries HuggingFace Hub API and calculates RAM compatibility |
| `download_hf_gguf(repo_id, file)` | models | `Result<GgufRegistryEntry, String>` | Streams chunked download with progress reporting |
| `get_dataset_stats()` | models | `Result<DatasetStats, String>` | Returns count of logged samples and PII mask stats |
| `export_training_dataset(fmt, path)` | models | `Result<usize, String>` | Exports dataset into ShareGPT or DPO JSONL format |
| `clear_training_dataset()` | models | `Result<usize, String>` | Purges logged training samples |
| `start_wake_listener()` | voice | `Result<(), String>` | Starts WASAPI ring buffer and wake word detector |
| `interrupt_speech()` | voice | `Result<VoiceInterruptResponse, String>` | Kills active speech output process immediately |
| `mcp_list_tools()` | mcp | `Result<Vec<McpToolItem>, String>` | Aggregates tools across all connected MCP servers |

### 6.2 Real-Time Tauri Event Channels

| Channel | Payload Type | Trigger Frequency | Purpose |
|---|---|---|---|
| `agent:step` | `AgentStepPayload` | Per ReAct step | Streams thought, action, observation, and duration to chat |
| `llm:stream_chunk` | `String` | Per token chunk | Streams generation text during final answer synthesis |
| `voice:wake` | `WakeEventData` | On "Deyana" match | Shifts floating HUD state to `LISTENING` |
| `voice:speaking_done` | `VoiceSpeakingPayload`| On TTS completion | Returns HUD state to `IDLE` |
| `proactive:context_card` | `ProactiveCard` | On context shift | Displays unobtrusive active window recommendations |

---

## 7. Testing Architecture & Verification Integrity

Deyana adheres to a decoupled test organization:

```text
apps/desktop/src-tauri/
  src/
    agent/
      react.rs
      tests/
        react_tests.rs           <-- Domain-co-located unit tests
    models/
      tests/
        downloader_tests.rs      <-- Dedicated unit tests
  tests/
    react_loop_integration_tests.rs  <-- Root integration test suite
    empirical_m1_challenger_tests.rs <-- Adversarial boundary tests
```

### Verification Status: 100% Green
- **Unit Tests:** 101 passed; 0 failed (`cargo test --bin deyana-desktop`).
- **Integration Tests:** 7 passed; 0 failed (`cargo test --test react_loop_integration_tests`).
- **Challenger Tests:** 4 passed; 0 failed (`cargo test --test empirical_m1_challenger_tests`).
- **Frontend Build:** 0 type errors; production bundle built cleanly via Vite (`pnpm run build`).
