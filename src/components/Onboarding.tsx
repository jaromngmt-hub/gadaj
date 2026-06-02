import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../store";

type Step = "hotkey" | "model" | "done";

export function Onboarding() {
  const { t } = useTranslation();
  const setView = useAppStore((s) => s.setView);
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const models = useAppStore((s) => s.models);
  const loadModels = useAppStore((s) => s.loadModels);
  const downloadModel = useAppStore((s) => s.downloadModel);

  const [step, setStep] = useState<Step>("hotkey");
  const [recording, setRecording] = useState(false);
  const [capturedKey, setCapturedKey] = useState<string | null>(null);

  useEffect(() => {
    if (models.length === 0) void loadModels();
  }, [models.length, loadModels]);

  useEffect(() => {
    if (step === "model" && !recording) {
      const downloaded = models.find((m) => m.id === settings.modelId);
      if (downloaded?.downloaded) setStep("done");
    }
  }, [step, models, settings.modelId, recording]);

  useEffect(() => {
    if (!recording) return;

    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const k = formatKey(e);
      setCapturedKey(k);
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording]);

  const confirmKey = async () => {
    if (!capturedKey) return;
    await setSettings({ hotkey: capturedKey });
    setRecording(false);
    setStep("model");
  };

  const skipKey = () => {
    setRecording(false);
    setStep("model");
  };

  const startDownload = async () => {
    await downloadModel(settings.modelId);
  };

  const finish = () => {
    setView("main");
  };

  return (
    <div className="flex h-full flex-col p-6 gap-6 max-w-2xl mx-auto w-full">
      <header>
        <h1 className="text-3xl font-semibold">{t("onboarding.welcome")}</h1>
        <p className="text-gadaj-500 dark:text-gadaj-400 mt-2">{t("onboarding.intro")}</p>
      </header>

      <div className="flex-1 flex flex-col gap-4">
        {step === "hotkey" && (
          <div className="card flex flex-col gap-4">
            <h2 className="text-xl font-medium">{t("onboarding.step1Title")}</h2>
            <p className="text-sm text-gadaj-500 dark:text-gadaj-400">
              {t("onboarding.step1Desc")}
            </p>

            <div className="flex items-center gap-3">
              <button
                className={recording ? "btn-primary" : "btn-secondary"}
                onClick={() => setRecording((r) => !r)}
              >
                {recording ? t("onboarding.pressAnyKey") : t("onboarding.recordKey")}
              </button>
              {capturedKey && (
                <kbd className="kbd text-lg px-3 py-2">{capturedKey}</kbd>
              )}
            </div>

            <div className="flex gap-2">
              <button
                className="btn-primary"
                disabled={!capturedKey}
                onClick={confirmKey}
              >
                {t("common.ok")}
              </button>
              <button className="btn-ghost" onClick={skipKey}>
                {t("onboarding.skipKey")}
              </button>
            </div>
          </div>
        )}

        {step === "model" && (
          <div className="card flex flex-col gap-4">
            <h2 className="text-xl font-medium">{t("onboarding.step2Title")}</h2>
            <p className="text-sm text-gadaj-500 dark:text-gadaj-400">
              {t("onboarding.step2Desc")}
            </p>

            {models
              .filter((m) => m.id === settings.modelId)
              .map((m) => (
                <div key={m.id} className="flex items-center justify-between p-3 rounded-lg border border-gadaj-200 dark:border-gadaj-800">
                  <div>
                    <p className="font-medium">{m.name}</p>
                    <p className="text-xs text-gadaj-500">{m.description}</p>
                  </div>
                  {m.downloaded ? (
                    <span className="text-green-600 dark:text-green-400 text-sm">✓ {t("settings.downloaded")}</span>
                  ) : m.downloading ? (
                    <span className="text-sm">{t("onboarding.downloading")} {m.progress}%</span>
                  ) : (
                    <button className="btn-primary" onClick={startDownload}>
                      {t("onboarding.downloadModel")}
                    </button>
                  )}
                </div>
              ))}

            <div className="flex gap-2">
              <button
                className="btn-primary"
                disabled={!models.find((m) => m.id === settings.modelId)?.downloaded}
                onClick={() => setStep("done")}
              >
                {t("common.next")}
              </button>
            </div>
          </div>
        )}

        {step === "done" && (
          <div className="card flex flex-col gap-4">
            <h2 className="text-xl font-medium">{t("onboarding.step3Title")}</h2>
            <p className="text-sm text-gadaj-500 dark:text-gadaj-400">
              {t("onboarding.step3Desc")}
            </p>
            <button className="btn-primary" onClick={finish}>
              {t("onboarding.finish")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

function formatKey(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.metaKey || e.ctrlKey) parts.push("Cmd/Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  const k = e.key;
  if (k !== "Meta" && k !== "Control" && k !== "Alt" && k !== "Shift") {
    parts.push(k.length === 1 ? k.toUpperCase() : k);
  }
  return parts.join("+") || k;
}
