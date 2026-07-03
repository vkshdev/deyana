# DEYANA Vault Structure

Deyana creates this user-owned Markdown vault during onboarding. It stores compressed local summaries and source references rather than raw private dumps by default.

```text
Daily/
Projects/
People/
Meetings/
Emails/
GitHub/
Slack/
Tasks/
Decisions/
Sources/
Inbox/
```

The runtime source of truth is `VAULT_FOLDERS` in `services/core/src/deyana_core/storage.py`; the folders are created automatically and do not require placeholder files in the repository.
