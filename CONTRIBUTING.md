# Contributing to Deyana

Thank you for contributing to Deyana — an open-source, privacy-first desktop AI assistant.

Deyana is a monorepo containing a native Rust runtime (`apps/desktop/src-tauri`) and a React 18 frontend (`apps/desktop/src`). All business logic, AI routing, memory management, and privacy enforcement reside strictly in the Rust backend. Full architectural specifications and IPC contracts are documented in [ARCHITECTURE.md](ARCHITECTURE.md).

## Prerequisites & Development Commands

| Requirement / Action | Command / Tool | Working Directory | Notes |
|---|---|---|---|
| **Node.js & pnpm** | Node 20+, pnpm 9+ | Root | Monorepo package manager |
| **Rust Toolchain** | Stable `rustup` | Root | Desktop backend compiler |
| **Local LLM Engine** | Ollama on port `11434` | System | Pull dev models: `qwen2.5:1.5b`, `all-minilm` |
| **Install Dependencies** | `pnpm install` | Root | Run once after cloning |
| **Start Dev App** | `pnpm --filter @deyana/desktop dev` | Root | Launches Vite dev server and debug Rust binary |
| **Rust Compile Check** | `cargo check` | `apps/desktop/src-tauri` | Fast compilation check |
| **Rust Test Suite** | `cargo test` | `apps/desktop/src-tauri` | Unit and integration tests |
| **Frontend Type Check** | `npm --prefix apps/desktop run build` | Root | Validates TypeScript and production bundle |
| **Release Build** | `cargo build --release` | `apps/desktop/src-tauri` | Production binary compilation |

## Architectural Rules

1. **Rust Core Ownership:** All state, business logic, file I/O, and networking belong in Rust (`apps/desktop/src-tauri/src/`). The React UI is strictly a presentation shell.
2. **Pure Tauri IPC:** Inter-process communication must use Tauri `invoke` for commands and `listen` for event streaming. No internal HTTP servers or loopback web ports.
3. **Local Privacy Boundary:** Private context (files, chat logs, code, vault notes, active window titles) must never leave the machine.
4. **Firewall Gatekeeping:** Outbound network calls (connector sync, public web search) must pass through `privacy::commands::evaluate_privacy_policy` before execution.
5. **Explicit User Consent:** Destructive actions, disk writes, or external posts require user confirmation via explicit action plans.

## Quality Assurance

Before submitting a Pull Request, verify that all backend and frontend checks pass cleanly:

```bash
cd apps/desktop/src-tauri && cargo check && cargo test
cd ../.. && npm --prefix apps/desktop run build
```

## Security Reporting

To report security or privacy vulnerabilities (e.g. data leaking past the privacy firewall or unverified disk access), notify the project maintainers privately via GitHub Security Advisories rather than opening a public issue.

## License

Under the [MIT License](LICENSE). By contributing, you agree that your contributions will be licensed under the same terms.
