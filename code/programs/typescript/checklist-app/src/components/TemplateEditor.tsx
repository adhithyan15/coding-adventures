/**
 * TemplateEditor — Screen 2: create or edit a checklist template.
 *
 * The form maintains local draft state (DraftItem[]) until Save is pressed.
 * Only then does it write to the global AppState. Cancel discards all changes.
 *
 * The item list is recursive: a DraftItem of type "decision" has two sub-lists
 * (yesBranch and noBranch), each of which can contain further DraftItems.
 * The ItemEditor component renders itself recursively to handle arbitrary depth.
 *
 * DraftItem is a working copy of TemplateItem. It carries the same shape but
 * uses a local draft ID (not the stable template ID) so items can be freely
 * added, removed, and reordered without touching the AppState.
 */

import { useState } from "react";
import { useTranslation } from "@coding-adventures/ui-components";
import {
  appState,
  createTemplate,
  getTemplate,
  updateTemplate,
} from "../state.js";
import type { TemplateItem } from "../types.js";

interface TemplateEditorProps {
  /** If provided, edit an existing template; otherwise create a new one. */
  templateId?: string;
  onNavigate: (path: string) => void;
}

// ── Draft types ────────────────────────────────────────────────────────────
// These are local-only while editing. They are converted to TemplateItems on save.

interface DraftCheckItem {
  draftId: string;
  type: "check";
  label: string;
}

interface DraftDecisionItem {
  draftId: string;
  type: "decision";
  label: string;
  yesBranch: DraftItem[];
  noBranch: DraftItem[];
}

type DraftItem = DraftCheckItem | DraftDecisionItem;

let draftCounter = 0;
function newDraftId() {
  return `draft-${++draftCounter}`;
}

function newCheckItem(): DraftCheckItem {
  return { draftId: newDraftId(), type: "check", label: "" };
}

function newDecisionItem(): DraftDecisionItem {
  return {
    draftId: newDraftId(),
    type: "decision",
    label: "",
    yesBranch: [],
    noBranch: [],
  };
}

/** Convert saved TemplateItems into DraftItems for editing. */
function toDraft(items: TemplateItem[]): DraftItem[] {
  return items.map((item): DraftItem => {
    if (item.type === "check") {
      return { draftId: newDraftId(), type: "check", label: item.label };
    } else {
      return {
        draftId: newDraftId(),
        type: "decision",
        label: item.label,
        yesBranch: toDraft(item.yesBranch),
        noBranch: toDraft(item.noBranch),
      };
    }
  });
}

/** Convert DraftItems back to TemplateItems for saving. */
function toTemplateItems(drafts: DraftItem[]): TemplateItem[] {
  return drafts.map((draft, idx): TemplateItem => {
    if (draft.type === "check") {
      return { id: draft.draftId, type: "check", label: draft.label };
    } else {
      return {
        id: draft.draftId,
        type: "decision",
        label: draft.label,
        yesBranch: toTemplateItems(draft.yesBranch),
        noBranch: toTemplateItems(draft.noBranch),
      };
    }
    void idx; // suppress unused-variable warning
  });
}

// ── ItemEditor ─────────────────────────────────────────────────────────────

