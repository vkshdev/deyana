import { createDraftFirstAdapter } from "./draftFirstAdapterFactory";

export const slackAdapter = createDraftFirstAdapter({
  id: "slack",
  origin: "https://app.slack.com",
  titlePrefix: "Slack",
  readLabel: "Read visible Slack thread or channel",
  draftLabel: "Draft reply for visible Slack context",
  fieldLabel: "Insert reviewed Slack draft",
  titleSelectors: ["[data-qa='channel_name']", "[data-qa='channel_name_button']", "h1"],
  selectors: [
    "[data-qa='message_content']",
    "[data-qa='virtual-list-item']",
    "[role='listitem']",
    "[data-qa='thread_view']",
    "[data-qa='slack_kit_scrollbar']"
  ]
});
