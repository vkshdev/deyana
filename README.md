<p align="center">
  <img src="assets/brand/deyana-readme-banner.png" alt="DEYANA - AI that remembers what matters" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-111827?style=flat-square"></a>
  <img alt="Status: Building Phase" src="https://img.shields.io/badge/status-in%20active%20development-amber?style=flat-square">
  <img alt="Backend" src="https://img.shields.io/badge/backend-native%20Rust-b7410e?style=flat-square">
  <img alt="AI runtime" src="https://img.shields.io/badge/AI-Ollama/Other%20%2B%20llama.cpp-7c3aed?style=flat-square">
</p>

# Deyana (Active Development)

> **Project Status:** De-yana is currently in an active **building phase**. Features, APIs, and models are continuously evolving as we refine the core cognitive architecture.

**Deyana** is an AI assistant. Built with a native **Rust (Tauri 2)** backend and a calm, transparent **React** HUD, Deyana thinks alongside you without sending your personal notes, code, chats, or audio to the cloud.

---

## Key Highlights

- **100% Local & Privacy-Guaranteed** — All memory, chat logs, notes, and embeddings stay encrypted on your device (AES-256-GCM with Windows DPAPI).
- **Memory Graph Intelligence** — Stores relational links between facts and notes (`depends_on`, `related_to`, `same_entity`). When you ask a question, Deyana walks the graph to connect related context instead of treating memories as isolated fragments.
- **Cognitive Architecture ("The Mind Layer")** — Equipped with a persistent self-model, an internal emotional state machine (Russell Circumplex), metacognitive self-reflection, and an idle background thought loop.
- **Autonomous ReAct Agent Loop** — Runs multi-turn tool calling directly inside Rust with GBNF grammar constraints for guaranteed JSON parsing.
- **Model Context Protocol (MCP)** — Native support as both an **MCP Client** (connecting external tool servers) and an **MCP Server** (sharing memory with other agents).
- **Hands-Free Voice** — Push-to-talk and always-on wake word detection powered by `rustpotter` and local speech synthesis.

---

## Documentation Links

As the project is actively developing, detailed technical blueprints and specifications are maintained in the following documents:

- **[ARCHITECTURE.md](ARCHITECTURE.md)** — Complete runtime architecture, process topology, IPC commands, and security model.
-  **[CONTRIBUTING.md](CONTRIBUTING.md)** — Developer prerequisites, building from source, and running test suites.
-  **[LICENSE](LICENSE)** — MIT License terms.

---
