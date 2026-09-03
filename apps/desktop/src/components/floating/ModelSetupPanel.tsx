import { useState, useEffect } from "react";
import type { AssistantSnapshot } from "../../stores/assistantStore";
import type { LocalModelInfo, LocalModelRole, HfGgufFile, ModelSetupTab } from "@deyana/schemas";
import {
  Cpu,
  RefreshCw,
  Zap,
  Layers,
  Flame,
  Download,
  Shield,
  CheckCircle2,
  Trash2,
  Play,
  Square,
  Search,
  HardDrive,
  Check,
  AlertTriangle,
  Info
} from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface ModelSetupPanelProps {
  snapshot: AssistantSnapshot;
}

const PRESET_REPOS = [
  { label: "Qwen 2.5 7B", repo: "Qwen/Qwen2.5-7B-Instruct-GGUF" },
  { label: "Llama 3.2 3B", repo: "bartowski/Llama-3.2-3B-Instruct-GGUF" },
  { label: "Mistral 7B", repo: "bartowski/Mistral-7B-Instruct-v0.3-GGUF" },
  { label: "Phi-3.5 Mini", repo: "bartowski/Phi-3.5-mini-instruct-GGUF" }
];

const CONTEXT_OPTIONS = [2048, 4096, 8192, 16384, 32768];

