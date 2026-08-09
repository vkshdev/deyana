import { useEffect, useRef, useState } from "react";
import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Activity, Mic, Volume2, Sparkles } from "lucide-react";

interface VoiceWaveformVisualizerProps {
  snapshot: AssistantSnapshot;
  className?: string;
  height?: number;
}

export function VoiceWaveformVisualizer({
  snapshot,
  className = "",
  height = 56
}: VoiceWaveformVisualizerProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [micActive, setMicActive] = useState(false);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const animFrameRef = useRef<number | null>(null);

  // Support both snapshot.assistantState and any voiceState property
  const currentState = snapshot.assistantState || (snapshot as unknown as { voiceState?: string }).voiceState || "IDLE";
  const isListening = currentState === "LISTENING";
  const isSpeaking = currentState === "SPEAKING";
  const isThinking = currentState === "THINKING" || currentState === "TRANSCRIBING";

  // Web Audio API mic input capture setup when listening
  useEffect(() => {
    let mounted = true;

    if (isListening && typeof window !== "undefined" && window.navigator?.mediaDevices?.getUserMedia) {
      window.navigator.mediaDevices
        .getUserMedia({ audio: true })
        .then((stream) => {
          if (!mounted) {
            stream.getTracks().forEach((track) => track.stop());
            return;
          }
          try {
            const AudioCtx =
              window.AudioContext ||
              (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
            const ctx = new AudioCtx();
            const analyser = ctx.createAnalyser();
            analyser.fftSize = 64;
            analyser.smoothingTimeConstant = 0.8;
            const source = ctx.createMediaStreamSource(stream);
            source.connect(analyser);

            audioCtxRef.current = ctx;
            analyserRef.current = analyser;
            streamRef.current = stream;
            setMicActive(true);
          } catch (err) {
            console.warn("AudioContext init skipped/failed:", err);
            setMicActive(false);
          }
        })
        .catch(() => {
          if (mounted) setMicActive(false);
        });
    } else {
      setMicActive(false);
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
        streamRef.current = null;
      }
      if (audioCtxRef.current && audioCtxRef.current.state !== "closed") {
        audioCtxRef.current.close().catch(() => {});
        audioCtxRef.current = null;
      }
    }

    return () => {
      mounted = false;
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
        streamRef.current = null;
      }
      if (audioCtxRef.current && audioCtxRef.current.state !== "closed") {
        audioCtxRef.current.close().catch(() => {});
        audioCtxRef.current = null;
      }
    };
  }, [isListening]);

  // High-FPS 60FPS Canvas Animation Loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let phase = 0;
    const barCount = 32;

    const render = () => {
      const width = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, width, h);

      phase += 0.07;

      const freqs: number[] = new Array(barCount).fill(0);

      if (isListening && analyserRef.current && micActive) {
        const buffer = new Uint8Array(analyserRef.current.frequencyBinCount);
        analyserRef.current.getByteFrequencyData(buffer);
        for (let i = 0; i < barCount; i++) {
          const sampleIdx = Math.floor((i / barCount) * buffer.length);
          freqs[i] = buffer[sampleIdx] / 255;
        }
      } else if (isListening) {
        // Procedural listening waveform with dynamic arc-reactor harmonics
        for (let i = 0; i < barCount; i++) {
          const sinVal = Math.sin(phase * 1.6 + i * 0.4);
          const cosVal = Math.cos(phase * 0.9 - i * 0.25);
          freqs[i] = Math.max(0.12, Math.min(0.95, 0.45 + sinVal * 0.35 + cosVal * 0.2));
        }
      } else if (isSpeaking) {
        // Rhythmic speaking spectrum with gold highlights
        for (let i = 0; i < barCount; i++) {
          const centerDist = 1 - Math.abs(i - barCount / 2) / (barCount / 2);
          const wave = Math.sin(phase * 2.4 + i * 0.5) * Math.cos(phase * 1.2 + i * 0.2);
          freqs[i] = Math.max(0.15, Math.min(1.0, (0.55 + wave * 0.45) * centerDist * 1.25));
        }
      } else if (isThinking) {
        // Gentle pulse phase sweep for processing
        for (let i = 0; i < barCount; i++) {
          const wave = Math.sin(phase * 1.4 + i * 0.3);
          freqs[i] = Math.max(0.08, 0.22 + wave * 0.18);
        }
      } else {
        // Ambient idle breathing wave
        for (let i = 0; i < barCount; i++) {
          freqs[i] = 0.08 + Math.sin(phase * 0.5 + i * 0.2) * 0.05;
        }
      }

      // Draw background glass grid glow
      const gap = 3;
      const totalGaps = (barCount - 1) * gap;
      const barWidth = Math.max(2, (width - totalGaps) / barCount);

      for (let i = 0; i < barCount; i++) {
        const x = i * (barWidth + gap);
        const val = freqs[i];
        const barHeight = Math.max(4, val * (h - 12));
        const y = (h - barHeight) / 2;
        const rx = Math.min(barWidth / 2, 3);

        // Palette: Arc-reactor blue (#83a9d4), Gold (#d8b978)
        const isPeak = val > 0.65;
        const colorPrimary = isPeak ? "#d8b978" : "#83a9d4";
        const colorSecondary = isPeak ? "rgba(216, 185, 120, 0.35)" : "rgba(131, 169, 212, 0.3)";

        ctx.shadowBlur = isPeak ? 10 : 4;
        ctx.shadowColor = colorPrimary;

        const grad = ctx.createLinearGradient(0, y, 0, y + barHeight);
        grad.addColorStop(0, colorPrimary);
        grad.addColorStop(1, colorSecondary);

        ctx.fillStyle = grad;
        ctx.beginPath();
        if (typeof ctx.roundRect === "function") {
          ctx.roundRect(x, y, barWidth, barHeight, rx);
        } else {
          ctx.rect(x, y, barWidth, barHeight);
        }
        ctx.fill();

        // Draw top accent gold peak indicator
        if (isPeak && barHeight > 16) {
          ctx.fillStyle = "#f1d9a7";
          ctx.beginPath();
          ctx.arc(x + barWidth / 2, y - 2, 1.5, 0, Math.PI * 2);
          ctx.fill();
        }
      }

      animFrameRef.current = requestAnimationFrame(render);
    };

    render();

    return () => {
      if (animFrameRef.current !== null) {
        cancelAnimationFrame(animFrameRef.current);
      }
    };
  }, [isListening, isSpeaking, isThinking, micActive]);

  const stateLabel = isListening
    ? micActive
      ? "Live Voice Input"
      : "Listening..."
    : isSpeaking
    ? "Deyana Speaking..."
    : isThinking
    ? "Processing Speech..."
    : "Voice Waveform";

  return (
    <div
      className={`voice-waveform-visualizer ${className}`}
      style={{
        background: "rgba(4, 8, 14, 0.82)",
        border: "1px solid rgba(131, 169, 212, 0.25)",
        borderRadius: "8px",
        padding: "8px 12px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.4), inset 0 0 12px rgba(131, 169, 212, 0.1)",
        backdropFilter: "blur(16px)"
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: "6px"
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          {isListening ? (
            <Mic size={13} style={{ color: "#83a9d4" }} className="animate-pulse" aria-hidden="true" />
          ) : isSpeaking ? (
            <Volume2 size={13} style={{ color: "#d8b978" }} className="animate-pulse" aria-hidden="true" />
          ) : isThinking ? (
            <Sparkles size={13} style={{ color: "#83a9d4" }} className="animate-spin" aria-hidden="true" />
          ) : (
            <Activity size={13} style={{ color: "#a8b2c0" }} aria-hidden="true" />
          )}
          <span
            style={{
              fontSize: "11px",
              fontWeight: 600,
              letterSpacing: "0.5px",
              color: isSpeaking ? "#d8b978" : isListening ? "#83a9d4" : "#a8b2c0"
            }}
          >
            {stateLabel}
          </span>
        </div>
        {(isListening || isSpeaking) && (
          <span
            style={{
              fontSize: "9px",
              fontWeight: 700,
              letterSpacing: "0.8px",
              color: "#83a9d4",
              background: "rgba(131, 169, 212, 0.15)",
              border: "1px solid rgba(131, 169, 212, 0.3)",
              borderRadius: "4px",
              padding: "1px 5px",
              display: "flex",
              alignItems: "center",
              gap: "4px"
            }}
          >
            <span
              style={{
                width: "5px",
                height: "5px",
                borderRadius: "50%",
                background: isSpeaking ? "#d8b978" : "#83a9d4",
                boxShadow: `0 0 6px ${isSpeaking ? "#d8b978" : "#83a9d4"}`
              }}
            />
            HOLOGRAPHIC HUD
          </span>
        )}
      </div>

      <div style={{ display: "flex", justifyContent: "center", width: "100%" }}>
        <canvas
          ref={canvasRef}
          width={340}
          height={height}
          style={{ width: "100%", height: `${height}px`, display: "block" }}
        />
      </div>
    </div>
  );
}
