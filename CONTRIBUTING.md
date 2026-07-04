# Contributing to Deyana

We're so glad you're interested in contributing to Deyana! This project is all about building something meaningful for real users, with a strong focus on privacy and data security. We really value contributions that prioritize privacy, safety, and long-term maintainability over just getting things done quickly.



## Setup

Prerequisites:

- Windows 10 or 11
- Git
- Node.js 22+
- pnpm 11+
- Rust with the MSVC toolchain
- Python 3.12 or 3.13
- Ollama

Install dependencies:

```powershell
pnpm install
pnpm core:install
```

Run the desktop app:

```powershell
pnpm desktop:dev
```

Run the backend only:

```powershell
pnpm core:dev
```

## Engineering Standards

To keep our codebase healthy and easy to work with, here are some guidelines we try to follow:

- It's a good idea to keep business logic separate from your UI components.
- Most of our core behavior lives in `services/core`.
- You'll find shared contracts in `packages/schemas`.
- We generally prefer small, explicit changes that are easy to review over large, broad rewrites.
- Try to avoid adding new dependencies if the standard library or existing tools can do the job.
- When you're coding, please keep low-spec Windows laptops in mind for performance.
- And finally, let's all try to avoid leaving placeholder code, fake integrations, or TODO stubs in committed code.

## Privacy Rules

Privacy is at the core of Deyana, and it's super important to us. When contributing, please keep these critical privacy principles in mind:

We can't accept changes that would send private data to:

- cloud AI APIs
- hosted embedding APIs
- hosted rerankers
- cloud STT
- cloud TTS
- any unapproved external services

Remember, every external request needs to pass through our privacy firewall. Any destructive actions or external writes should always require explicit user confirmation.

Deyana is committed to being permanently free and open source. With that in mind, we kindly ask that contributions do not introduce subscriptions, paid tiers, advertisements, checkout processes, in-app purchases, license enforcement, donation gating, or any form of payment collection.

Just to be clear, "private data" includes things like connector payloads, local files, source code, notes, vault content, chat history, voice transcripts, memory, summaries, embeddings, and compressed context.

## Security Contributions

We truly appreciate any security contributions!. When it comes to security fixes, our active public development line is `main`, so please target `main` first.

We kindly ask that you please refrain from publishing exploit details or sensitive evidence in a public issue. Instead, if GitHub private vulnerability reporting is enabled, please use that. Otherwise, you can open a minimal public issue stating that a private security report is available and request a safe contact channel.

For us to best understand and address a private report, a useful one generally includes:

- the affected version or commit
- the operating system
- minimal reproduction steps
- expected impact and severity
- whether private data can leave the device
- whether tokens, files, memory, voice, tools, or connector data are involved
- a suggested mitigation when known

We consider these areas to be particularly high-risk, so please approach them with extra care:

- connector OAuth and encrypted token storage
- privacy firewall allow/block decisions
- permissioned tool execution
- local file and source-code access
- memory export and deletion
- voice transcription and speech
- logs and privacy audit output
- update, packaging, and installer flows

It's crucial that you never commit `.env` files, OAuth tokens, API keys, local databases, vault data, runtime logs, installers, model files, voice recordings, or transcripts. If you accidentally commit a secret, please rotate it immediately and remove it from Git history before publishing.

## Test Commands

Before you submit your contribution, here are some commands to help you test your changes and ensure everything is working as expected:

To run the full release gate:

```powershell
pnpm release:check
```

Or, if you prefer to run individual checks:

```powershell
pnpm check
pnpm core:test
pnpm --filter @deyana/desktop build
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

For privacy-sensitive changes, please consider adding targeted tests for allow/block behavior, audit logs, token redaction, permission prompts, and deletion safety. These are super important!

## Pull Requests

When you're ready to open a pull request, here's what makes a great one and helps us review it quickly:

-   A clear problem statement: Tell us what issue you're addressing.
-   Focused implementation: Keep your changes concise and to the point.
-   Good test coverage (or a clear reason why tests weren't added).
-   Screenshots for any visible UI changes are super helpful!
-   Privacy impact notes, especially when you're working with connectors, tools, memory, voice, or model routing.

Please remember not to include any secrets, real tokens, private logs, personal connector data, local vault contents, generated model files, installers, or runtime databases directly in your pull request. It's a security best practice!
