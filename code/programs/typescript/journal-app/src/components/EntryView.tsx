/**
 * EntryView.tsx — Read-only rendered view of a single journal entry.
 *
 * Displays the entry's title as a heading and the content as rendered
 * HTML via the GFM pipeline. Provides Edit and Delete actions.
 */

import { useStore } from "@coding-adventures/store";
import { useTranslation } from "@coding-adventures/ui-components";
import { store } from "../state.js";
import { entryDeleteAction } from "../actions.js";
import { renderPreview } from "../preview.js";
import { navigate } from "../App.js";

interface EntryViewProps {
  entryId: string;
}

/**
 * formatDate — format an ISO 8601 date string as a human-readable date.
 *
 * Example: "2026-04-02" → "April 2, 2026"
 */
function formatDate(dateStr: string): string {
  const date = new Date(dateStr + "T00:00:00Z");
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  });
}

export function EntryView({ entryId }: EntryViewProps): JSX.Element {
  const state = useStore(store);
  const { t } = useTranslation();
  const entry = state.entries.find((e) => e.id === entryId);

  if (!entry) {
    return (
      <div className="entry-view">
        <p>{t("entry.notFound")}</p>
        <button onClick={() => navigate("/")} type="button">
          {t("entry.back")}
        </button>
      </div>
    );
  }

  function handleDelete() {
    if (window.confirm(t("entry.confirmDelete"))) {
      store.dispatch(entryDeleteAction(entryId));
      navigate("/");
    }
  }

  const html = renderPreview(entry.content);

  return (
    <div className="entry-view">
      <header className="entry-view__header">
        <button
          className="entry-view__back"
          onClick={() => navigate("/")}
          type="button"
        >
          {t("entry.back")}
        </button>
        <span className="entry-view__date">{formatDate(entry.createdAt)}</span>
        <div className="entry-view__actions">
          <button
            className="entry-view__edit"
            onClick={() => navigate(`/entry/${entryId}/edit`)}
            type="button"
          >
            {t("entry.editButton")}
          </button>
          <button
            className="entry-view__delete"
            onClick={handleDelete}
            type="button"
          >
            {t("entry.delete")}
          </button>
        </div>
      </header>

      <h1 className="entry-view__title">{entry.title}</h1>

      <div
        className="entry-view__content preview"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  );
}
