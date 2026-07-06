import type { BrowserContextMode, BrowserPageContext } from "@deyana/schemas";

export interface BrowserSiteAdapter {
  readonly id: string;
  readonly version: number;
  matches(url: URL): boolean;
  extract(mode: BrowserContextMode): BrowserPageContext;
}
