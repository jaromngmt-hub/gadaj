import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../store";

interface MainProps {
  onOpenSettings: () => void;
  onOpenHistory: () => void;
}

export function Main({ onOpenSettings, onOpenHistory }: MainProps) {
  const { t } = useTranslation();
  const state = useAppStore((s) => s.state);
  const settings = useAppStore((s) => s.settings);
  const [level, setLevel] = useState(0);

  useEffect(() => {
    let raf = 0;
    let mounted = true;

    const poll = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const l = (await invoke("get_mic_level")) as number;
        if (mounted) setLevel(l);
      } catch {
        // backend może jeszcze nie odpowiadać
      }
      raf = window.setTimeout(poll, 100);
    };

    void poll();
    return () => {
      mounted = false;
      window.clearTimeout(raf);
    };
  }, [state]);

  const statusKey = `status.${state}`;
  const hotkeyLabel = settings.hotkey ?? "—";

  return (
    <div className="flex h-full flex-col p-6 gap-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">{t("app.name")}</h1>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400">{t("app.tagline")}</p>
        </div>
        <nav className="flex gap-2">
          <button className="btn-ghost" onClick={onOpenHistory}>
            {t("nav.history")}
          </button>
          <button className="btn-ghost" onClick={onOpenSettings}>
            {t("nav.settings")}
          </button>
        </nav>
      </header>

      <main className="flex-1 flex flex-col items-center justify-center gap-8">
        <div
          className={`relative w-48 h-48 rounded-full flex items-center justify-center
                      transition-colors duration-200
                      ${state === "recording" ? "bg-red-500/20" : "bg-accent-500/10"}`}
        >
          <div
            className={`w-32 h-32 rounded-full flex items-center justify-center
                        text-4xl font-bold transition-all
                        ${state === "recording" ? "bg-red-500 text-white scale-110" : "bg-accent-500 text-white"}`}
          >
            🎙
          </div>
        </div>

        <div className="text-center">
          <p className="text-2xl font-medium">{t(statusKey)}</p>
          <p className="text-sm text-gadaj-500 dark:text-gadaj-400 mt-2">
            {t("status.pressKeyToStart")}: <kbd className="kbd">{hotkeyLabel}</kbd>
          </p>
        </div>

        {state === "recording" && (
          <div className="w-64 h-2 bg-gadaj-200 dark:bg-gadaj-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-red-500 transition-all duration-100"
              style={{ width: `${Math.min(100, level * 100)}%` }}
            />
          </div>
        )}
      </main>

      <footer className="text-center text-xs text-gaday-400 dark:text-gadaj-500">
        <kbd className="kbd">Esc</kbd> = anuluj
      </footer>
    </div>
  );
}
