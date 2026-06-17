import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Brain, Check, ChevronRight, FolderOpen, HardDrive, ShieldCheck } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { FloatingDockHandle } from "../floating/FloatingDockHandle";

interface OnboardingFlowProps {
  snapshot: AssistantSnapshot;
}

const vaultFolders = [
  "Daily",
  "Projects",
  "People",
  "Meetings",
  "Emails",
  "GitHub",
  "Slack",
  "Tasks",
  "Decisions",
  "Sources",
  "Inbox"
];

export function OnboardingFlow({ snapshot }: OnboardingFlowProps) {
  const step = snapshot.onboardingStep;

  return (
    <section className="floating-panel onboarding-panel" aria-label="DE'YANA onboarding">
      <FloatingDockHandle />
      <header className="onboarding-header">
        <div>
          <strong>DE'YANA</strong>
          <span>{step === "complete" ? "Ready" : "Private desktop setup"}</span>
        </div>
        <div className="onboarding-progress" aria-label="Onboarding progress">
          {["welcome", "privacy", "local_ai", "vault"].map((item) => (
            <span
              key={item}
              className={item === step ? "progress-dot progress-dot-active" : "progress-dot"}
            />
          ))}
        </div>
      </header>

      {step === "welcome" ? <WelcomeStep /> : null}
      {step === "privacy" ? <PrivacyStep /> : null}
      {step === "local_ai" ? <LocalAiStep snapshot={snapshot} /> : null}
      {step === "vault" || step === "complete" ? <VaultStep snapshot={snapshot} /> : null}

      {snapshot.error ? <div className="panel-error">{snapshot.error}</div> : null}
    </section>
  );
}

function WelcomeStep() {
  return (
    <div className="onboarding-step">
      <div className="onboarding-icon">
        <ShieldCheck size={24} aria-hidden="true" />
      </div>
      <div className="onboarding-copy">
        <h1>Private assistant, local first.</h1>
        <p>DE'YANA keeps the desktop shell, backend, settings, and vault on this computer.</p>
      </div>
      <button
        className="primary-action"
        type="button"
        onClick={() => assistantStore.setOnboardingStep("privacy")}
      >
        <span>Continue</span>
        <ChevronRight size={16} aria-hidden="true" />
      </button>
    </div>
  );
}

function PrivacyStep() {
  return (
    <div className="onboarding-step">
      <div className="onboarding-icon">
        <ShieldCheck size={24} aria-hidden="true" />
      </div>
      <div className="onboarding-copy">
        <h1>Private data never goes to cloud AI.</h1>
        <p>Connector content, local files, source code, notes, transcripts, memory, and summaries stay local.</p>
      </div>
      <div className="mode-row" role="radiogroup" aria-label="Privacy mode">
        <button className="mode-option mode-option-selected" type="button" role="radio" aria-checked="true">
          <ShieldCheck size={15} aria-hidden="true" />
          <span>Local only</span>
        </button>
      </div>
      <button
        className="primary-action"
        type="button"
        onClick={() => assistantStore.setOnboardingStep("local_ai")}
      >
        <span>Continue</span>
        <ChevronRight size={16} aria-hidden="true" />
      </button>
    </div>
  );
}

function LocalAiStep({ snapshot }: OnboardingFlowProps) {
  return (
    <div className="onboarding-step">
      <div className="onboarding-icon">
        <Brain size={24} aria-hidden="true" />
      </div>
      <div className="onboarding-copy">
        <h1>Low-spec local AI mode.</h1>
        <p>Optimized for this 8 GB laptop with conservative sync and local models.</p>
      </div>
      <div className="mode-row" role="radiogroup" aria-label="Model profile">
        <button
          className={
            snapshot.onboarding.selectedModelProfile === "low_spec"
              ? "mode-option mode-option-selected"
              : "mode-option"
          }
          type="button"
          role="radio"
          aria-checked={snapshot.onboarding.selectedModelProfile === "low_spec"}
          onClick={() => assistantStore.setOnboardingModelProfile("low_spec")}
        >
          <Brain size={15} aria-hidden="true" />
          <span>Low spec</span>
        </button>
        <button
          className={
            snapshot.onboarding.selectedModelProfile === "balanced"
              ? "mode-option mode-option-selected"
              : "mode-option"
          }
          type="button"
          role="radio"
          aria-checked={snapshot.onboarding.selectedModelProfile === "balanced"}
          onClick={() => assistantStore.setOnboardingModelProfile("balanced")}
        >
          <span>Balanced</span>
        </button>
      </div>
      <button
        className="primary-action"
        type="button"
        onClick={() => assistantStore.setOnboardingStep("vault")}
      >
        <span>Continue</span>
        <ChevronRight size={16} aria-hidden="true" />
      </button>
    </div>
  );
}

function VaultStep({ snapshot }: OnboardingFlowProps) {
  const completed = snapshot.onboarding.completed;

  return (
    <div className="onboarding-step">
      <div className="onboarding-icon">
        {completed ? <Check size={24} aria-hidden="true" /> : <HardDrive size={24} aria-hidden="true" />}
      </div>
      <div className="onboarding-copy">
        <h1>{completed ? "Vault ready." : "Choose your local vault."}</h1>
        <p>Markdown folders are created in the selected directory and remain user-owned.</p>
      </div>

      <div className="vault-picker">
        <input
          type="text"
          value={snapshot.onboardingVaultPath}
          placeholder="D:\\DEYANA Vault"
          aria-label="Vault folder path"
          onChange={(event) => assistantStore.setOnboardingVaultPath(event.target.value)}
        />
        <button
          className="icon-button"
          type="button"
          title="Choose folder"
          aria-label="Choose folder"
          onClick={() => void assistantStore.chooseVaultFolder()}
        >
          <FolderOpen size={17} aria-hidden="true" />
        </button>
      </div>

      <div className="vault-folder-grid" aria-label="Vault folders">
        {vaultFolders.map((folder) => (
          <span key={folder}>{folder}</span>
        ))}
      </div>

      <button
        className="primary-action"
        type="button"
        disabled={snapshot.onboardingBusy || completed}
        onClick={() => void assistantStore.completeOnboarding()}
      >
        <span>{completed ? "Completed" : snapshot.onboardingBusy ? "Creating vault" : "Create vault and start"}</span>
        <Check size={16} aria-hidden="true" />
      </button>
    </div>
  );
}
