import type { BrowserSiteAdapter } from "./BrowserSiteAdapter";
import { gmailAdapter } from "./gmailAdapter";
import { genericPageAdapter } from "./genericPageAdapter";
import { githubAdapter } from "./githubAdapter";
import { linkedinAdapter } from "./linkedinAdapter";
import { slackAdapter } from "./slackAdapter";
import { whatsappWebAdapter } from "./whatsappWebAdapter";

const adapters: BrowserSiteAdapter[] = [
  whatsappWebAdapter,
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
