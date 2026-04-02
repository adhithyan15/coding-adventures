/**
 * EntryEditor.tsx — Split-pane markdown editor with live preview.
 *
 * Used for both creating new entries (#/entry/new) and editing existing
 * ones (#/entry/:id/edit). When entryId is provided, the editor loads
 * the existing entry's title and content.
 *
 * === Split-pane layout ===
 *
 * Left pane:  <textarea> for raw GFM markdown input (monospace font)
 * Right pane: rendered HTML preview (updated live as the user types)
 *
 * === Debounced preview ===
 *
 * The preview re-renders on every keystroke, debounced at 150ms. This
 * means continuous typing triggers at most ~6.7 re-renders per second —
 * fast enough to feel instant, slow enough to avoid jank on complex
 * documents.
 */

import { useState, useEffect, useRef } from "react";
import { useStore } from "@coding-adventures/store";
import { useTranslation } from "@coding-adventures/ui-components";
import { store } from "../state.js";
import { entryCreateAction, entryUpdateAction } from "../actions.js";
import { renderPreview } from "../preview.js";
import { navigate } from "../App.js";

const DEBOUNCE_MS = 150;

interface EntryEditorProps {
  entryId?: string;
}

export function EntryEditor({ entryId }: EntryEditorProps): JSX.Element {
  const state = useStore(store);
  const { t } = useTranslation();

  // Find the existing entry if editing
  const existingEntry = entryId
    ? state.entries.find((e) => e.id === entryId)
    : undefined;

  const [title, setTitle] = useState(existingEntry?.title ?? "");
  const [content, setContent] = useState(existingEntry?.content ?? "");
  const [previewHtml, setPreviewHtml] = useState(() =>
    renderPreview(existingEntry?.content ?? ""),
  );
  const debounceRef = useRef<number | null>(null);

  // Update preview when content changes (debounced)
  useEffect(() => {
    if (debounceRef.current !== null) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(() => {
      setPreviewHtml(renderPreview(content));
    }, DEBOUNCE_MS);
    return () => {
      if (debounceRef.current !== null) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [content]);

  function handleSave() {
    if (!title.trim()) return;
    if (existingEntry) {
      store.dispatch(entryUpdateAction(existingEntry.id, title, content));
      navigate(`/entry/${existingEntry.id}`);
    } else {
      store.dispatch(entryCreateAction(title, content));
      navigate("/");
    }
  }

  function handleCancel() {
    if (existingEntry) {
      navigate(`/entry/${existingEntry.id}`);
    } else {
      navigate("/");
    }
  }

  // Show not-found message if editing a non-existent entry
  if (entryId && !existingEntry) {
    return (
      <div className="entry-editor">
        <p>{t("entry.notFound")}</p>
        <button onClick={() => navigate("/")} type="button">
          {t("entry.back")}
        </button>
      </div>
    );
  }

  const isNew = !existingEntry;
  const heading = isNew ? t("entry.new") : t("entry.edit");

  return (
    <div className="entry-editor">
      <header className="entry-editor__header">
        <button
          className="entry-editor__back"
          onClick={handleCancel}
          type="button"
        >
          {t("entry.back")}
        </button>
        <h1>{heading}</h1>
        <button
          className="entry-editor__save"
          onClick={handleSave}
          disabled={!title.trim()}
          type="button"
        >
          {t("entry.save")}
        </button>
      </header>

      <div className="entry-editor__title-row">
        <input
          className="entry-editor__title-input"
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder={t("entry.titlePlaceholder")}
          aria-label={t("entry.titlePlaceholder")}
        />
      </div>

      <div className="entry-editor__split-pane">
        <textarea
          className="entry-editor__textarea"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder={t("entry.contentPlaceholder")}
          aria-label={t("entry.contentPlaceholder")}
        />
        <div
          className="entry-editor__preview preview"
          dangerouslySetInnerHTML={{ __html: previewHtml }}
        />
      </div>
    </div>
  );
}
