import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore, type PasteMethod } from "../store";

interface SettingsProps {
  onBack: () => void;
}

export function Settings({ onBack }: SettingsProps) {
  const { t, i18n } = useTranslation();
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const models = useAppStore((s) => s.models);
  const loadModels = useAppStore((s) => s.loadModels);
  const downloadModel = useAppStore((s) => s.downloadModel);
  const deleteModel = useAppStore((s) => s.deleteModel);

  const [hotkeyDraft, setHotkeyDraft] = useState(settings.hotkey ?? "");
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (models.length === 0) void loadModels();
  }, [models.length, loadModels]);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const parts: string[] = [];
      if (e.metaKey || e.ctrlKey) parts.push("Cmd/Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");
      const k = e.key;
      if (k !== "Meta" && k !== "Control" && k !== "Alt" && k !== "Shift") {
        parts.push(k.length === 1 ? k.toUpperCase() : k);
      }
      setHotkeyDraft(parts.join("+") || k);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording]);

  return (
    <div className="flex h-full flex-col p-6 gap-6 max-w-3xl mx-auto w-full">
      <header className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
        <button className="btn-ghost" onClick={onBack}>← {t("common.close")}</button>
      </header>

      <main className="flex-1 flex flex-col gap-4 overflow-y-auto">
        <section className="card flex flex-col gap-2">
          <h2 className="text-lg font-medium">{t("settings.hotkey")}</h2>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400">{t("settings.hotkeyDesc")}</p>
          <div className="flex gap-2 items-center">
            <input
              className="input flex-1"
              readOnly
              value={hotkeyDraft}
              placeholder={t("onboarding.pressAnyKey")}
            />
            <button
              className={recording ? "btn-primary" : "btn-secondary"}
              onClick={() => setRecording((r) => !r)}
            >
              {recording ? t("onboarding.pressAnyKey") : t("onboarding.recordKey")}
            </button>
            <button
              className="btn-primary"
              disabled={!hotkeyDraft}
              onClick={() => void setSettings({ hotkey: hotkeyDraft })}
            >
              {t("common.save")}
            </button>
          </div>
        </section>

        <section className="card flex flex-col gap-2">
          <h2 className="text-lg font-medium">{t("settings.model")}</h2>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400">{t("settings.modelDesc")}</p>
          <div className="flex flex-col gap-2">
            {models.map((m) => (
              <div key={m.id} className="flex items-center justify-between p-3 rounded-lg border border-gadaj-200 dark:border-gadaj-800">
                <div className="flex-1">
                  <p className="font-medium">{m.name}</p>
                  <p className="text-xs text-gadaj-500">{m.description}</p>
                  {m.downloading && (
                    <div className="mt-2 h-1 bg-gadaj-200 dark:bg-gadaj-800 rounded overflow-hidden">
                      <div className="h-full bg-accent-500" style={{ width: `${m.progress}%` }} />
                    </div>
                  )}
                </div>
                <div className="flex gap-2 items-center">
                  {m.downloaded ? (
                    <>
                      <span className="text-sm text-green-600 dark:text-green-400">✓</span>
                      <button className="btn-ghost text-sm" onClick={() => void deleteModel(m.id)}>🗑</button>
                    </>
                  ) : m.downloading ? (
                    <span className="text-sm">{m.progress}%</span>
                  ) : (
                    <button className="btn-primary" onClick={() => void downloadModel(m.id)}>
                      {t("settings.download")}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="card flex flex-col gap-2">
          <h2 className="text-lg font-medium">{t("settings.language")}</h2>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400">{t("settings.languageDesc")}</p>
          <select
            className="input"
            value={settings.language}
            onChange={(e) => void setSettings({ language: e.target.value })}
          >
            <option value="auto">{t("settings.languageAuto")}</option>
            <option value="pl">Polski</option>
            <option value="en">English</option>
          </select>
        </section>

        <section className="card flex flex-col gap-2">
          <h2 className="text-lg font-medium">{t("settings.pasteMethod")}</h2>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400">{t("settings.pasteMethodDesc")}</p>
          <select
            className="input"
            value={settings.pasteMethod}
            onChange={(e) => void setSettings({ pasteMethod: e.target.value as PasteMethod })}
          >
            <option value="auto">{t("settings.pasteAuto")}</option>
            <option value="clipboard">{t("settings.pasteClipboard")}</option>
          </select>
        </section>

        <section className="card flex flex-col gap-2">
          <h2 className="text-lg font-medium">{t("settings.ui")}</h2>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400">{t("settings.uiDesc")}</p>
          <select
            className="input"
            value={i18n.language}
            onChange={(e) => void setSettings({ uiLanguage: e.target.value as "pl" | "en" })}
          >
            <option value="pl">Polski</option>
            <option value="en">English</option>
          </select>
        </section>
      </main>
    </div>
  );
}
