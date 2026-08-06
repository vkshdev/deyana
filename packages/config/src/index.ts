export const productIdentity = {
  name: "Deyana",
  brand: "DEYANA",
  pronunciation: "De-Yana"
} as const;

export const desktopWindow = {
  compact: {
    width: 92,
    height: 144
  },
  expanded: {
    width: 408,
    height: 652
  },
  defaultTopOffset: 108,
  defaultRightOffset: 24
} as const;


export const coreService = {
  host: "localhost",
  port: 0,
  endpoint: "tauri://localhost",
  websocketUrl: "tauri://localhost/ws",
  heartbeatGraceMs: 12_000
} as const;

export const modelDefaults = {
  profile: "low_spec",
  chatModel: "qwen3:1.7b",
  embeddingModel: "all-minilm:latest",
  maxParallelModelJobs: 1,
  think: false
} as const;

export const designTokens = {
  color: {
    glass: "rgba(4, 8, 14, 0.82)",
    glassStrong: "rgba(7, 13, 23, 0.94)",
    border: "rgba(164, 184, 210, 0.2)",
    text: "#f3f1ec",
    textMuted: "#a8b2c0",
    blue: "#83a9d4",
    gold: "#d8b978",
    goldSoft: "#f1d9a7",
    danger: "#e78494"
  },
  radius: {
    panel: 8,
    control: 8,
    round: 999
  },
  shadow: {
    panel: "0 24px 74px rgba(0, 0, 0, 0.5)",
    glow: "0 0 26px rgba(74, 108, 151, 0.16)"
  }
} as const;
