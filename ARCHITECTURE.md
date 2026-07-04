# DEYANA Architecture

Deyana is a free, open-source, local-first private desktop AI assistant. It is designed for personal and internal data, so the architecture prioritizes privacy boundaries, explicit permissions, local storage, low-spec Windows hardware, and maintainable process separation. The product has no subscription, paid tier, advertising, checkout, license enforcement, or payment collection path.

## Core Principles

- Private data is processed locally.
- No cloud AI, hosted embedding, hosted reranker, cloud STT, or cloud TTS path is allowed for private context.
- External connector APIs are allowed only for user-approved data retrieval.
- Every external request passes through the privacy firewall.
- Destructive actions and external writes require explicit user confirmation.
- Business logic belongs in the Python core, not the desktop UI.
- The UI should remain calm, lightweight, and useful on low-spec Windows laptops.

## Runtime Overview

```text
+---------+       +--------------------+       +---------------------+
|  Owner  | <---> | React Desktop UI   | <---> | Tauri Rust Runtime  |
+---------+       | floating assistant |       | window, tray, core  |
                  +--------------------+       +----------+----------+
                                                        |
                                             local HTTP + WebSocket
                                                        |
                                             +----------v----------+
                                             | Python FastAPI Core |
                                             | product decisions   |
                                             +----------+----------+
                                                        |
              +-------------------+---------------------+-------------------+
              |                   |                     |                   |
      +-------v--------+  +-------v--------+   +--------v-------+  +--------v-------+
      | Local Memory   |  | Local Ollama   |   | Privacy        |  | Permissioned  |
      | SQLite + vault |  | chat + embeds  |   | Firewall       |  | Tools + Voice |
      +----------------+  +----------------+   +--------+-------+  +----------------+
                                                        |
                                               approved requests only
                                                        |
                                             +----------v----------+
                                             | External APIs / Web |
                                             +---------------------+

All decisions, failures, privacy checks, and tool outcomes are written to local audit logs.
```

## Workflow Block Diagram

The user works from the floating desktop UI, Tauri owns native lifecycle, and the Python core owns product decisions. Private data remains inside the local boundary unless the firewall approves a narrowly scoped external request.

```text
USER COMMAND
     |
     v
+--------------------+
| Floating Desktop UI|
+---------+----------+
          | typed request / local event
          v
+--------------------+
| Tauri Rust Runtime |
| window + lifecycle |
+---------+----------+
          | authenticated localhost transport
          v
+--------------------+
| Python FastAPI Core|
| intent + policy    |
+---------+----------+
          |
          +------------------------+--------------------------+
          |                        |                          |
          v                        v                          v
+-------------------+    +-------------------+      +-------------------+
| Local Memory      |    | Local Ollama      |      | Privacy Firewall  |
| retrieve + store  |    | reason + generate |      | classify request  |
+---------+---------+    +---------+---------+      +---------+---------+
          |                        |                          |
          +-------------+----------+                    +-----+------+
                        |                               |            |
                        v                            BLOCK         ALLOW
              +-------------------+                    |            |
              | Local response    |                    v            v
              | with references   |              +----------+  +----------------+
              +---------+---------+              | Audit log|  | Approved API / |
                        |                        +----------+  | public web      |
                        |                                      +-------+--------+
                        |                                              |
                        +--------------------+-------------------------+
                                             |
                                             v
                                   +-------------------+
                                   | UI result / status|
                                   +-------------------+
```

## Layers

### Desktop UI

The desktop app is built with Tauri, React, TypeScript, TailwindCSS, and Framer Motion. It renders the product surface:

- onboarding
- floating assistant
- local chat
- model setup
- memory browser
- connector status
- privacy audit
- voice controls
- release-quality panels

The UI is not the source of business truth. It calls the core service, subscribes to WebSocket events, and renders state from shared schemas.

### Rust Runtime

The Tauri runtime owns native desktop responsibilities:

- transparent floating window
- tray lifecycle
- monitor-aware placement
- native settings bridge
- secure file dialogs
- Python core child process startup and restart
- app data directory management

