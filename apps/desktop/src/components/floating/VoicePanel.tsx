import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Mic, MicOff, Volume2, VolumeX } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface VoicePanelProps {
  snapshot: AssistantSnapshot;
}

export function VoicePanel({ snapshot }: VoicePanelProps) {
  const settings = snapshot.voiceSettings;
  const status = snapshot.voiceStatus;
  const enabled = settings?.enabled ?? false;
  const muted = settings?.muted ?? true;
  const ttsEnabled = settings?.ttsEnabled ?? false;
  const canListen = enabled && !muted && !snapshot.voiceBusy;

  return (
    <section className="voice-panel" aria-label="Local voice">
      <div className="voice-panel-header">
        <div>
          <strong>Voice</strong>
          <span>{status?.detail ?? "Local voice is loading"}</span>
        </div>
        <button
          className={enabled ? "icon-button voice-enabled" : "icon-button"}
          type="button"
          title={enabled ? "Disable voice" : "Enable voice"}
          aria-label={enabled ? "Disable voice" : "Enable voice"}
          onClick={() =>
            void assistantStore.patchVoiceSettings({
              enabled: !enabled,
              muted: enabled ? true : false
            })
          }
        >
          {enabled ? <Mic size={17} aria-hidden="true" /> : <MicOff size={17} aria-hidden="true" />}
        </button>
      </div>

      <div className="voice-actions">
        <button
          className="voice-control"
          type="button"
          title="Push to talk"
          disabled={!canListen}
          onClick={() => void assistantStore.runPushToTalk()}
        >
          <Mic size={17} aria-hidden="true" />
          <span>{snapshot.voiceBusy ? snapshot.assistantState.replaceAll("_", " ") : "Push to talk"}</span>
        </button>
        <button
          className="icon-button"
          type="button"
          title={muted ? "Unmute microphone" : "Mute microphone"}
          aria-label={muted ? "Unmute microphone" : "Mute microphone"}
          onClick={() => void assistantStore.patchVoiceSettings({ muted: !muted })}
        >
          {muted ? <MicOff size={17} aria-hidden="true" /> : <Mic size={17} aria-hidden="true" />}
        </button>
        <button
          className="icon-button"
          type="button"
          title={ttsEnabled ? "Disable speech" : "Enable speech"}
          aria-label={ttsEnabled ? "Disable speech" : "Enable speech"}
          onClick={() => void assistantStore.patchVoiceSettings({ ttsEnabled: !ttsEnabled })}
        >
          {ttsEnabled ? <Volume2 size={17} aria-hidden="true" /> : <VolumeX size={17} aria-hidden="true" />}
        </button>
        <button
          className="icon-button"
          type="button"
          title="Speak last response"
          aria-label="Speak last response"
          disabled={!ttsEnabled || snapshot.voiceBusy}
          onClick={() => void assistantStore.speakLastAssistantMessage()}
        >
          <Volume2 size={17} aria-hidden="true" />
        </button>
      </div>

      {snapshot.voiceTranscript?.transcript ? (
        <div className="voice-transcript">
          <span>{snapshot.voiceTranscript.transcript}</span>
        </div>
      ) : null}
    </section>
  );
}
