import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const telegramAdapter = createDraftFirstAdapter({
  id: "telegram",
  origin: "https://web.telegram.org",
  allowedOrigins: ["https://web.telegram.org"],
  allowedHostnames: ["web.telegram.org"],
  pathPrefixes: ["/a/", "/k/", "/"],
  titlePrefix: "Telegram Web",
  readLabel: "Read visible Telegram Web chat thread",
  draftLabel: "Draft reply for Telegram Web thread",
  fieldLabel: "Insert reviewed Telegram draft",
  titleSelectors: [
    ".chat-info .peer-title",
    ".top-bar .person-name",
    ".chat-title",
    "h3"
  ],
  selectors: [
    ".message-list .message",
    ".messages-container .message",
    ".message-content",
    ".text-content",
    "div[contenteditable='true']"
  ]
});
