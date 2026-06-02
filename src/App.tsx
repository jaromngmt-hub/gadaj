import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "./store";
import { Onboarding } from "./components/Onboarding";
import { Settings } from "./components/Settings";
import { History } from "./components/History";
import { Main } from "./components/Main";

function App() {
  const { i18n } = useTranslation();
  const view = useAppStore((s) => s.view);
  const setView = useAppStore((s) => s.setView);
  const settings = useAppStore((s) => s.settings);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const loadModels = useAppStore((s) => s.loadModels);
  const loadHistory = useAppStore((s) => s.loadHistory);

  useEffect(() => {
    loadSettings();
    loadModels();
    loadHistory();
  }, [loadSettings, loadModels, loadHistory]);

  useEffect(() => {
    if (settings.uiLanguage && i18n.language !== settings.uiLanguage) {
      void i18n.changeLanguage(settings.uiLanguage);
    }
  }, [settings.uiLanguage, i18n]);

  useEffect(() => {
    if (view === "main" && !settings.hotkey) {
      setView("onboarding");
    }
  }, [view, settings.hotkey, setView]);

  switch (view) {
    case "onboarding":
      return <Onboarding />;
    case "settings":
      return <Settings onBack={() => setView("main")} />;
    case "history":
      return <History onBack={() => setView("main")} />;
    case "main":
    default:
      return <Main onOpenSettings={() => setView("settings")} onOpenHistory={() => setView("history")} />;
  }
}

export default App;
