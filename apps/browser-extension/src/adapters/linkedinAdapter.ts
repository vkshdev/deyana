import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const linkedinAdapter = createDraftFirstAdapter({
  id: "linkedin",
  origin: "https://www.linkedin.com",
  titlePrefix: "LinkedIn",
  readLabel: "Read visible LinkedIn conversation or post",
  draftLabel: "Draft LinkedIn reply",
  fieldLabel: "Insert reviewed LinkedIn draft",
  titleSelectors: [
    ".msg-entity-lockup__entity-title",
    ".msg-thread__link-to-profile",
    ".feed-shared-actor__name",
    "h1"
  ],
  selectors: [
    ".msg-s-message-list__event",
    ".msg-s-event-listitem",
    ".msg-s-event__content",
    ".feed-shared-update-v2",
    ".comments-comment-item",
    "main"
  ]
});
