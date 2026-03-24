/**
 * TemplateEditor — Screen 2: create or edit a checklist template.
 *
 * V0.2 uses the shared TreeView and BranchGroup components to render the
 * item tree with CSS connectors — "what you build is what you run."
 *
 * V0.3 replaces direct state mutations with store.dispatch(action).
 * The editor still maintains local draft state (DraftItem[]) until Save
 * is pressed. Only then does it dispatch a TEMPLATE_CREATE or TEMPLATE_UPDATE
 * action. Cancel discards all changes (no action dispatched).
 *
 * Decision items show two BranchGroups (yes/no), each containing a nested
 * TreeView of its branch items. "Add step" buttons appear at the end of
 * each branch.
 */

import { useState } from "react";
import {
  TreeView,
  BranchGroup,
  useTranslation,
} from "@coding-adventures/ui-components";
import { useStore } from "@coding-adventures/store";
import { store } from "../state.js";
import {
  createTemplateAction,
  updateTemplateAction,
} from "../actions.js";
import type { TemplateItem } from "../types.js";

interface TemplateEditorProps {
  templateId?: string;
  onNavigate: (path: string) => void;
}

// ── Draft types ────────────────────────────────────────────────────────────

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

function toTemplateItems(drafts: DraftItem[]): TemplateItem[] {
  return drafts.map((draft): TemplateItem => {
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
  });
}

// ── TreeView node adapter ──────────────────────────────────────────────────

interface DraftTreeNode {
  id: string;
  draft: DraftItem;
  children?: DraftTreeNode[];
}

function toDraftTreeNodes(items: DraftItem[]): DraftTreeNode[] {
  return items.map((item) => ({
    id: item.draftId,
    draft: item,
  }));
}

// ── Recursive editor item list ─────────────────────────────────────────────

interface EditorItemListProps {
  items: DraftItem[];
  depth: number;
  onChange: (updated: DraftItem[]) => void;
}

function EditorItemList({ items, depth, onChange }: EditorItemListProps) {
  const { t } = useTranslation();
  const nodes = toDraftTreeNodes(items);

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

  function addItem() {
    onChange([...items, newCheckItem()]);
  }

  function updateBranch(
    itemIdx: number,
    branch: "yesBranch" | "noBranch",
    updated: DraftItem[],
  ) {
    const item = items[itemIdx];
    if (item?.type === "decision") {
      updateItem(itemIdx, { ...item, [branch]: updated });
    }
  }

  return (
    <div>
      <TreeView
        nodes={nodes}
        renderNode={(treeNode) => {
          const idx = items.findIndex((i) => i.draftId === treeNode.draft.draftId);
          const item = treeNode.draft;

          return (
            <div>
              <ItemEditorRow
                item={item}
                index={idx}
                total={items.length}
                onChange={(updated) => updateItem(idx, updated)}
                onRemove={() => removeItem(idx)}
                onMoveUp={() => moveItem(idx, -1)}
                onMoveDown={() => moveItem(idx, 1)}
              />
              {item.type === "decision" && (
                <div style={{ marginTop: "8px" }}>
                  <BranchGroup
                    label={
                      <span className="item-editor__branch-label--yes">
                        {t("branch.yes")}
                      </span>
                    }
                    collapsed={false}
                    inactive={false}
                  >
                    <EditorItemList
                      items={item.yesBranch}
                      depth={depth + 1}
                      onChange={(updated) =>
                        updateBranch(idx, "yesBranch", updated)
                      }
                    />
                  </BranchGroup>
                  <BranchGroup
                    label={
                      <span className="item-editor__branch-label--no">
                        {t("branch.no")}
                      </span>
                    }
                    collapsed={false}
                    inactive={false}
                  >
                    <EditorItemList
                      items={item.noBranch}
                      depth={depth + 1}
                      onChange={(updated) =>
                        updateBranch(idx, "noBranch", updated)
                      }
                    />
                  </BranchGroup>
                </div>
              )}
            </div>
          );
        }}
        ariaLabel="Checklist editor"
      />
      <button
        className="btn--ghost"
        onClick={addItem}
        type="button"
        style={{ alignSelf: "flex-start", marginTop: "8px" }}
      >
        + {depth === 0 ? t("editor.addItem") : t("editor.addBranchItem")}
      </button>
    </div>
  );
}

// ── ItemEditorRow ──────────────────────────────────────────────────────────

interface ItemEditorRowProps {
  item: DraftItem;
  index: number;
  total: number;
  onChange: (updated: DraftItem) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

function ItemEditorRow({
  item,
  index,
  total,
  onChange,
  onRemove,
  onMoveUp,
  onMoveDown,
}: ItemEditorRowProps) {
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

  return (
    <div
      className={`item-editor${item.type === "decision" ? " item-editor--decision" : ""}`}
    >
      <div className="item-editor__row">
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
    </div>
  );
}

// ── TemplateEditor ─────────────────────────────────────────────────────────

export function TemplateEditor({ templateId, onNavigate }: TemplateEditorProps) {
  const { t } = useTranslation();
  const state = useStore(store);
  const existing = templateId
    ? state.templates.find((tpl) => tpl.id === templateId)
    : undefined;

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
      store.dispatch(
        updateTemplateAction(existing.id, {
          name: name.trim(),
          description: description.trim(),
          items: templateItems,
        }),
      );
    } else {
      store.dispatch(
        createTemplateAction(name.trim(), description.trim(), templateItems),
      );
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

      <EditorItemList items={items} depth={0} onChange={setItems} />

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
