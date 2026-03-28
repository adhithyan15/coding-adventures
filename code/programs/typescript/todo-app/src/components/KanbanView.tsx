/**
 * KanbanView.tsx — Kanban board renderer (V1 stub).
 *
 * In V1, the Kanban view shows a "coming soon" placeholder. The data model
 * (KanbanViewConfig with groupByField) is complete and ready — we just haven't
 * built the column UI yet.
 *
 * This stub allows the "Board" view tab to exist and be navigable without
 * crashing the app, while making it clear to the user that the feature is
 * on the way.
 *
 * === What full V2 will do ===
 *
 * groupByField is the key on Task that determines columns. For example:
 *   - groupByField: "status"   → columns: todo | in-progress | done
 *   - groupByField: "priority" → columns: urgent | high | medium | low
 *   - groupByField: "category" → one column per unique category value
 *
 * Tasks within each column are sorted by sortOrder (for drag-to-reorder).
 * columnOrder in the config controls the left-to-right column sequence.
 */

import type { Task } from "../types.js";
import type { KanbanViewConfig } from "../views.js";
import { t } from "../strings.js";

interface KanbanViewProps {
  tasks: Task[];
  config: KanbanViewConfig;
  onNavigate: (path: string) => void;
}

/**
 * KanbanView — renders a coming-soon placeholder.
 *
 * Props are accepted but not used yet. The component signature matches
 * the final interface so that the swap to a real implementation requires
 * no changes to the parent ViewRenderer.
 */
export function KanbanView({ tasks: _tasks, config: _config }: KanbanViewProps) {
  return (
    <div className="kanban-view kanban-view--stub" id="kanban-view">
      <div className="empty-state" id="kanban-coming-soon">
        <div className="empty-state__icon">🗂️</div>
        <h2>{t("board.comingSoonHeading")}</h2>
        <p>{t("board.comingSoonBody")}</p>
        <p className="empty-state__hint">{t("board.comingSoonHint")}</p>
      </div>
    </div>
  );
}
