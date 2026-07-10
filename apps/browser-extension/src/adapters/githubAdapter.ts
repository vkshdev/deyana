import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const githubAdapter = createDraftFirstAdapter({
  id: "github",
  origin: "https://github.com",
  titlePrefix: "GitHub",
  readLabel: "Read visible GitHub issue or pull request",
  draftLabel: "Draft GitHub comment",
  fieldLabel: "Insert reviewed GitHub comment draft",
  titleSelectors: [".js-issue-title", "bdi.js-issue-title", "h1 bdi", "h1"],
  selectors: [
    ".js-discussion",
    ".timeline-comment",
    ".comment-body",
    ".markdown-body",
    "[data-testid='issue-body']",
    "main"
  ]
});
