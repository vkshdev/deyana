import type { BrowserSiteAdapter } from "./BrowserSiteAdapter";
import { discordAdapter } from "./discordAdapter";
import { genericPageAdapter } from "./genericPageAdapter";
import { githubAdapter } from "./githubAdapter";
import { gmailAdapter } from "./gmailAdapter";
import { instagramAdapter } from "./instagramAdapter";
import { linkedinAdapter } from "./linkedinAdapter";
import { messengerAdapter } from "./messengerAdapter";
import { slackAdapter } from "./slackAdapter";
import { telegramAdapter } from "./telegramAdapter";
import { whatsappWebAdapter } from "./whatsappWebAdapter";

const adapters: BrowserSiteAdapter[] = [
  whatsappWebAdapter,
  messengerAdapter,
  instagramAdapter,
  discordAdapter,
  telegramAdapter,
  gmailAdapter,
  slackAdapter,
  githubAdapter,
  linkedinAdapter,
  genericPageAdapter
];

export function adapterForCurrentPage(): BrowserSiteAdapter {
  const url = new URL(window.location.href);
  return adapters.find((adapter) => adapter.matches(url)) ?? genericPageAdapter;
}
