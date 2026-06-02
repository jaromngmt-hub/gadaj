import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore, type HistoryEntry } from "../store";

interface HistoryProps {
  onBack: () => void;
}

export function History({ onBack }: HistoryProps) {
  const { t } = useTranslation();
  const history = useAppStore((s) => s.history);
  const loadHistory = useAppStore((s) => s.loadHistory);
  const deleteHistoryEntry = useAppStore((s) => s.deleteHistoryEntry);
  const copyToClipboard = useAppStore((s) => s.copyToClipboard);

  const [query, setQuery] = useState("");

  useEffect(() => {
    const id = window.setTimeout(() => {
      void loadHistory(query);
    }, 200);
    return () => window.clearTimeout(id);
  }, [query, loadHistory]);

  return (
    <div className="flex h-full flex-col p-6 gap-4 max-w-4xl mx-auto w-full">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-2xl font-semibold">{t("history.title")}</h1>
        <button className="btn-ghost" onClick={onBack}>← {t("common.close")}</button>
      </header>

      <input
        className="input"
        type="search"
        placeholder={t("history.search")}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />

      <main className="flex-1 overflow-y-auto flex flex-col gap-2">
        {history.length === 0 ? (
          <p className="text-center text-gadaj-500 dark:text-gadaj-400 py-12">
            {t("history.empty")}
          </p>
        ) : (
          history.map((e) => <HistoryRow key={e.id} entry={e} onDelete={deleteHistoryEntry} onCopy={copyToClipboard} />)
        )}
      </main>
    </div>
  );
}

function HistoryRow({
  entry,
  onDelete,
  onCopy,
}: {
  entry: HistoryEntry;
  onDelete: (id: number) => Promise<void>;
  onCopy: (text: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  return (
    <div className="card flex flex-col gap-2">
      <p className="text-sm whitespace-pre-wrap">{entry.text}</p>
      <div className="flex items-center justify-between text-xs text-gadaj-500 dark:text-gadaj-400">
        <span>{t("history.savedAt", { date: new Date(entry.createdAt).toLocaleString() })}</span>
        <div className="flex gap-1">
          <button className="btn-ghost text-xs" onClick={() => void onCopy(entry.text)}>📋 {t("history.copy")}</button>
          <button
            className="btn-ghost text-xs text-red-600"
            onClick={() => {
              if (confirm(t("history.deleteConfirm"))) void onDelete(entry.id);
            }}
          >
            🗑 {t("history.delete")}
          </button>
        </div>
      </div>
    </div>
  );
}
