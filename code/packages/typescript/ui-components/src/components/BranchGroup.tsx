/**
 * BranchGroup — collapsible wrapper for a group of child nodes.
 *
 * Used inside tree views to wrap decision branches (e.g., "If yes:" / "If no:").
 * Handles three visual states:
 *
 *   1. **Active** — children visible at full opacity, fully interactive.
 *   2. **Inactive + collapsed** — children hidden, a one-line summary shown
 *      (e.g., "3 steps • click to expand"). Dimmed to 40% opacity.
 *   3. **Inactive + expanded** — children visible but dimmed and non-interactive.
 *      The user clicked the summary to review the unchosen path.
 *
 * The collapse/expand transition uses `max-height` + `opacity` for a smooth
 * animation that works without knowing the exact content height.
 *
 * @example
 * ```tsx
 * <BranchGroup
 *   label="If yes:"
 *   collapsed={!showYesBranch}
 *   inactive={decision.answer === "no"}
 *   summary="3 steps • click to expand"
 *   onToggleCollapse={() => setShowYesBranch(!showYesBranch)}
 * >
 *   {yesBranchItems.map(item => <TreeNode ... />)}
 * </BranchGroup>
 * ```
 */

import type { ReactNode } from "react";

export interface BranchGroupProps {
  /** Label rendered above the group (e.g., "If yes:", "If no:"). */
  label: ReactNode;

  /** When true, children are hidden and the summary line is shown. */
  collapsed: boolean;

  /** When true, the group is dimmed (40% opacity, pointer-events: none). */
  inactive: boolean;

  /** Text shown when collapsed (e.g., "3 steps • click to expand"). */
  summary?: string;

  /** Called when the user clicks the summary to toggle collapse. */
  onToggleCollapse?: () => void;

  /** The child tree nodes. */
  children: ReactNode;

  /** CSS class on the outermost wrapper. */
  className?: string;
}

export function BranchGroup({
  label,
  collapsed,
  inactive,
  summary,
  onToggleCollapse,
  children,
  className,
}: BranchGroupProps) {
  const stateClass = inactive
    ? collapsed
      ? "branch-group--inactive branch-group--collapsed"
      : "branch-group--inactive branch-group--expanded"
    : "branch-group--active";

  return (
    <div
      className={`branch-group ${stateClass}${className ? ` ${className}` : ""}`}
    >
      <div className="branch-group__label">{label}</div>

      {collapsed && summary ? (
        <button
          className="branch-group__summary"
          onClick={onToggleCollapse}
          type="button"
          aria-label={summary}
        >
          {summary}
        </button>
      ) : (
        <div className="branch-group__content">{children}</div>
      )}
    </div>
  );
}
