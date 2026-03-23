/**
 * StatsView — Screen 4: summary of a completed or abandoned instance.
 *
 * Shows the completion rate as a large number, a grid of detail stats,
 * and action buttons to run again or return to the library.
 *
 * V0.3: Uses useStore(store) for reactive state and store.dispatch for
 * mutations. The "Run Again" button dispatches INSTANCE_CREATE and reads
 * the new instance from store.getState() to navigate to it.
 *
 * formatDuration converts milliseconds to a human-readable string:
 *   < 60 000 ms  ->  "42s"
 *   >= 60 000 ms ->  "3m 22s"
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { useStore } from "@coding-adventures/store";
import { store, computeStats } from "../state.js";
import { createInstanceAction } from "../actions.js";

interface StatsViewProps {
  instanceId: string;
  onNavigate: (path: string) => void;
}

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes === 0) return `${seconds}s`;
  return `${minutes}m ${seconds}s`;
}

export function StatsView({ instanceId, onNavigate }: StatsViewProps) {
  const { t } = useTranslation();
  const state = useStore(store);
  const instance = state.instances.find((i) => i.id === instanceId);

  if (!instance) {
    return <p>Instance not found.</p>;
  }

  const stats = computeStats(instance);
  const isComplete = instance.status === "completed";
  const rateLabel = `${Math.round(stats.completionRate)}%`;

  function handleRunAgain() {
    store.dispatch(createInstanceAction(instance!.templateId));
    const newState = store.getState();
    const newInstance = newState.instances[newState.instances.length - 1];
    if (newInstance) {
      onNavigate(`/instance/${newInstance.id}`);
    }
  }

  return (
    <div className="stats-view">
      <div className="stats-view__header">
        <p>
          <span
            className={`status-badge status-badge--${instance.status}`}
          >
            {t(`status.${instance.status === "in-progress" ? "inProgress" : instance.status}`)}
          </span>
        </p>
        <h1 className="stats-view__title">{instance.templateName}</h1>
        <p
          className={`stats-view__rate${isComplete && stats.completionRate === 100 ? " stats-view__rate--complete" : ""}`}
          aria-label={`Completion rate: ${rateLabel}`}
        >
          {rateLabel}
        </p>
      </div>

      <div className="stats-view__grid">
        <div className="stat-card">
          <p className="stat-card__value">{stats.totalItems}</p>
          <p className="stat-card__label">{t("stats.totalItems")}</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{stats.checkedItems}</p>
          <p className="stat-card__label">{t("stats.checkedItems")}</p>
        </div>
        <div className="stat-card">
          <p className="stat-card__value">{stats.decisionCount}</p>
          <p className="stat-card__label">{t("stats.decisions")}</p>
        </div>
        {stats.durationMs !== null && (
          <div className="stat-card">
            <p className="stat-card__value">{formatDuration(stats.durationMs)}</p>
            <p className="stat-card__label">{t("stats.duration")}</p>
          </div>
        )}
      </div>

      <div className="stats-view__actions">
        <button
          className="btn--secondary"
          onClick={() => onNavigate("/")}
          type="button"
        >
          {t("stats.backToLibrary")}
        </button>
        <button
          className="btn--primary"
          onClick={handleRunAgain}
          type="button"
        >
          {t("stats.runAgain")}
        </button>
      </div>
    </div>
  );
}