export function ModelSetupPanel({ snapshot }: ModelSetupPanelProps) {
  const [selectedFileForGauge, setSelectedFileForGauge] = useState<HfGgufFile | null>(null);
  const [contextTokens, setContextTokens] = useState<number>(4096);
  const [confirmClear, setConfirmClear] = useState<boolean>(false);

  const activeTab: ModelSetupTab = snapshot.modelSetupTab ?? "local";

  useEffect(() => {
    if (snapshot.registeredModels.length === 0 && !snapshot.registeredModelsLoading) {
      void assistantStore.loadRegisteredModels();
    }
    if (!snapshot.activeProfile) {
      void assistantStore.loadActiveProfile();
    }
    if (!snapshot.llamaCppStatus) {
      void assistantStore.loadLlamaCppStatus();
    }
    if (!snapshot.datasetStats) {
      void assistantStore.loadDatasetStats();
    }
  }, []);

  const status = snapshot.modelStatusDetail;
  const chatModels = modelOptions(status?.setupModels ?? [], "chat", snapshot.coreSettings.selectedChatModel);
  const embeddingModels = modelOptions(
    status?.setupModels ?? [],
    "embedding",
    snapshot.coreSettings.selectedEmbeddingModel
  );

  // Active file for RAM estimation gauge (fall back to first available HF file)
  const activeGaugeFile = selectedFileForGauge || snapshot.hfFiles[0] || null;
  const systemRamGb = snapshot.activeProfile?.systemRamGb ?? 16;
  const weightsRamGb = activeGaugeFile
    ? (activeGaugeFile.ramEstimateGb || (activeGaugeFile.sizeBytes / (1024 * 1024 * 1024)) * 1.15)
    : 4.5;
  const kvCacheRamGb = (contextTokens / 4096) * 0.45;
  const runtimeOverheadGb = 0.8;
  const totalEstimatedRamGb = weightsRamGb + kvCacheRamGb + runtimeOverheadGb;
  const ramUsagePercent = Math.min(100, Math.round((totalEstimatedRamGb / systemRamGb) * 100));

  const ramSafetyTier =
    ramUsagePercent < 70 ? "optimal" : ramUsagePercent <= 90 ? "warning" : "danger";

  return (
    <section className="model-setup-root" aria-label="Local model management and flywheel">
      {/* Segmented Navigation Tab Bar */}
      <nav className="model-setup-nav-tabs" aria-label="Model Setup Tabs">
        <button
          type="button"
          className={`model-nav-tab ${activeTab === "local" ? "active" : ""}`}
          onClick={() => assistantStore.setModelSetupTab("local")}
        >
          <HardDrive size={13} aria-hidden="true" />
          <span>Local Runtimes</span>
        </button>
        <button
          type="button"
          className={`model-nav-tab ${activeTab === "hf" ? "active" : ""}`}
          onClick={() => {
            assistantStore.setModelSetupTab("hf");
            if (snapshot.hfFiles.length === 0 && !snapshot.hfLoading) {
              void assistantStore.searchHfGgufFiles();
            }
          }}
        >
          <Layers size={13} aria-hidden="true" />
          <span>HuggingFace Hub</span>
        </button>
        <button
          type="button"
          className={`model-nav-tab ${activeTab === "flywheel" ? "active" : ""}`}
          onClick={() => {
            assistantStore.setModelSetupTab("flywheel");
            void assistantStore.loadDatasetStats();
          }}
        >
          <Flame size={13} aria-hidden="true" />
          <span>Fine-Tuning Flywheel</span>
        </button>
      </nav>

      {/* Tab 1: Local Runtimes */}
      {activeTab === "local" && (
        <div className="model-tab-content">
          <header className="model-setup-header">
            <div className="section-heading">
              <Cpu size={15} aria-hidden="true" />
              <span>Ollama & llama.cpp Runtimes</span>
            </div>
            <div className="model-tools">
              <button
                className="icon-button"
                type="button"
                title="Refresh model status"
                aria-label="Refresh model status"
                onClick={() => void assistantStore.loadModelStatus()}
              >
                <RefreshCw size={14} aria-hidden="true" />
              </button>
              <button
                className="icon-button"
                type="button"
                title="Run local test prompt"
                aria-label="Run local test prompt"
                disabled={snapshot.modelTestBusy || snapshot.modelStatus !== "available"}
                onClick={() => void assistantStore.testModel()}
              >
                <Zap size={14} aria-hidden="true" />
              </button>
            </div>
          </header>

          {/* Hardware Profile Badge */}
          <div className="hardware-profile-banner">
            <div className="profile-chip">
              <span className="profile-tier-badge">
                {snapshot.activeProfile?.tier?.toUpperCase() || "BALANCED"}
              </span>
              <span className="profile-ram">
                {snapshot.activeProfile?.systemRamGb || 16} GB RAM
              </span>
            </div>
            <div className="profile-recommendation">
              Recommend: <strong>{snapshot.activeProfile?.recommendedQuant || "Q4_K_M"}</strong> • {snapshot.activeProfile?.recommendedContext || 4096} tokens ctx
            </div>
          </div>

          {/* Ollama Model Selection Grid */}
          <div className="model-select-grid">
            <label className="model-field">
              <span>Chat Model</span>
              <select
                value={snapshot.coreSettings.selectedChatModel}
                onChange={(event) => void assistantStore.selectModel({ chatModel: event.target.value })}
              >
                {chatModels.map((model) => (
                  <option key={model.name} value={model.name}>
                    {modelLabel(model)}
                  </option>
                ))}
              </select>
            </label>
            <label className="model-field">
              <span>Embedding Model</span>
              <select
                value={snapshot.coreSettings.selectedEmbeddingModel}
                onChange={(event) => void assistantStore.selectModel({ embeddingModel: event.target.value })}
              >
                {embeddingModels.map((model) => (
                  <option key={model.name} value={model.name}>
                    {modelLabel(model)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className={`model-state model-state-${snapshot.modelStatus}`}>
            {status?.message ?? "Checking local model runtime"}
          </div>

          {snapshot.modelTestResponse ? (
            <div className="model-test-result">
              {snapshot.modelTestResponse.model}: {snapshot.modelTestResponse.response}
            </div>
          ) : null}

          {/* Local Registered GGUFs & llama.cpp Server Section */}
          <div className="llamacpp-section">
            <div className="sub-heading">
              <ServerIcon running={Boolean(snapshot.llamaCppStatus?.running)} />
              <span>Local llama.cpp Server</span>
              {snapshot.llamaCppStatus?.running ? (
                <span className="running-badge">Port {snapshot.llamaCppStatus.port || 8080} (PID {snapshot.llamaCppStatus.pid})</span>
              ) : (
                <span className="stopped-badge">Offline</span>
              )}
            </div>

            {snapshot.registeredModels.length === 0 ? (
              <div className="empty-models-notice">
                <Info size={13} />
                <span>No local GGUF models registered. Browse the HuggingFace Hub tab to download models.</span>
              </div>
            ) : (
              <div className="registered-models-list">
                {snapshot.registeredModels.map((model) => {
                  const isServerRunningThis =
                    snapshot.llamaCppStatus?.running &&
                    snapshot.llamaCppStatus?.model === model.path;

                  return (
                    <div key={model.id} className="registered-model-card">
                      <div className="model-card-info">
                        <span className="model-name" title={model.path}>{model.name}</span>
                        <div className="model-badges">
                          <span className="quant-tag">{model.quantization}</span>
                          <span className="size-badge">{formatBytes(model.sizeBytes)}</span>
                          <span className="ram-badge">~{model.ramEstimateGb} GB RAM</span>
                        </div>
                      </div>
                      <div className="model-card-actions">
                        {isServerRunningThis ? (
                          <button
                            type="button"
                            className="btn-server-stop"
                            title="Stop llama.cpp server"
                            onClick={() => void assistantStore.stopLlamaCpp()}
                          >
                            <Square size={11} /> Stop
                          </button>
                        ) : (
                          <button
                            type="button"
                            className="btn-server-start"
                            title="Start llama.cpp server with this model"
                            onClick={() => void assistantStore.startLlamaCpp(model.path)}
                          >
                            <Play size={11} /> Run
                          </button>
                        )}
                        <button
                          type="button"
                          className="btn-model-delete"
                          title="Delete registered model"
                          onClick={() => void assistantStore.deleteModel(model.id)}
                        >
                          <Trash2 size={12} />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Tab 2: HuggingFace GGUF Hub */}
      {activeTab === "hf" && (
        <div className="model-tab-content">
          {/* Preset Repos Chips */}
          <div className="hf-presets-container">
            <span className="preset-label">Popular Presets:</span>
            <div className="preset-chips">
              {PRESET_REPOS.map((preset) => (
                <button
                  key={preset.repo}
                  type="button"
                  className={`preset-chip ${snapshot.hfRepoQuery === preset.repo ? "active" : ""}`}
                  onClick={() => void assistantStore.selectHfPreset(preset.repo)}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          </div>

          {/* Repo Search Bar */}
          <div className="hf-search-bar">
            <input
              type="text"
              placeholder="e.g. Qwen/Qwen2.5-7B-Instruct-GGUF"
              value={snapshot.hfRepoQuery}
              onChange={(e) => assistantStore.setHfRepoQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void assistantStore.searchHfGgufFiles();
              }}
            />
            <button
              type="button"
              className="btn-search"
              disabled={snapshot.hfLoading}
              onClick={() => void assistantStore.searchHfGgufFiles()}
            >
              {snapshot.hfLoading ? <RefreshCw size={13} className="spin" /> : <Search size={13} />}
              <span>Fetch</span>
            </button>
          </div>

          {snapshot.hfError && (
            <div className="hf-error-banner">
              <AlertTriangle size={13} />
              <span>{snapshot.hfError}</span>
            </div>
          )}

          {/* Live RAM Estimator Gauge */}
          {activeGaugeFile && (
            <div className={`ram-gauge-card ram-gauge-${ramSafetyTier}`}>
              <div className="ram-gauge-header">
                <div className="ram-gauge-title">
                  <Cpu size={13} />
                  <span>Live RAM Estimator Gauge</span>
                </div>
                <div className={`ram-safety-indicator badge-${ramSafetyTier}`}>
                  {ramSafetyTier === "optimal" && <CheckCircle2 size={12} />}
                  {ramSafetyTier === "warning" && <AlertTriangle size={12} />}
                  {ramSafetyTier === "danger" && <AlertTriangle size={12} />}
                  <span>
                    {ramSafetyTier === "optimal" ? "Safe" : ramSafetyTier === "warning" ? "High Pressure" : "OOM Risk"}
                  </span>
                </div>
              </div>

              <div className="ram-gauge-details">
                <div className="ram-stat-line">
                  <span className="stat-label">Model Target:</span>
                  <span className="stat-value">{activeGaugeFile.filename} ({activeGaugeFile.quantization})</span>
                </div>
                <div className="ram-stat-line">
                  <span className="stat-label">Estimated Memory:</span>
                  <span className="stat-value">
                    <strong>{totalEstimatedRamGb.toFixed(1)} GB</strong> / {systemRamGb} GB ({ramUsagePercent}%)
                  </span>
                </div>
              </div>

              {/* Visual Gauge Bar */}
              <div className="ram-gauge-bar-track" role="progressbar" aria-valuenow={ramUsagePercent} aria-valuemin={0} aria-valuemax={100}>
                <div
                  className={`ram-gauge-bar-fill fill-${ramSafetyTier}`}
                  style={{ width: `${ramUsagePercent}%` }}
                />
              </div>

              {/* Context Selector Pills */}
              <div className="context-selector-row">
                <span className="context-label">Context Size:</span>
                <div className="context-pills">
                  {CONTEXT_OPTIONS.map((ctx) => (
                    <button
                      key={ctx}
                      type="button"
                      className={`context-pill ${contextTokens === ctx ? "active" : ""}`}
                      onClick={() => setContextTokens(ctx)}
                    >
                      {ctx >= 1024 ? `${ctx / 1024}K` : ctx}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* HuggingFace GGUF Files List */}
          <div className="hf-files-list">
            {snapshot.hfFiles.length === 0 && !snapshot.hfLoading ? (
              <div className="empty-files-hint">
                <span>Enter a HuggingFace repository ID and click Fetch to browse available .gguf quantizations.</span>
              </div>
            ) : (
              snapshot.hfFiles.map((file) => {
                const downloadKey = `${snapshot.hfRepoQuery}/${file.filename}`;
                const downloadProgress =
                  snapshot.activeDownloads[downloadKey] ||
                  snapshot.activeDownloads[file.filename];

                const isInstalled = snapshot.registeredModels.some(
                  (m) => m.filename === file.filename || m.name === file.filename
                );
                const isSelectedForGauge = activeGaugeFile?.filename === file.filename;

                return (
                  <div
                    key={file.filename}
                    className={`hf-file-row ${isSelectedForGauge ? "selected-gauge" : ""}`}
                    onClick={() => setSelectedFileForGauge(file)}
                  >
                    <div className="file-info-group">
                      <div className="file-primary-line">
                        <span className="quant-tag">{file.quantization}</span>
                        <span className="filename-text" title={file.filename}>{file.filename}</span>
                      </div>
                      <div className="file-meta-line">
                        <span>{formatBytes(file.sizeBytes)}</span>
                        <span>•</span>
                        <span>~{file.ramEstimateGb} GB RAM</span>
                        {file.recommendedProfile && (
                          <>
                            <span>•</span>
                            <span className="tier-tag">{file.recommendedProfile}</span>
                          </>
                        )}
                      </div>
                    </div>

                    <div className="file-action-group" onClick={(e) => e.stopPropagation()}>
                      {isInstalled ? (
                        <div className="installed-badge">
                          <Check size={12} /> Installed
                        </div>
                      ) : downloadProgress && downloadProgress.percent < 100 ? (
                        <div className="download-progress-container">
                          <div className="download-progress-text">
                            <span>{downloadProgress.percent.toFixed(0)}%</span>
                            {downloadProgress.total_bytes > 0 && (
                              <span className="bytes-text">
                                {formatBytes(downloadProgress.bytes_downloaded)} / {formatBytes(downloadProgress.total_bytes)}
                              </span>
                            )}
                          </div>
                          <div className="download-progress-bar">
                            <div
                              className="download-progress-fill"
                              style={{ width: `${Math.max(5, downloadProgress.percent)}%` }}
                            />
                          </div>
                        </div>
                      ) : (
                        <button
                          type="button"
                          className="btn-download-model"
                          onClick={() => void assistantStore.downloadHfModel(snapshot.hfRepoQuery, file.filename)}
                        >
                          <Download size={12} />
                          <span>Get</span>
                        </button>
                      )}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}

      {/* Tab 3: Fine-Tuning Flywheel */}
      {activeTab === "flywheel" && (
        <div className="model-tab-content">
          {/* Automatic PII Masking Shield Badge */}
          <div className="pii-shield-banner">
            <Shield size={16} className="shield-icon" aria-hidden="true" />
            <div className="shield-text">
              <strong>Autonomous Zero-Leakage PII Redaction</strong>
              <span>API keys (sk-...), tokens, and emails are automatically masked prior to persistence.</span>
            </div>
          </div>

          {/* Dataset Statistics Grid */}
          <div className="dataset-stats-grid">
            <div className="dataset-stat-card">
              <span className="stat-number">{snapshot.datasetStats?.totalCount ?? 0}</span>
              <span className="stat-label">Total Logged Turns</span>
            </div>
            <div className="dataset-stat-card">
              <span className="stat-number text-emerald">{snapshot.datasetStats?.positiveCount ?? 0}</span>
              <span className="stat-label">Positive Feedback (+1)</span>
            </div>
            <div className="dataset-stat-card">
              <span className="stat-number text-rose">{snapshot.datasetStats?.negativeCount ?? 0}</span>
              <span className="stat-label">Negative Feedback (-1)</span>
            </div>
            <div className="dataset-stat-card">
              <span className="stat-number">
                {snapshot.datasetStats?.modelCounts ? Object.keys(snapshot.datasetStats.modelCounts).length : 0}
              </span>
              <span className="stat-label">Models Logged</span>
            </div>
          </div>

          {/* Model Breakdown */}
          {snapshot.datasetStats?.modelCounts && Object.keys(snapshot.datasetStats.modelCounts).length > 0 && (
            <div className="models-breakdown-card">
              <span className="breakdown-title">Model Distribution</span>
              <div className="breakdown-items">
                {Object.entries(snapshot.datasetStats.modelCounts).map(([model, count]) => (
                  <div key={model} className="breakdown-item">
                    <span className="breakdown-model-name">{model}</span>
                    <span className="breakdown-count">{count} turns</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 1-Click Export Actions */}
          <div className="flywheel-export-box">
            <span className="export-title">1-Click Fine-Tuning Dataset Export</span>
            <p className="export-desc">
              Export recorded interactions directly to standard JSONL formats for Supervised Fine-Tuning (SFT) or Direct Preference Optimization (DPO).
            </p>

            <div className="export-buttons-row">
              <button
                type="button"
                className="btn-export btn-sharegpt"
                disabled={snapshot.datasetBusy}
                onClick={() => void assistantStore.exportDataset("sharegpt")}
              >
                <Download size={13} />
                <span>Export ShareGPT JSONL</span>
              </button>
              <button
                type="button"
                className="btn-export btn-dpo"
                disabled={snapshot.datasetBusy}
                onClick={() => void assistantStore.exportDataset("dpo")}
              >
                <Download size={13} />
                <span>Export DPO Preference Pairs</span>
              </button>
            </div>

            {/* Clear Dataset Control */}
            <div className="clear-dataset-row">
              {confirmClear ? (
                <div className="confirm-clear-box">
                  <span>Clear all logged interaction records?</span>
                  <div className="confirm-actions">
                    <button
                      type="button"
                      className="btn-confirm-yes"
                      onClick={() => {
                        setConfirmClear(false);
                        void assistantStore.clearDataset();
                      }}
                    >
                      Yes, Clear
                    </button>
                    <button
                      type="button"
                      className="btn-confirm-no"
                      onClick={() => setConfirmClear(false)}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  className="btn-clear-dataset"
                  disabled={snapshot.datasetBusy || (snapshot.datasetStats?.totalCount ?? 0) === 0}
                  onClick={() => setConfirmClear(true)}
                >
                  <Trash2 size={12} />
                  <span>Clear Dataset</span>
                </button>
              )}
            </div>

            {/* Export Status Banner */}
            {snapshot.datasetMessage && (
              <div className="dataset-status-banner">
                <CheckCircle2 size={13} />
                <span>{snapshot.datasetMessage}</span>
              </div>
            )}
          </div>
        </div>
      )}
    </section>
  );
}

function ServerIcon({ running }: { running: boolean }) {
  return (
    <span className={`server-indicator-dot ${running ? "dot-online" : "dot-offline"}`} />
  );
}

const formatBytes = (bytes: number): string => {
  if (!bytes || bytes <= 0) return "0 B";
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const modelOptions = (
  models: LocalModelInfo[],
  role: LocalModelRole,
  selectedModel: string
): LocalModelInfo[] => {
  const matching = models.filter((model) => model.role === role || model.name === selectedModel);
  if (!matching.some((model) => model.name === selectedModel)) {
    matching.unshift({
      name: selectedModel,
      role,
      installed: false,
      recommended: false,
      profile: null,
      sizeBytes: null,
      detail: "Selected local model"
    });
  }
  return matching;
};

const modelLabel = (model: LocalModelInfo) => {
  const state = model.installed ? "installed" : "missing";
  return model.recommended ? `${model.name} (${state}, recommended)` : `${model.name} (${state})`;
};
