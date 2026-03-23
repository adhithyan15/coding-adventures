/**
 * InstanceRunner — Screen 3: execute a checklist instance with tree view.
 *
 * V0.2 replaces the flat list with a recursive tree view via the shared
 * TreeView and BranchGroup components from @coding-adventures/ui-components.
 *
 * The tree renders the full item structure with CSS connectors. Decision
 * nodes show two BranchGroups (yes/no). The active branch is fully
 * interactive; the inactive branch is dimmed and collapsed to a summary
 * line (expandable on click for review).
 *
 * Key changes from V0.1:
 *   - flattenVisibleItems is no longer used for rendering (still used by
 *     computeStats for the progress bar and stats calculations)
 *   - The recursive tree structure is rendered directly, showing both
 *     branches of every decision at all times
 *   - Inactive branches show "N steps • click to expand" and can be
 *     expanded for read-only review
 */

import { useState, useCallback } from "react";
import {
  TreeView,
  BranchGroup,
  useTranslation,
} from "@coding-adventures/ui-components";
import {
  appState,
  checkItem,
  uncheckItem,
  answerDecision,
  completeInstance,
  abandonInstance,
  flattenVisibleItems,
  countBranchItems,
  getInstance,
} from "../state.js";
import type {
  InstanceItem,
  CheckInstanceItem,
  DecisionInstanceItem,
} from "../state.js";
import { ProgressBar } from "./shared/ProgressBar.js";

interface InstanceRunnerProps {
  instanceId: string;
  onNavigate: (path: string) => void;
}

// ── Adapter: InstanceItem → TreeViewNode ───────────────────────────────────
//
// The TreeView expects nodes with `id` and `children`. InstanceItems have
// `templateItemId` and decision items have `yesBranch`/`noBranch`. We don't
// restructure the data — instead we use `renderNode` to handle the branching
// logic inline, and pass an empty `children` array to prevent TreeView from
// recursing (we manage the branch recursion ourselves via BranchGroups).

interface InstanceTreeNode {
  id: string;
  item: InstanceItem;
  children?: InstanceTreeNode[];
}

function toTreeNodes(items: InstanceItem[]): InstanceTreeNode[] {
  return items.map((item) => ({
    id: item.templateItemId,
    item,
    // We handle decision branches in renderNode, not via TreeView recursion
  }));
}

// ── InstanceRunner ─────────────────────────────────────────────────────────