The Rust layer should stay small. It should not duplicate memory, connector, privacy, model, or tool logic.

### Python Core

The Python core is the product brain. It uses FastAPI, Pydantic, WebSocket events, SQLite, and local filesystem storage.

It owns:

- settings and onboarding state
- vault setup
- SQLite memory
- Markdown vault writes
- local memory analysis
- local chat orchestration
- Ollama model routing
- privacy firewall
- connector registration and sync
- encrypted connector token storage
- permissioned tools
- local voice services
- release readiness, logs, export, deletion, performance, and crash recovery

## Data Flow

### Chat With Memory

```text
+-------+     +------------+     +-------------+
| Owner | --> | Desktop UI | --> | Python Core |
+-------+     +------------+     +------+------+ 
                                      |
                         +------------+-------------+
                         |                          |
                         v                          v
                +----------------+         +----------------+
                | SQLite Memory  |         | Recent Chat    |
                | Markdown Vault |         | History        |
                +-------+--------+         +-------+--------+
                        |                          |
                        +------------+-------------+
                                     |
                                     v
                           +--------------------+
                           | Context Builder    |
                           | rank + compress    |
                           +---------+----------+
                                     |
                                     v
                           +--------------------+
                           | Local Ollama Model |
                           +---------+----------+
                                     |
                          answer + source references
                                     |
                        +------------+-------------+
                        |                          |
                        v                          v
                +---------------+          +---------------+
                | Store locally |          | Render in UI  |
                +---------------+          +---------------+
```

Private memory never leaves the machine during this flow.

### Connector Sync

```text
+-------+     +------------+     +-------------+     +------------------+
| Owner | --> | Desktop UI | --> | Python Core | --> | Privacy Firewall |
+-------+     +------------+     +-------------+     +---------+--------+
                                                            |
                                                 +----------+----------+
                                                 |                     |
                                               BLOCK                 ALLOW
                                                 |                     |
                                                 v                     v
                                         +---------------+    +-------------------+
                                         | Local audit   |    | Connector API     |
                                         | + UI reason   |    | approved read     |
                                         +---------------+    +---------+---------+
                                                                      |
                                                                      v
                                                            +-------------------+
                                                            | Normalize locally |
                                                            | summarize + index |
                                                            +---------+---------+
                                                                      |
                                                                      v
                                                            +-------------------+
                                                            | SQLite + Markdown |
                                                            | local memory      |
                                                            +---------+---------+
                                                                      |
                                                                      v
                                                            +-------------------+
                                                            | UI status event   |
                                                            +-------------------+
```

Connector access is read-oriented by default. External writes require separate confirmation.

### Permissioned Tool Use

Tools are divided by risk:

- low-risk local inspection
- public web access
- local file access
- source-code access
- dangerous or destructive action

The core must request explicit confirmation before risky actions. Tool results that could contain private data must not be forwarded to cloud AI.

## Storage

Deyana uses local storage only:

- SQLite for structured memory, chat history, connector metadata, and audit records
- Markdown vault for user-editable memory
- OS-backed encrypted token storage where available
- local logs for diagnostics and privacy events
- local settings for model profile, vault path, sync mode, and voice preferences

Runtime user data, databases, logs, vault contents, model files, recordings, and installer artifacts are ignored by Git.

## Local AI Strategy

The default model strategy is hardware-aware. Deyana should recommend local models from the owner's actual desktop or laptop capacity instead of assuming one model fits every machine.

The model setup flow should use locally available signals:

- installed RAM
- CPU class and logical core count
- integrated GPU or discrete GPU presence
- available VRAM when the OS exposes it
- available disk space for model downloads
- battery or low-power preference
- currently installed Ollama models

This hardware profile is used only for local recommendation and should not be uploaded. The user can override the recommendation, but Deyana should always prefer the safest model that can run reliably on the current machine.

