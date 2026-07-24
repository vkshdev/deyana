import type {
  BrowserClearFieldRequest,
  BrowserClickActionRequest,
  BrowserClickActionResponse,
  BrowserContextMode,
  BrowserFillFieldRequest,
  BrowserFillFieldResponse,
  BrowserPageContext,
  BrowserWritableField
} from "@deyana/schemas";

export interface BrowserSiteAdapter {
  readonly id: string;
  readonly version: number;
  readonly allowedOrigins?: readonly string[];
  readonly allowedHostnames?: readonly string[];
  readonly pathPrefixes?: readonly string[];
  matches(url: URL): boolean;
  extract(mode: BrowserContextMode): BrowserPageContext;
  listWritableFields(): BrowserWritableField[];
  fillField(request: BrowserFillFieldRequest): BrowserFillFieldResponse;
  clearField(request: BrowserClearFieldRequest): BrowserFillFieldResponse;
  clickAction(request: BrowserClickActionRequest): Promise<BrowserClickActionResponse>;
}