export function InstanceRunner({ instanceId, onNavigate }: InstanceRunnerProps) {
  const { t } = useTranslation();
  const [, setTick] = useState(0);
  // Track which inactive branches the user has manually expanded for review
  const [expandedInactive, setExpandedInactive] = useState<Set<string>>(
    new Set(),
  );
  // Track which decisions are in re-answer mode
  const [reanswering, setReanswering] = useState<Set<string>>(new Set());

  const instance = getInstance(appState, instanceId);
  if (!instance) {
    return <p>Instance not found.</p>;
  }

  // Stats for progress bar (still uses flattenVisibleItems)
  const visible = flattenVisibleItems(instance.items);
  const checkItems = visible.filter(
    (i) => i.type === "check",
  ) as CheckInstanceItem[];
  const decisionItems = visible.filter(
    (i) => i.type === "decision",
  ) as DecisionInstanceItem[];
  const totalCheckItems = checkItems.length;
  const checkedCount = checkItems.filter((i) => i.checked).length;
  const allDecisionsAnswered = decisionItems.every((i) => i.answer !== null);
  const allChecked = checkedCount === totalCheckItems;
  const canComplete =
    (totalCheckItems === 0 || allChecked) && allDecisionsAnswered;

  function refresh() {
    setTick((n) => n + 1);
  }

  function handleCheck(templateItemId: string, currentlyChecked: boolean) {
    if (currentlyChecked) {
      uncheckItem(appState, instanceId, templateItemId);
    } else {
      checkItem(appState, instanceId, templateItemId);
    }
    refresh();
  }

  function handleAnswer(templateItemId: string, answer: "yes" | "no") {
    answerDecision(appState, instanceId, templateItemId, answer);
    setReanswering((prev) => {
      const next = new Set(prev);
      next.delete(templateItemId);
      return next;
    });
    refresh();
  }

  function handleReanswer(templateItemId: string) {
    setReanswering((prev) => new Set(prev).add(templateItemId));
  }

  function toggleInactiveBranch(key: string) {
    setExpandedInactive((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  function handleComplete() {
    completeInstance(appState, instanceId);
    onNavigate(`/instance/${instanceId}/stats`);
  }

  function handleAbandon() {
    abandonInstance(appState, instanceId);
    onNavigate(`/instance/${instanceId}/stats`);
  }

  function branchSummary(items: InstanceItem[]): string {
    const { checks, decisions } = countBranchItems(items);
    const summary =
      decisions > 0
        ? t("branch.summaryWithDecisions")
            .replace("{checks}", String(checks))
            .replace("{decisions}", String(decisions))
        : t("branch.summary").replace("{checks}", String(checks));
    return `${summary} • ${t("branch.clickToExpand")}`;
  }

  // ── Recursive item renderer ──────────────────────────────────────────────

  const renderItems = useCallback(
    (items: InstanceItem[]): React.ReactNode => {
      const nodes = toTreeNodes(items);
      return (
        <TreeView
          nodes={nodes}
          renderNode={(treeNode) => {
            const item = treeNode.item;
            if (item.type === "check") {
              return (
                <CheckItemRow
                  item={item}
                  onToggle={() =>
                    handleCheck(item.templateItemId, item.checked)
                  }
                />
              );
            }

            // Decision item: render question + two branch groups
            const isReanswering = reanswering.has(item.templateItemId);
            const yesActive = item.answer === "yes";
            const noActive = item.answer === "no";
            const unanswered = item.answer === null;
            const yesInactiveKey = `${item.templateItemId}-yes`;
            const noInactiveKey = `${item.templateItemId}-no`;

            return (
              <div>
                <DecisionItemRow
                  item={item}
                  isReanswering={isReanswering}
                  onAnswer={(a) => handleAnswer(item.templateItemId, a)}
                  onReanswer={() => handleReanswer(item.templateItemId)}
                />
                {!unanswered && (
                  <div style={{ marginTop: "8px" }}>
                    <BranchGroup
                      label={
                        <span className="item-editor__branch-label--yes">
                          {t("branch.yes")}
                        </span>
                      }
                      collapsed={!yesActive && !expandedInactive.has(yesInactiveKey)}
                      inactive={!yesActive}
                      summary={branchSummary(item.yesBranch)}
                      onToggleCollapse={() => toggleInactiveBranch(yesInactiveKey)}
                    >
                      {renderItems(item.yesBranch)}
                    </BranchGroup>
                    <BranchGroup
                      label={
                        <span className="item-editor__branch-label--no">
                          {t("branch.no")}
                        </span>
                      }
                      collapsed={!noActive && !expandedInactive.has(noInactiveKey)}
                      inactive={!noActive}
                      summary={branchSummary(item.noBranch)}
                      onToggleCollapse={() => toggleInactiveBranch(noInactiveKey)}
                    >
                      {renderItems(item.noBranch)}
                    </BranchGroup>
                  </div>
                )}
              </div>
            );
          }}
          ariaLabel="Checklist"
        />
      );
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [instanceId, reanswering, expandedInactive],
  );

  return (
    <div>
      <div className="instance-runner__header">
        <h1 className="instance-runner__title">{instance.templateName}</h1>
        <div className="instance-runner__progress">
          <ProgressBar checked={checkedCount} total={totalCheckItems} />
        </div>
      </div>

      {instance.items.length === 0 ? (
        <p>{t("runner.empty")}</p>
      ) : (
        <div className="instance-runner__items">
          {renderItems(instance.items)}
        </div>
      )}

      <div className="instance-runner__actions">
        <button className="btn--danger" onClick={handleAbandon} type="button">
          {t("runner.abandon")}
        </button>
        <button
          className="btn--primary"
          onClick={handleComplete}
          disabled={!canComplete}
          type="button"
        >
          {t("runner.complete")}
        </button>
      </div>
    </div>
  );
}

// ── CheckItemRow ───────────────────────────────────────────────────────────

interface CheckItemRowProps {
  item: CheckInstanceItem;
  onToggle: () => void;
}

function CheckItemRow({ item, onToggle }: CheckItemRowProps) {
  return (
    <div
      className={`check-item${item.checked ? " check-item--checked" : ""}`}
      onClick={onToggle}
      role="checkbox"
      aria-checked={item.checked}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === " " || e.key === "Enter") {
          e.preventDefault();
          onToggle();
        }
      }}
    >
      <div className="check-item__checkbox" aria-hidden="true">
        {item.checked && <span className="check-item__checkmark">✓</span>}
      </div>
      <span className="check-item__label">{item.label}</span>
    </div>
  );
}

// ── DecisionItemRow ────────────────────────────────────────────────────────

interface DecisionItemRowProps {
  item: DecisionInstanceItem;
  isReanswering: boolean;
  onAnswer: (answer: "yes" | "no") => void;
  onReanswer: () => void;
}

function DecisionItemRow({
  item,
  isReanswering,
  onAnswer,
  onReanswer,
}: DecisionItemRowProps) {
  const { t } = useTranslation();
  const showButtons = item.answer === null || isReanswering;

  return (
    <div className="decision-item">
      <p className="decision-item__question">{item.label}</p>
      {showButtons ? (
        <div className="decision-item__buttons">
          <button
            className="btn--yes"
            onClick={() => onAnswer("yes")}
            type="button"
          >
            {t("runner.yes")}
          </button>
          <button
            className="btn--no"
            onClick={() => onAnswer("no")}
            type="button"
          >
            {t("runner.no")}
          </button>
        </div>
      ) : (
        <div className="decision-item__answered">
          <span
            className={`decision-item__answer-badge decision-item__answer-badge--${item.answer}`}
          >
            {item.answer === "yes" ? t("runner.yes") : t("runner.no")}
          </span>
          <button
            className="btn--ghost"
            onClick={onReanswer}
            type="button"
          >
            {t("runner.changeAnswer")}
          </button>
        </div>
      )}
    </div>
  );
}
