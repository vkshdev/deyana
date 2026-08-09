import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Mic, MicOff, Square, Volume2, VolumeX } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { VoiceWaveformVisualizer } from "./VoiceWaveformVisualizer";

interface VoicePanelProps {
  snapshot: AssistantSnapshot;
}

export function VoicePanel({ snapshot }: VoicePanelProps) {
  const settings = snapshot.voiceSettings;
  const status = snapshot.voiceStatus;
  const enabled = settings?.enabled ?? false;
  const muted = settings?.muted ?? true;
  const ttsEnabled = settings?.ttsEnabled ?? false;
  const availableVoices = status?.availableTtsVoices ?? [];
  const selectedVoice = settings?.ttsVoice ?? status?.activeTtsVoice ?? "";
  const canListen = enabled && !muted && !snapshot.voiceBusy;

  return (
    <section
      className="voice-panel"
      aria-label="Local voice"
      style={{
        background: "rgba(4, 8, 14, 0.82)",
        border: "1px solid rgba(131, 169, 212, 0.22)",
        borderRadius: "10px",
        padding: "12px",
        display: "flex",
        flexDirection: "column",
        gap: "10px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.45), 0 0 16px rgba(131, 169, 212, 0.12)",
        backdropFilter: "blur(20px)"
      }}
    >
      <div className="voice-panel-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <strong style={{ color: "#f3f1ec", fontSize: "13px", fontWeight: 600 }}>Voice Control</strong>
          <span style={{ display: "block", color: "#a8b2c0", fontSize: "11px" }}>
            {status?.detail ?? "Local voice engine active"}
          </span>
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
          style={{
            borderColor: enabled ? "#d8b978" : "rgba(164, 184, 210, 0.2)",
            color: enabled ? "#d8b978" : "#a8b2c0",
            boxShadow: enabled ? "0 0 12px rgba(216, 185, 120, 0.3)" : "none"
          }}
        >
          {enabled ? <Mic size={17} aria-hidden="true" /> : <MicOff size={17} aria-hidden="true" />}
        </button>
      </div>

      {/* Real-time Voice Waveform Visualizer */}
      <VoiceWaveformVisualizer snapshot={snapshot} height={52} />

      <div className="voice-actions" style={{ display: "flex", gap: "6px", alignItems: "center" }}>
        <button
          className="voice-control"
          type="button"
          title="Push to talk"
          disabled={!canListen}
          onClick={() => void assistantStore.runPushToTalk()}
          style={{
            flex: 1,
            background: canListen
              ? "linear-gradient(135deg, rgba(216, 185, 120, 0.25), rgba(131, 169, 212, 0.2))"
              : "rgba(255, 255, 255, 0.05)",
            border: `1px solid ${canListen ? "#d8b978" : "rgba(255, 255, 255, 0.1)"}`,
            color: canListen ? "#d8b978" : "#a8b2c0",
            boxShadow: canListen ? "0 0 16px rgba(216, 185, 120, 0.25)" : "none",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: "8px",
            padding: "8px 12px",
            borderRadius: "6px",
            fontWeight: 600,
            fontSize: "12px",
            cursor: canListen ? "pointer" : "not-allowed"
          }}
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
          style={{
            borderColor: muted ? "rgba(231, 132, 148, 0.4)" : "rgba(131, 169, 212, 0.3)",
            color: muted ? "#e78494" : "#83a9d4"
          }}
        >
          {muted ? <MicOff size={17} aria-hidden="true" /> : <Mic size={17} aria-hidden="true" />}
        </button>

        <button
          className="icon-button"
          type="button"
          title={ttsEnabled ? "Disable speech" : "Enable speech"}
          aria-label={ttsEnabled ? "Disable speech" : "Enable speech"}
          onClick={() => void assistantStore.patchVoiceSettings({ ttsEnabled: !ttsEnabled })}
          style={{
            borderColor: ttsEnabled ? "rgba(131, 169, 212, 0.4)" : "rgba(255, 255, 255, 0.1)",
            color: ttsEnabled ? "#83a9d4" : "#a8b2c0"
          }}
        >
          {ttsEnabled ? <Volume2 size={17} aria-hidden="true" /> : <VolumeX size={17} aria-hidden="true" />}
        </button>

        <button
          className="icon-button"
          type="button"
          title="Stop speech"
          aria-label="Stop speech"
          disabled={!ttsEnabled}
          onClick={() => void assistantStore.interruptVoice()}
          style={{ color: "#e78494" }}
        >
          <Square size={15} aria-hidden="true" />
        </button>

        <button
          className="icon-button"
          type="button"
          title="Speak last response"
          aria-label="Speak last response"
          disabled={!ttsEnabled || snapshot.voiceBusy}
          onClick={() => void assistantStore.speakLastAssistantMessage()}
          style={{ color: "#d8b978" }}
        >
          <Volume2 size={17} aria-hidden="true" />
        </button>
      </div>

      <label className="voice-selector" style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        <span style={{ fontSize: "11px", color: "#a8b2c0" }}>Deyana voice profile</span>
        <select
          value={selectedVoice}
          disabled={!availableVoices.length || snapshot.voiceBusy}
          onChange={(event) =>
            void assistantStore.patchVoiceSettings({ ttsVoice: event.currentTarget.value || null })
          }
          style={{
            background: "rgba(9, 19, 33, 0.9)",
            border: "1px solid rgba(131, 169, 212, 0.3)",
            color: "#f3f1ec",
            borderRadius: "6px",
            padding: "6px 8px",
            fontSize: "12px"
          }}
        >
          {!availableVoices.length ? <option value="">No local voices found</option> : null}
          {availableVoices.map((voice) => (
            <option key={voice.name} value={voice.name}>
              {voice.name} - Female - {voice.language}
            </option>
          ))}
        </select>
      </label>

      {snapshot.voiceTranscript?.transcript ? (
        <div
          className="voice-transcript"
          style={{
            background: "rgba(9, 19, 33, 0.8)",
            border: "1px solid rgba(216, 185, 120, 0.25)",
            borderRadius: "6px",
            padding: "8px 10px",
            fontSize: "11px",
            color: "#d8b978"
          }}
        >
          <span>{snapshot.voiceTranscript.transcript}</span>
        </div>
      ) : null}
    </section>
  );
}
