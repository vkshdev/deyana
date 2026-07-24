import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const messengerAdapter = createDraftFirstAdapter({
  id: "messenger",
  origin: "https://www.messenger.com",
  allowedOrigins: ["https://www.messenger.com", "https://messenger.com", "https://www.facebook.com", "https://facebook.com"],
  allowedHostnames: ["www.messenger.com", "messenger.com", "www.facebook.com", "facebook.com"],
  pathPrefixes: ["/", "/messages", "/t/"],
  titlePrefix: "Messenger",
  readLabel: "Read visible Meta Messenger chat thread",
  draftLabel: "Draft reply for visible Messenger thread",
  fieldLabel: "Insert reviewed Messenger draft",
  titleSelectors: [
    "[role='main'] h1",
    "[role='main'] [role='heading']",
    "h2",
    "[data-testid='conversation_header']"
  ],
  selectors: [
    "[role='main'] [role='row']",
    "[role='main'] [data-scope='messages_table']",
    "[role='main'] [role='article']",
    "[role='main'] [dir='auto']",
    "[role='textbox'][contenteditable='true']"
  ]
});