interface ItemEditorProps {
  item: DraftItem;
  index: number;
  total: number;
  depth: number;
  onChange: (updated: DraftItem) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

function ItemEditor({
  item,
  index,
  total,
  depth,
  onChange,
  onRemove,
  onMoveUp,
  onMoveDown,
}: ItemEditorProps) {
  const { t } = useTranslation();

  function setLabel(label: string) {
    onChange({ ...item, label });
  }

  function switchToCheck() {
    onChange({ draftId: item.draftId, type: "check", label: item.label });
  }

  function switchToDecision() {
    onChange({
      draftId: item.draftId,
      type: "decision",
      label: item.label,
      yesBranch: [],
      noBranch: [],
    });
  }

  function updateBranch(
    branch: "yesBranch" | "noBranch",
    updated: DraftItem[],
  ) {
    if (item.type === "decision") {
      onChange({ ...item, [branch]: updated });
    }
  }

  return (
    <div
      className={`item-editor${item.type === "decision" ? " item-editor--decision" : ""}`}
    >
      <div className="item-editor__row">
        {/* Type toggle */}
        <div className="item-editor__type-toggle">
          <button
            className={`item-editor__type-btn${item.type === "check" ? " item-editor__type-btn--active" : ""}`}
            onClick={switchToCheck}
            type="button"
            title={t("editor.typeCheck")}
          >
            {t("editor.typeCheck")}
          </button>
          <button
            className={`item-editor__type-btn${item.type === "decision" ? " item-editor__type-btn--active" : ""}`}
            onClick={switchToDecision}
            type="button"
            title={t("editor.typeDecision")}
          >
            {t("editor.typeDecision")}
          </button>
        </div>

        {/* Label input */}
        <input
          type="text"
          value={item.label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder={
            item.type === "check"
              ? t("editor.itemPlaceholder")
              : t("editor.questionPlaceholder")
          }
          aria-label={
            item.type === "check"
              ? t("editor.itemPlaceholder")
              : t("editor.questionPlaceholder")
          }
        />

        {/* Controls */}
        <div className="item-editor__controls">
          <button
            className="btn--ghost"
            onClick={onMoveUp}
            disabled={index === 0}
            title={t("editor.moveUp")}
            type="button"
            aria-label={t("editor.moveUp")}
          >
            ↑
          </button>
          <button
            className="btn--ghost"
            onClick={onMoveDown}
            disabled={index === total - 1}
            title={t("editor.moveDown")}
            type="button"
            aria-label={t("editor.moveDown")}
          >
            ↓
          </button>
          <button
            className="btn--ghost"
            onClick={onRemove}
            title={t("editor.removeItem")}
            type="button"
            aria-label={t("editor.removeItem")}
          >
            ✕
          </button>
        </div>
      </div>

      {/* Decision branches (recursive) */}
      {item.type === "decision" && (
        <>
          <div className="item-editor__branch-label item-editor__branch-label--yes">
            {t("editor.yesBranch")}
          </div>
          <ItemList
            items={item.yesBranch}
            depth={depth + 1}
            onChange={(updated) => updateBranch("yesBranch", updated)}
          />
          <div className="item-editor__branch-label item-editor__branch-label--no">
            {t("editor.noBranch")}
          </div>
          <ItemList
            items={item.noBranch}
            depth={depth + 1}
            onChange={(updated) => updateBranch("noBranch", updated)}
          />
        </>
      )}
    </div>
  );
}

// ── ItemList ───────────────────────────────────────────────────────────────

interface ItemListProps {
  items: DraftItem[];
  depth: number;
  onChange: (updated: DraftItem[]) => void;
}

function ItemList({ items, depth, onChange }: ItemListProps) {
  const { t } = useTranslation();

  function addItem() {
    onChange([...items, newCheckItem()]);
  }

  function updateItem(idx: number, updated: DraftItem) {
    const next = [...items];
    next[idx] = updated;
    onChange(next);
  }

  function removeItem(idx: number) {
    onChange(items.filter((_, i) => i !== idx));
  }

  function moveItem(idx: number, direction: -1 | 1) {
    const next = [...items];
    const target = idx + direction;
    if (target < 0 || target >= next.length) return;
    [next[idx], next[target]] = [next[target]!, next[idx]!];
    onChange(next);
  }

  return (
    <div className={`item-list${depth > 0 ? " item-list--branch" : ""}`}>
      {items.map((item, idx) => (
        <ItemEditor
          key={item.draftId}
          item={item}
          index={idx}
          total={items.length}
          depth={depth}
          onChange={(updated) => updateItem(idx, updated)}
          onRemove={() => removeItem(idx)}
          onMoveUp={() => moveItem(idx, -1)}
          onMoveDown={() => moveItem(idx, 1)}
        />
      ))}
      <button
        className="btn--ghost"
        onClick={addItem}
        type="button"
        style={{ alignSelf: "flex-start", marginTop: "4px" }}
      >
        + {depth === 0 ? t("editor.addItem") : t("editor.addBranchItem")}
      </button>
    </div>
  );
}

// ── TemplateEditor ─────────────────────────────────────────────────────────

export function TemplateEditor({ templateId, onNavigate }: TemplateEditorProps) {
  const { t } = useTranslation();
  const existing = templateId ? getTemplate(appState, templateId) : undefined;

  const [name, setName] = useState(existing?.name ?? "");
  const [description, setDescription] = useState(existing?.description ?? "");
  const [items, setItems] = useState<DraftItem[]>(
    existing ? toDraft(existing.items) : [],
  );
  const [error, setError] = useState("");

  function handleSave() {
    if (!name.trim()) {
      setError(t("editor.nameRequired"));
      return;
    }
    const templateItems = toTemplateItems(items);
    if (existing) {
      updateTemplate(appState, existing.id, {
        name: name.trim(),
        description: description.trim(),
        items: templateItems,
      });
    } else {
      createTemplate(appState, name.trim(), description.trim(), templateItems);
    }
    onNavigate("/");
  }

  return (
    <div className="template-editor">
      <h1 className="app-header__title">
        {existing ? t("editor.titleEdit") : t("editor.titleNew")}
      </h1>

      <div className="template-editor__field">
        <label className="template-editor__label" htmlFor="template-name">
          Name
        </label>
        <input
          id="template-name"
          type="text"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setError("");
          }}
          placeholder={t("editor.namePlaceholder")}
        />
        {error && <p className="template-editor__error">{error}</p>}
      </div>

      <div className="template-editor__field">
        <label className="template-editor__label" htmlFor="template-desc">
          Description
        </label>
        <textarea
          id="template-desc"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder={t("editor.descriptionPlaceholder")}
          rows={2}
        />
      </div>

      <ItemList items={items} depth={0} onChange={setItems} />

      <div className="template-editor__actions">
        <button
          className="btn--secondary"
          onClick={() => onNavigate("/")}
          type="button"
        >
          {t("editor.cancel")}
        </button>
        <button className="btn--primary" onClick={handleSave} type="button">
          {t("editor.save")}
        </button>
      </div>
    </div>
  );
}
