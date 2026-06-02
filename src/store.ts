import { create } from "zustand";

export type View = "onboarding" | "main" | "settings" | "history";
export type DictationState = "idle" | "recording" | "transcribing" | "pasting" | "error";
export type PasteMethod = "auto" | "clipboard";

export interface Settings {
  hotkey: string | null;
  modelId: string;
  language: string;
  pasteMethod: PasteMethod;
  uiLanguage: "pl" | "en";
}

export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  sizeBytes: number;
  downloaded: boolean;
  downloading: boolean;
  progress: number;
}

export interface HistoryEntry {
  id: number;
  text: string;
  createdAt: string;
  audioPath: string | null;
  language: string | null;
  durationMs: number;
}

interface AppState {
  view: View;
  setView: (view: View) => void;

  state: DictationState;
  setState: (state: DictationState) => void;

  settings: Settings;
  setSettings: (settings: Partial<Settings>) => Promise<void>;
  loadSettings: () => Promise<void>;

  models: ModelInfo[];
  loadModels: () => Promise<void>;
  downloadModel: (id: string) => Promise<void>;
  deleteModel: (id: string) => Promise<void>;

  history: HistoryEntry[];
  loadHistory: (query?: string) => Promise<void>;
  deleteHistoryEntry: (id: number) => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;
}

const DEFAULT_SETTINGS: Settings = {
  hotkey: null,
  modelId: "parakeet-tdt-0.6b-v3",
  language: "auto",
  pasteMethod: "auto",
  uiLanguage: "pl",
};

export const useAppStore = create<AppState>((set, get) => ({
  view: "onboarding",
  setView: (view) => set({ view }),

  state: "idle",
  setState: (state) => set({ state }),

  settings: DEFAULT_SETTINGS,
  setSettings: async (partial) => {
    const next = { ...get().settings, ...partial };
    set({ settings: next });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("save_settings", { settings: next });
    } catch (e) {
      console.error("save_settings failed", e);
    }
  },
  loadSettings: async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const s = (await invoke("get_settings")) as Settings;
      const merged = { ...DEFAULT_SETTINGS, ...s };
      set({ settings: merged });
      if (merged.hotkey) {
        set({ view: "main" });
      }
    } catch (e) {
      console.error("get_settings failed", e);
    }
  },

  models: [],
  loadModels: async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const m = (await invoke("get_available_models")) as ModelInfo[];
      set({ models: m });
    } catch (e) {
      console.error("get_available_models failed", e);
    }
  },
  downloadModel: async (id) => {
    set({
      models: get().models.map((m) =>
        m.id === id ? { ...m, downloading: true, progress: 0 } : m,
      ),
    });
    try {
      // Nasłuchuj eventów postępu z backendu
      const { listen } = await import("@tauri-apps/api/event");
      const unlisten = await listen<{
        id: string;
        progress: number;
        downloaded_bytes: number;
        total_bytes: number;
      }>("model-download-progress", (event) => {
        if (event.payload.id !== id) return;
        const { progress, downloaded_bytes, total_bytes } = event.payload;
        set({
          models: get().models.map((m) =>
            m.id === id
              ? {
                  ...m,
                  progress,
                  sizeBytes: total_bytes || m.sizeBytes,
                  downloading: progress < 100,
                }
              : m,
          ),
        });
        console.log(
          `Download ${id}: ${progress}% (${(downloaded_bytes / 1e6).toFixed(1)}/${(total_bytes / 1e6).toFixed(1)} MB)`,
        );
      });
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("download_model", { id });
        await get().loadModels();
      } finally {
        await unlisten();
      }
    } catch (e) {
      console.error("download_model failed", e);
      set({
        models: get().models.map((m) =>
          m.id === id ? { ...m, downloading: false, progress: 0 } : m,
        ),
      });
    }
  },
  deleteModel: async (id) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("delete_model", { id });
      await get().loadModels();
    } catch (e) {
      console.error("delete_model failed", e);
    }
  },

  history: [],
  loadHistory: async (query) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const h = (await invoke("get_history_entries", { query: query ?? null })) as HistoryEntry[];
      set({ history: h });
    } catch (e) {
      console.error("get_history_entries failed", e);
    }
  },
  deleteHistoryEntry: async (id) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("delete_history_entry", { id });
      await get().loadHistory();
    } catch (e) {
      console.error("delete_history_entry failed", e);
    }
  },
  copyToClipboard: async (text) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("copy_to_clipboard", { text });
    } catch (e) {
      console.error("copy_to_clipboard failed", e);
    }
  },
}));
