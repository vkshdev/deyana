<p align="center">
  <img src="assets/brand/deyana-readme-banner.png" alt="DEYANA - AI that remembers what matters" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-111827?style=flat-square"></a>
  <img alt="Local first" src="https://img.shields.io/badge/local--first-private-2563eb?style=flat-square">
  <img alt="Desktop" src="https://img.shields.io/badge/desktop-Tauri%20%2B%20React-0f766e?style=flat-square">
  <img alt="AI runtime" src="https://img.shields.io/badge/AI-Ollama%20local-7c3aed?style=flat-square">
</p>

Deyana (pronounced **De-Yana**) is a privacy-first desktop assistant that runs on your computer, keeps memory on your machine, uses local AI models by default, and routes every external request through an explicit privacy boundary.

It is built for people who want an assistant over their real personal or work context without sending private files, chat history, source code, connector data, voice, memory, embeddings, or summaries to cloud AI APIs.

## Why Deyana Exists

Most assistants are either cloud chat windows, browser dashboards, or powerful agents that need broad access to your tools. Deyana takes a narrower and safer bet:

- your memory stays local in SQLite and a user-editable Markdown vault
- private context is processed by local models through Ollama
- connector data is read only after user approval
- external requests pass through a privacy firewall
- destructive actions and external writes require explicit confirmation
- the main product surface is a floating desktop UI, not another browser tab

The goal is not to build the loudest agent. The goal is to build the assistant you can trust with internal data.

## Free And Open Source

Deyana is free software for the open-source community. It has no subscription, paid tier, license fee, advertising, checkout, in-app payment processing, or payment-service integration.

## Features

- Floating transparent desktop assistant for fast access while you work.
- Local onboarding, model profile selection, and vault setup.
- SQLite-backed structured memory mirrored into Markdown notes.
- Local chat with memory retrieval, compressed context, and source references.
- Ollama model detection, model selection, and low-spec defaults.
- Privacy firewall for external requests and local audit logs.
- Read-only connector architecture for approved services.
- Permission-gated tools for web, files, git, coding, and planning.
- Local-only push-to-talk and TTS with a female-only Deyana voice and selectable installed female Windows voices.
- Release-quality panels for logs, privacy export, deletion, connector health, performance, and crash recovery.

## Privacy Model

Deyana has one non-negotiable rule:

> Sensitive user data must not be sent to cloud AI APIs.

Sensitive data includes local files, source code, personal notes, vault content, voice transcripts, chat history, memory, summaries, embeddings, compressed context, Gmail content, Calendar events, Drive files, Slack messages, Notion pages, GitHub private repository content, and Jira or Linear issues.

Allowed external traffic is limited to user-approved connector API calls, public web requests, OAuth token refresh, and optional update checks. AI processing over private data stays local.

## Install On Windows

**Windows installer coming soon.**

The first signed DEYANA installer will be published through GitHub Releases. Source setup and development instructions are available in [CONTRIBUTING.md](CONTRIBUTING.md).

## Architecture

Read the full design in [ARCHITECTURE.md](ARCHITECTURE.md).

## Comparison

This comparison is positioning, not a claim that Deyana replaces every other open-source assistant. These projects are strong in different categories. Deyana focuses on local desktop privacy for personal and internal data.

| Project | Primary shape | Internal data and offline posture | Where Deyana is different |
| --- | --- | --- | --- |
| [Open Interpreter](https://github.com/openinterpreter/openinterpreter) | Coding agent and terminal workflow | Strong for code execution and model/provider flexibility, with local session state | Deyana is a desktop personal assistant with memory, connectors, privacy firewall, and local-first UX beyond coding |
| [Khoj](https://github.com/khoj-ai/khoj) | Self-hostable second brain and research assistant | Works with docs, web, agents, automations, and local or online LLMs | Deyana makes private data local-only by product rule and optimizes for Windows desktop presence |
| [AnythingLLM](https://github.com/Mintplex-Labs/anything-llm) | Local-first RAG and agent workspace | Strong document/workspace ownership story | Deyana adds an OS-level floating assistant, permissioned tools, local audit logs, and strict external request routing |
| [Open WebUI](https://github.com/open-webui/open-webui) | Web interface for interacting with models | Excellent model UI, including Ollama and hosted providers | Deyana is not just a model UI; it owns memory, connectors, tool permissions, and desktop lifecycle |
| [Dify](https://github.com/langgenius/dify) | Agentic workflow development platform | Production workflow platform, commonly deployed as a server product | Deyana is a single-user local desktop assistant for personal/internal context, not a hosted workflow platform |
| [Hermes Agent](https://github.com/NousResearch/hermes-agent) | Self-improving multi-channel agent | Strong learning loop, skills, gateways, and flexible cloud/VPS/serverless operation | Deyana deliberately prioritizes local laptop privacy, low-spec defaults, and user-visible permission boundaries |
| [OpenClaw](https://github.com/openclaw/openclaw) | Personal AI assistant with broad channel support | Local-first gateway with many messaging channels and powerful tools | Deyana has a smaller surface area by design: desktop-first, private-memory-first, and firewall-first |
| [OpenHands](https://github.com/OpenHands/OpenHands) | Developer control center for coding agents and automations | Can run locally or with remote/cloud backends; sandboxing matters for filesystem safety | Deyana targets private everyday desktop memory and connector data, with no cloud AI path for private context |

Autonomous agents are powerful because they can touch real tools and internal data. That is also the risk. Deyana treats privacy, permissions, and local auditability as core product features instead of add-ons.

## Repository Layout

```text
de-yana/
  apps/desktop/          Tauri, React, TypeScript desktop app
  packages/config/       Shared product defaults
  packages/schemas/      Shared TypeScript contracts
  services/core/         Python FastAPI core service
  vault-template/        Default local Markdown vault structure
  assets/brand/          Public README logo and banner assets
```

## Contributing

Setup, development commands, testing, pull-request guidance, and private security reporting are documented in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
