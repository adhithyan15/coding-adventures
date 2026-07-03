import { useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { stateLoadAction } from "../actions.js";
import {
  createEngramSnapshot,
  parseEngramSnapshot,
} from "../snapshot.js";
import { store } from "../state.js";
import type { AppState } from "../types.js";

interface CollectionToolsProps {
  state: AppState;
}

export function CollectionTools({ state }: CollectionToolsProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [status, setStatus] = useState<string>("");

  function handleExport() {
    const snapshot = createEngramSnapshot(state);
    const blob = new Blob([JSON.stringify(snapshot, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `engram-backup-${new Date(snapshot.exportedAt)
      .toISOString()
      .slice(0, 10)}.json`;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
    setStatus("Backup exported.");
  }

  async function handleImport(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;

    try {
      const restored = parseEngramSnapshot(await file.text());
      const totalCards = restored.cards.length;
      const confirmed = window.confirm(
        `Restore this Engram backup with ${restored.decks.length} deck${
          restored.decks.length === 1 ? "" : "s"
        } and ${totalCards} card${
          totalCards === 1 ? "" : "s"
        }? This replaces the current local collection.`,
      );
      if (!confirmed) return;

      store.dispatch(
        stateLoadAction(
          restored.decks,
          restored.cards,
          restored.cardProgress,
          restored.sessions,
          restored.reviews,
          true,
        ),
      );
      setStatus("Backup restored.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Import failed.");
    }
  }

  return (
    <div className="collection-tools">
      <div className="collection-tools__actions">
        <button type="button" className="btn--secondary" onClick={handleExport}>
          Export
        </button>
        <button
          type="button"
          className="btn--secondary"
          onClick={() => inputRef.current?.click()}
        >
          Import
        </button>
      </div>
      <input
        ref={inputRef}
        className="collection-tools__file"
        type="file"
        accept="application/json,.json"
        onChange={handleImport}
      />
      {status && <p className="collection-tools__status">{status}</p>}
    </div>
  );
}
