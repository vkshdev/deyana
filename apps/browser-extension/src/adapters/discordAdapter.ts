import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const discordAdapter = createDraftFirstAdapter({
  id: "discord",
  origin: "https://discord.com",
  allowedOrigins: ["https://discord.com", "https://ptb.discord.com", "https://canary.discord.com"],
  allowedHostnames: ["discord.com", "ptb.discord.com", "canary.discord.com"],
  pathPrefixes: ["/channels/"],
  titlePrefix: "Discord",
  readLabel: "Read visible Discord channel or DM thread",
  draftLabel: "Draft Discord message or reply",
  fieldLabel: "Insert reviewed Discord draft",
  titleSelectors: [
    "h1",
    "h3[class*='title']",
    "[role='heading'][aria-label]",
    "[data-list-item-id*='channels']"
  ],
  selectors: [
    "[role='listitem'][class*='message']",
    "[class*='messageContent']",
    "[class*='markup']",
    "[role='textbox'][contenteditable='true']"
  ]
});
