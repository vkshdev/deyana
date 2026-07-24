import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const gmailAdapter = createDraftFirstAdapter({
  id: "gmail",
  origin: "https://mail.google.com",
  allowedOrigins: ["https://mail.google.com"],
  titlePrefix: "Gmail",
  readLabel: "Read visible Gmail thread",
  draftLabel: "Draft reply for visible Gmail thread",
  fieldLabel: "Insert reviewed Gmail draft",
  titleSelectors: ["h2[data-thread-perm-id]", "h2", "[role='main'] h1", "[role='main'] [aria-label*='Subject']"],
  selectors: [
    "[role='main'] h2",
    "[role='main'] [data-message-id]",
    "[role='main'] .a3s",
    "[role='main'] [aria-label*='Message Body']",
    "[role='main'] [role='document']",
    "[role='textbox'][aria-label*='Message Body']"
  ]
});
