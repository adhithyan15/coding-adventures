/**
 * Timeline.tsx — Home screen showing entries grouped by date.
 *
 * Entries are grouped by their createdAt date string and displayed in
 * reverse chronological order (newest date first). Within each date,
 * entries are sorted by updatedAt descending (most recently edited first).
 *
 * === Date grouping algorithm ===
 *
 * Since createdAt is an ISO 8601 date string ("YYYY-MM-DD"), grouping
 * is a simple Map lookup. No timezone conversion needed — the date is
 * already a calendar date, not a timestamp.
 */

import { useStore } from "@coding-adventures/store";
import { useTranslation } from "@coding-adventures/ui-components";
import { store } from "../state.js";
import { navigate } from "../App.js";
import { EntryCard } from "./EntryCard.js";
import type { Entry } from "../types.js";

/**
 * groupByDate — group entries by their createdAt date string.
 *
 * Returns a Map where keys are date strings ("YYYY-MM-DD") and values
 * are arrays of entries for that date.
 */
function groupByDate(entries: Entry[]): Map<string, Entry[]> {
  const groups = new Map<string, Entry[]>();
  for (const entry of entries) {
    const existing = groups.get(entry.createdAt) ?? [];
    existing.push(entry);
    groups.set(entry.createdAt, existing);
  }
  return groups;
}

/**
 * formatDateHeading — format a "YYYY-MM-DD" string as a human-readable
 * date heading.
 *
 * Example: "2026-04-02" → "April 2, 2026"
 */
function formatDateHeading(dateStr: string): string {
  // Parse as UTC to avoid timezone shift (adding T00:00:00Z prevents
  // the Date constructor from interpreting YYYY-MM-DD as local time,
  // which can shift the date by one day near midnight).
  const date = new Date(dateStr + "T00:00:00Z");
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  });
}

export function Timeline(): JSX.Element {
  const state = useStore(store);
  const { t } = useTranslation();

  const groups = groupByDate(state.entries);

  // Sort dates descending (newest first)
  const sortedDates = Array.from(groups.keys()).sort((a, b) =>
    b.localeCompare(a),
  );

  return (
    <div className="timeline">
      <header className="timeline__header">
        <h1>{t("timeline.title")}</h1>
        <button
          className="timeline__new-button"
          onClick={() => navigate("/entry/new")}
          type="button"
        >
          {t("timeline.newEntry")}
        </button>
      </header>

      {sortedDates.length === 0 && (
        <p className="timeline__empty">{t("timeline.empty")}</p>
      )}

      {sortedDates.map((date) => {
        const entries = groups.get(date)!;
        // Sort entries within date by updatedAt descending
        const sorted = [...entries].sort(
          (a, b) => b.updatedAt - a.updatedAt,
        );
        return (
          <section key={date} className="timeline__date-group">
            <h2 className="timeline__date-heading">
              {formatDateHeading(date)}
            </h2>
            <div className="timeline__entries">
              {sorted.map((entry) => (
                <EntryCard key={entry.id} entry={entry} />
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}
