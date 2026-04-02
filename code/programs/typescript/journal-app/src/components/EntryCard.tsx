/**
 * EntryCard.tsx — A single entry card in the timeline.
 *
 * Displays the entry's title, a truncated plain-text preview of the
 * content (first ~100 characters, not rendered markdown), and the
 * time of last edit.
 *
 * Clicking the card navigates to the entry's read-only view.
 */

import type { Entry } from "../types.js";
import { navigate } from "../App.js";

const PREVIEW_LENGTH = 100;

/**
 * formatTime — format a Unix timestamp as a locale-appropriate time string.
 *
 * Examples: "8:32 AM", "14:32", depending on the user's locale.
 */
function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });
}

/**
 * truncate — return the first N characters of a string, adding ellipsis
 * if truncated.
 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength).trimEnd() + "...";
}

interface EntryCardProps {
  entry: Entry;
}

export function EntryCard({ entry }: EntryCardProps): JSX.Element {
  return (
    <button
      className="entry-card"
      onClick={() => navigate(`/entry/${entry.id}`)}
      type="button"
    >
      <span className="entry-card__title">{entry.title}</span>
      <span className="entry-card__preview">
        {truncate(entry.content, PREVIEW_LENGTH)}
      </span>
      <span className="entry-card__time">{formatTime(entry.updatedAt)}</span>
    </button>
  );
}
