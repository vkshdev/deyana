<p align="center">
  <img src="assets/brand/deyana-readme-banner.png" alt="DEYANA - AI that remembers what matters" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-111827?style=flat-square"></a>
  <img alt="Local first" src="https://img.shields.io/badge/local--first-private-2563eb?style=flat-square">
  <img alt="Desktop" src="https://img.shields.io/badge/desktop-Tauri%202%20%2B%20React%2018-0f766e?style=flat-square">
  <img alt="Backend" src="https://img.shields.io/badge/backend-native%20Rust-b7410e?style=flat-square">
  <img alt="AI runtime" src="https://img.shields.io/badge/AI-Ollama%20local-7c3aed?style=flat-square">
  <img alt="MCP" src="https://img.shields.io/badge/MCP-client%20%2B%20server-059669?style=flat-square">
</p>

Deyana is an open-source, privacy-first desktop AI assistant designed to run entirely on local hardware. Built with a native Rust backend and a lightweight React frontend, Deyana keeps all chat history, memory, vector embeddings, and notes strictly on your machine.

Instead of routing private code, documents, or context to cloud APIs, Deyana leverages local Ollama models and an on-device SQLite database paired with a Markdown vault. A built-in privacy firewall enforces request boundaries, while a proactive context sentinel surfaces insights as you work. Deyana supports hands-free voice wake word detection ("Deyana") and includes full Model Context Protocol (MCP) support as both a client and a server.

Technical architecture details are documented in [ARCHITECTURE.md](ARCHITECTURE.md).

## Core Capabilities

- **Native Rust Core Engine** — Zero Python runtime dependencies or sidecar server processes. Built as a single lightweight Tauri 2 binary for fast cold-starts and low memory usage.
- **On-Device Hybrid Memory** — Combines SQLite FTS5 keyword indexing with local vector embeddings (via Ollama) to retrieve accurate context from past chats and personal notes.
- **Proactive Context Sentinel** — Periodically analyzes active window titles and desktop context to surface relevant suggestions and reminders without manual prompting.
- **Hands-Free Voice Pipeline** — Always-on wake word detection ("Deyana") powered by `rustpotter` and `cpal` WASAPI, paired with local speech-to-text and native speech synthesis.
- **Dual-Role MCP Runtime** — Functions as an MCP Client to connect external tool servers, and as an MCP Server (axum SSE) exposing local memory and vault search to external AI environments.
- **Hardened Privacy Firewall** — Intercepts all external network requests. Classifies traffic by destination and data type, blocking private context from leaving the device while maintaining audit logs with SHA-256 signatures.
- **Read-Only Connectors & Safe Tools** — Integrates with services like GitHub, Gmail, Calendar, Slack, Drive, Notion, Jira, and Linear under strict read-only constraints, requiring explicit user approval for external actions.

## How Deyana Compares

Deyana is designed specifically for users who need a persistent, local desktop assistant operating over sensitive personal or workspace data.

| Feature Category | Cloud AI Chatbots & Web Portals | Heavy Python Agent Frameworks | Desktop Widgets & OS Plugins | Deyana Desktop Assistant |
| --- | --- | --- | --- | --- |
| **Execution Runtime** | Remote Cloud Servers | Python Virtual Environments / CLI | Background Webview / OS Process | Native Rust (Tauri 2 + Async Tokio) |
| **Data Boundary** | Cloud Storage & Remote Logs | Depends on script configuration | Cloud Telemetry & Sync | 100% Local (On-Device SQLite + Vault) |
| **AI Model Processing** | Proprietary Hosted APIs | Configurable Cloud or Local APIs | Proprietary Hosted APIs | Local Models via Ollama Engine |
| **Privacy Enforcement** | Terms of Service Policy | Manual Code Checks | Cloud Privacy Settings | Deterministic Rust Firewall + Audit Logs |
| **Desktop Presence** | Browser Tab / Web Window | Terminal / CLI Interface | System Tray / Taskbar Overlay | Floating Transparent HUD + Win32 Sentinel |
| **Extensibility** | Closed Plugin Ecosytems | Custom Python Scripts | Fixed API Integrations | Standard MCP Client & MCP Server |

## Privacy Model

Deyana operates under a strict privacy guarantee:

> **Private user context must never be transmitted to cloud AI services.**

Private context includes local source code, personal notes, vault files, voice recordings, transcripts, chat history, memory items, summaries, vector embeddings, workspace connector data, and active window states.

External network calls are limited strictly to user-authorized API connector fetches, public web requests, OAuth token refreshes, and optional update checks. All outbound traffic is audited locally.

## Installation & Setup

For developer setup, building from source, running tests, and contribution guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

For detailed system architecture, IPC event contracts, and data storage design, see [ARCHITECTURE.md](ARCHITECTURE.md).

## License

Free and open-source software under the [MIT License](LICENSE).
