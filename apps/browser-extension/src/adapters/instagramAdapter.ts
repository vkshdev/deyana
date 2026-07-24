import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const instagramAdapter = createDraftFirstAdapter({
  id: "instagram",
  origin: "https://www.instagram.com",
  allowedOrigins: ["https://www.instagram.com", "https://instagram.com"],
  allowedHostnames: ["www.instagram.com", "instagram.com"],
  pathPrefixes: ["/direct/"],
  titlePrefix: "Instagram Direct",
  readLabel: "Read visible Instagram Direct thread",
  draftLabel: "Draft reply for Instagram Direct thread",
  fieldLabel: "Insert reviewed Instagram draft",
  titleSelectors: [
    "h1",
    "[role='main'] h2",
    "[role='main'] [role='heading']",
    "header span"
  ],
  selectors: [
    "[role='main'] [role='listitem']",
    "[role='main'] [role='row']",
    "[role='main'] span[dir='auto']",
    "div[contenteditable='true']",
    "textarea"
  ]
});