```text
+------------------+       +--------------------+
| Local Hardware   |       | Ollama Inventory   |
| RAM, CPU, GPU,   |       | installed models   |
| disk, power mode |       | and availability   |
+--------+---------+       +----------+---------+
         |                            |
         +-------------+--------------+
                       |
                       v
             +--------------------+
             | Hardware Profiler  |
             | local signals only |
             +---------+----------+
                       |
                       v
             +--------------------+
             | Recommend Profile  |
             | low / balanced /   |
             | power              |
             +---------+----------+
                       |
                       v
             +--------------------+
             | Test Model Locally |
             | latency + memory   |
             +---------+----------+
                       |
                       v
             +--------------------+
             | Owner Confirms     |
             | persist selection  |
             +--------------------+
```

Recommended profile rules:

| Owner machine profile | Typical specs | Default recommendation | Runtime posture |
| --- | --- | --- | --- |
| Low-spec | About 8 GB RAM, i3/Ryzen 3 class CPU, 2-4 logical cores, integrated graphics | `qwen3:1.7b` for chat and summaries, `all-minilm:latest` for embeddings | One model job at a time, aggressive context compression, voice disabled by default |
| Balanced | About 16 GB RAM, modern 4-8 core CPU, optional integrated or entry GPU | 3B to 4B quantized local chat model when installed, small local embedding model | Normal context window, cautious background sync, optional voice |
| Power | 32 GB+ RAM, strong CPU, discrete GPU or high VRAM | 7B to 14B quantized local model only when latency is acceptable | Longer context, richer retrieval, optional local reranking later |

The first supported profile targets low-spec Windows laptops with about 8 GB RAM, integrated graphics, and modest CPU performance.

Default low-spec posture:

- chat and summarization: `qwen3:1.7b`
- embeddings: `all-minilm:latest`
- concurrent model jobs: one
- voice: disabled by default
- sync: manual or conservative
- context: compressed before generation

Larger 7B or 8B models should not become the default. They can be supported as user-selected balanced or power profiles later.

## Local Voice

Deyana uses only installed Windows speech APIs for voice input and output. Voice remains disabled and muted by default on low-spec systems. When the user enables TTS, the core selects Microsoft Zira when available or another installed female voice. Non-female voices are filtered out and rejected; if no female voice is installed, TTS remains unavailable instead of falling back to the Windows system default. The desktop voice selector exposes only installed female alternatives and persists the owner's choice.

Voice names, gender metadata, language, and the active selection stay on the device. No raw audio is stored, and no cloud STT or TTS provider is allowed.

## Privacy Firewall

Every external request must pass through the firewall before it leaves the device.

The firewall classifies:

- destination category
- request purpose
- data category
- connector authorization
- user consent state
- privacy mode

It blocks:

- private data sent to cloud AI APIs
- private data sent to hosted embedding or reranking APIs
- local voice sent to cloud STT
- local text sent to cloud TTS
- private summaries or compressed context sent externally
- unapproved local files, source code, vault content, or connector data sent externally

Blocked and allowed decisions are written to a local audit trail.

## Failure Modes

- Ollama missing: model status reports offline or missing, and the UI guides local setup.
- Selected model missing: the core returns a recoverable response instead of falling back to cloud AI.
- Backend crash: Tauri shows recovery state and can restart the Python core.
- Connector failure: sync run state is recorded locally and surfaced in the UI.
- Privacy block: the request is denied, logged, and paired with a safe local alternative when possible.
- Deletion request: local data deletion requires the exact confirmation phrase.

## Public Interfaces

The core exposes local HTTP and WebSocket interfaces for the desktop shell:

- health and status
- settings and onboarding
- vault selection
- memory CRUD, search, summaries, export, and deletion
- model status, selection, and test prompts
- chat messages and history
- privacy checks and audit events
- connector setup, settings, sync, and health
- permissioned tools
- voice settings, installed voice discovery and selection, transcription, and speech
- release readiness, logs, privacy export, deletion, performance, and crash recovery

The shared TypeScript schemas in `packages/schemas` should stay aligned with Python response models.
