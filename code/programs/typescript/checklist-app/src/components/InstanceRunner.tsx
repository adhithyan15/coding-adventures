/**
 * InstanceRunner — Screen 3: execute a checklist instance.
 *
 * This is the core interaction screen. It renders the flattened visible item
 * list and lets the user check items and answer decisions.
 *
 * Key design decisions:
 *
 *   - flattenVisibleItems is called on every render. It is a pure tree walk
 *     with no memoization needed for V0 (lists are short).
 *
 *   - Checking an item or answering a decision calls the state mutation, then
 *     calls forceUpdate to re-render. React re-runs flattenVisibleItems and
 *     the new visible list reflects the answered decision.
 *
 *   - "Complete" is enabled only when all visible check items are checked AND
 *     all visible decision items are answered. This mirrors the aviation rule:
 *     you cannot complete the checklist until every item is addressed.
 *
 *   - Decision items can be re-answered. Clicking the answer badge opens the
 *     Yes / No buttons again. This is intentional: pilots can correct a
 *     premature answer.
 */

import { useState } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import {
  appState,
  checkItem,
  uncheckItem,
  answerDecision,
  completeInstance,
  abandonInstance,
  flattenVisibleItems,
  getInstance,
} from "../state.js";
import type { CheckInstanceItem, DecisionInstanceItem } from "../state.js";
import { ProgressBar } from "./shared/ProgressBar.js";

interface InstanceRunnerProps {
  instanceId: string;
  onNavigate: (path: string) => void;
}

export function InstanceRunner({ instanceId, onNavigate }: InstanceRunnerProps) {
  const { t } = useTranslation();
  // forceUpdate trick: toggling a boolean causes React to re-render, which
  // re-calls flattenVisibleItems with the latest state.
  const [, setTick] = useState(0);
  // Track which decision items are in "re-answer mode" (showing yes/no buttons again).
  const [reanswering, setReanswering] = useState<Set<string>>(new Set());

  const instance = getInstance(appState, instanceId);
  if (!instance) {
    return <p>Instance not found.</p>;
  }

  const visible = flattenVisibleItems(instance.items);
  const checkItems = visible.filter((i) => i.type === "check") as CheckInstanceItem[];
  const decisionItems = visible.filter((i) => i.type === "decision") as DecisionInstanceItem[];

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

  function handleComplete() {
    completeInstance(appState, instanceId);
    onNavigate(`/instance/${instanceId}/stats`);
  }

  function handleAbandon() {
    abandonInstance(appState, instanceId);
    onNavigate(`/instance/${instanceId}/stats`);
  }

  return (
    <div>
      <div className="instance-runner__header">
        <h1 className="instance-runner__title">{instance.templateName}</h1>
        <div className="instance-runner__progress">
          <ProgressBar checked={checkedCount} total={totalCheckItems} />
        </div>
      </div>

      {visible.length === 0 ? (
        <p>{t("runner.empty")}</p>
      ) : (
        <div className="instance-runner__items">
          {visible.map((item) => {
            if (item.type === "check") {
              return (
                <CheckItemRow
                  key={item.templateItemId}
                  item={item}
                  onToggle={() =>
                    handleCheck(item.templateItemId, item.checked)
                  }
                />
              );
            } else {
              const isReanswering = reanswering.has(item.templateItemId);
              return (
                <DecisionItemRow
                  key={item.templateItemId}
                  item={item}
                  isReanswering={isReanswering}
                  onAnswer={(a) => handleAnswer(item.templateItemId, a)}
                  onReanswer={() => handleReanswer(item.templateItemId)}
                />
              );
            }
          })}
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
