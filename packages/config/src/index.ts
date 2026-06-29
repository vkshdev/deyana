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
  host: "127.0.0.1",
  port: 8765,
  endpoint: "http://127.0.0.1:8765",
  websocketUrl: "ws://127.0.0.1:8765/ws",
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
    glass: "rgba(10, 13, 16, 0.74)",
    glassStrong: "rgba(13, 18, 22, 0.9)",
    border: "rgba(208, 221, 230, 0.18)",
    text: "#eef6f8",
    textMuted: "#9fb0b5",
    cyan: "#66e3ff",
    jade: "#65f0bd",
    amber: "#ffc857",
    rose: "#ff7a90"
  },
  radius: {
    panel: 8,
    control: 8,
    round: 999
  },
  shadow: {
    panel: "0 22px 70px rgba(0, 0, 0, 0.42)",
    glow: "0 0 28px rgba(102, 227, 255, 0.24)"
  }
} as const;
