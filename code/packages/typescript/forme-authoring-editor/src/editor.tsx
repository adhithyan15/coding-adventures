/**
 * Accessible, capability-free authoring controls for Forme.
 *
 * The component owns no persistence or plugin authority. Every edit becomes a
 * semantic command for the injected AuthoringSession, while plugin controls
 * are inert descriptors whose execution is delegated to a sandbox-aware host.
 */

import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { FormEvent, ReactNode } from "react";
import type {
  BlockNode,
  InlineNode,
  TableCellNode,
  TableRowNode,
} from "@coding-adventures/document-ast";
import type {
  AuthoringCommand,
  AuthoringDocument,
  AuthoringDocumentStatus,
  AuthoringSession,
} from "@coding-adventures/forme-authoring-core";

import {
  createDefaultBlock,
  insertBlock,
  moveBlock,
  removeBlock,
  replaceBlock,
} from "./blocks.js";
import type { DefaultBlockKind } from "./blocks.js";
import {
  EditorBoundaryError,
  createEditorPluginRequest,
  validateEditorContributions,
  validateThemeOptions,
} from "./plugin.js";
import type {
  EditorContribution,
  EditorPluginBridge,
  EditorSlot,
} from "./plugin.js";

export const AUTHORING_EDITOR_CSS = `
.forme-authoring-editor { display: grid; gap: 1rem; max-width: 64rem; }
.forme-authoring-editor fieldset, .forme-authoring-editor [role="group"] { display: grid; gap: .5rem; }
.forme-authoring-editor label { display: grid; gap: .25rem; }
.forme-authoring-editor button, .forme-authoring-editor input, .forme-authoring-editor select,
.forme-authoring-editor textarea { font: inherit; }
.forme-authoring-editor :focus-visible { outline: 3px solid currentColor; outline-offset: 3px; }
.forme-authoring-editor__actions { display: flex; flex-wrap: wrap; gap: .5rem; }
.forme-authoring-editor__status:empty { min-height: 1.25rem; }
`;

const BLOCK_OPTIONS: readonly { readonly value: DefaultBlockKind; readonly label: string }[] = [
  { value: "paragraph", label: "Paragraph" },
  { value: "heading", label: "Heading" },
  { value: "list", label: "List" },
  { value: "image", label: "Image" },
  { value: "code_block", label: "Code block" },
  { value: "blockquote", label: "Blockquote" },
  { value: "table", label: "Table" },
  { value: "link", label: "Link" },
] as const;

const MAX_EDITOR_TEXT_UNITS = 1_048_576;
const MAX_LIST_ITEMS = 10_000;
const MAX_TABLE_ROWS = 10_000;
const MAX_TABLE_COLUMNS = 1_000;
const MAX_TABLE_CELLS = 40_000;
const DEFAULT_PLUGIN_ACTION_TIMEOUT_MS = 10_000;

export interface AuthoringEditorProps {
  readonly session: AuthoringSession;
  readonly themeOptions: unknown;
  readonly createDocumentIdentity: () => string | Promise<string>;
  readonly pluginContributions?: unknown;
  readonly pluginBridge?: EditorPluginBridge;
  /** Deadline for one sandbox bridge activation. Defaults to ten seconds. */
  readonly pluginActionTimeoutMs?: number;
}

function formValue(form: HTMLFormElement, name: string): string {
  return String(new FormData(form).get(name) ?? "");
}

function boundedLines(value: string, maximum: number, label: string): readonly string[] {
  if (value.length > MAX_EDITOR_TEXT_UNITS) throw new EditorBoundaryError(`${label} is too large.`);
  let lines = 1;
  for (let index = 0; index < value.length; index += 1) {
    if (value.charCodeAt(index) === 0x0a && ++lines > maximum) {
      throw new EditorBoundaryError(`${label} has too many rows.`);
    }
  }
  return value.split("\n");
}

function boundedTable(value: string): { readonly rows: readonly (readonly string[])[]; readonly width: number } {
  const lines = boundedLines(value, MAX_TABLE_ROWS, "Table");
  let width = 1;
  for (const line of lines) {
    let columns = 1;
    for (let index = 0; index < line.length; index += 1) {
      if (line.charCodeAt(index) === 0x09 && ++columns > MAX_TABLE_COLUMNS) {
        throw new EditorBoundaryError("Table has too many columns.");
      }
    }
    if (columns > width) width = columns;
  }
  if (lines.length * width > MAX_TABLE_CELLS) {
    throw new EditorBoundaryError("Table has too many cells.");
  }
  return { rows: lines.map((line) => line.split("\t")), width };
}

function inlineText(nodes: readonly InlineNode[]): string {
  return nodes.map((node): string => {
    switch (node.type) {
      case "text":
      case "code_span":
      case "raw_inline": return node.value;
      case "emphasis":
      case "strong":
      case "strikethrough":
      case "link": return inlineText(node.children);
      case "image": return node.alt;
      case "autolink": return node.destination;
      case "hard_break":
      case "soft_break": return "\n";
    }
  }).join("");
}

function paragraphText(block: BlockNode | undefined): string {
  return block?.type === "paragraph" ? inlineText(block.children) : "";
}

function blockKind(block: BlockNode): { readonly kind: DefaultBlockKind | null; readonly label: string } {
  if (block.type === "paragraph" && block.children.length === 1 && block.children[0]?.type === "image") {
    return { kind: "image", label: "Image" };
  }
  if (block.type === "paragraph" && block.children.length === 1 && block.children[0]?.type === "link") {
    return { kind: "link", label: "Link" };
  }
  switch (block.type) {
    case "paragraph": return { kind: "paragraph", label: "Paragraph" };
    case "heading": return { kind: "heading", label: "Heading" };
    case "list": return { kind: "list", label: "List" };
    case "code_block": return { kind: "code_block", label: "Code block" };
    case "blockquote": return { kind: "blockquote", label: "Blockquote" };
    case "table": return { kind: "table", label: "Table" };
    default: return { kind: null, label: "Unsupported block" };
  }
}

function PluginButtons(props: {
  readonly contributions: readonly EditorContribution[];
  readonly slot: EditorSlot;
  readonly disabled: boolean;
  readonly activate: (contribution: EditorContribution) => void;
}): ReactNode {
  const matches = props.contributions.filter((item) => item.slot === props.slot);
  if (matches.length === 0) return null;
  return <div className="forme-authoring-editor__actions">
    {matches.map((contribution) => <button
      key={`${contribution.pluginId}:${contribution.actionId}:${contribution.slot}`}
      type="button"
      disabled={props.disabled}
      onClick={() => props.activate(contribution)}
    >{contribution.label} — {contribution.pluginId}</button>)}
  </div>;
}

function text(value: string): InlineNode { return { type: "text", value }; }

function blockForm(
  block: BlockNode,
  index: number,
  disabled: boolean,
  save: (block: BlockNode | (() => BlockNode), message: string) => void,
): ReactNode {
  const classification = blockKind(block);
  switch (classification.kind) {
    case "paragraph":
      return <form onSubmit={(event) => {
        event.preventDefault();
        save({ type: "paragraph", children: [text(formValue(event.currentTarget, "text"))] }, "Paragraph saved.");
      }}>
        <label>Paragraph text<textarea name="text" defaultValue={paragraphText(block)} /></label>
        <button disabled={disabled}>Save paragraph</button>
      </form>;
    case "heading":
      return <form onSubmit={(event) => {
        event.preventDefault();
        const level = Number(formValue(event.currentTarget, "level")) as 1 | 2 | 3 | 4 | 5 | 6;
        save({ type: "heading", level, children: [text(formValue(event.currentTarget, "text"))] }, "Heading saved.");
      }}>
        <label>Heading level<select name="level" defaultValue={block.type === "heading" ? String(block.level) : "2"}>
          {[1, 2, 3, 4, 5, 6].map((level) => <option key={level} value={level}>{level}</option>)}
        </select></label>
        <label>Heading text<input name="text" defaultValue={block.type === "heading" ? inlineText(block.children) : ""} /></label>
        <button disabled={disabled}>Save heading</button>
      </form>;
    case "list": {
      const items = block.type === "list"
        ? block.children.map((item) => paragraphText(item.children[0])).join("\n")
        : "";
      return <form onSubmit={(event) => {
        event.preventDefault();
        const data = new FormData(event.currentTarget);
        const ordered = data.get("ordered") === "on";
        const value = formValue(event.currentTarget, "items");
        save(() => {
          const values = boundedLines(value, MAX_LIST_ITEMS, "List");
          return {
            type: "list",
            ordered,
            start: ordered ? 1 : null,
            tight: true,
            children: values.map((item) => ({
              type: "list_item" as const,
              children: [{ type: "paragraph" as const, children: [text(item)] }],
            })),
          };
        }, "List saved.");
      }}>
        <label>List items, one per line<textarea name="items" defaultValue={items} /></label>
        <label><input name="ordered" type="checkbox" defaultChecked={block.type === "list" && block.ordered} />Ordered list</label>
        <button disabled={disabled}>Save list</button>
      </form>;
    }
    case "image": {
      const image = block.type === "paragraph" && block.children[0]?.type === "image" ? block.children[0] : null;
      return <form onSubmit={(event) => {
        event.preventDefault();
        const title = formValue(event.currentTarget, "title");
        save({ type: "paragraph", children: [{
          type: "image",
          destination: formValue(event.currentTarget, "destination"),
          alt: formValue(event.currentTarget, "alt"),
          title: title === "" ? null : title,
        }] }, "Image saved.");
      }}>
        <label>Image URL<input name="destination" defaultValue={image?.destination ?? ""} /></label>
        <label>Image alternative text<input name="alt" defaultValue={image?.alt ?? ""} /></label>
        <label>Image title<input name="title" defaultValue={image?.title ?? ""} /></label>
        <button disabled={disabled}>Save image</button>
      </form>;
    }
    case "code_block":
      return <form onSubmit={(event) => {
        event.preventDefault();
        const language = formValue(event.currentTarget, "language");
        let value = formValue(event.currentTarget, "code");
        if (!value.endsWith("\n")) value += "\n";
        save({ type: "code_block", language: language === "" ? null : language, value }, "Code block saved.");
      }}>
        <label>Code language<input name="language" defaultValue={block.type === "code_block" ? block.language ?? "" : ""} /></label>
        <label>Code<textarea name="code" defaultValue={block.type === "code_block" ? block.value.replace(/\n$/, "") : ""} /></label>
        <button disabled={disabled}>Save code block</button>
      </form>;
    case "blockquote":
      return <form onSubmit={(event) => {
        event.preventDefault();
        save({
          type: "blockquote",
          children: [{ type: "paragraph", children: [text(formValue(event.currentTarget, "quote"))] }],
        }, "Blockquote saved.");
      }}>
        <label>Quote text<textarea name="quote" defaultValue={block.type === "blockquote" ? paragraphText(block.children[0]) : ""} /></label>
        <button disabled={disabled}>Save blockquote</button>
      </form>;
    case "table": {
      const value = block.type === "table" ? block.children.map((row) => row.children
        .map((cell) => inlineText(cell.children)).join("\t")).join("\n") : "";
      return <form onSubmit={(event) => {
        event.preventDefault();
        const value = formValue(event.currentTarget, "cells");
        save(() => {
          const { rows, width } = boundedTable(value);
          const children: TableRowNode[] = rows.map((row, rowIndex) => ({
            type: "table_row",
            isHeader: rowIndex === 0,
            children: Array.from({ length: width }, (_, cellIndex): TableCellNode => ({
              type: "table_cell",
              children: [text(row[cellIndex] ?? "")],
            })),
          }));
          return { type: "table", align: Array.from({ length: width }, () => null), children };
        }, "Table saved.");
      }}>
        <label>Table cells as tab-separated rows<textarea name="cells" defaultValue={value} /></label>
        <button disabled={disabled}>Save table</button>
      </form>;
    }
    case "link": {
      const link = block.type === "paragraph" && block.children[0]?.type === "link" ? block.children[0] : null;
      return <form onSubmit={(event) => {
        event.preventDefault();
        const title = formValue(event.currentTarget, "title");
        save({ type: "paragraph", children: [{
          type: "link",
          destination: formValue(event.currentTarget, "destination"),
          title: title === "" ? null : title,
          children: [text(formValue(event.currentTarget, "text"))],
        }] }, "Link saved.");
      }}>
        <label>Link text<input name="text" defaultValue={link === null ? "" : inlineText(link.children)} /></label>
        <label>Link URL<input name="destination" defaultValue={link?.destination ?? ""} /></label>
        <label>Link title<input name="title" defaultValue={link?.title ?? ""} /></label>
        <button disabled={disabled}>Save link</button>
      </form>;
    }
    case null:
      return <p>This block type can be moved or removed but is not editable here.</p>;
  }
}

export function AuthoringEditor(props: AuthoringEditorProps): ReactNode {
  const themes = useMemo(() => validateThemeOptions(props.themeOptions), [props.themeOptions]);
  const contributions = useMemo(
    () => validateEditorContributions(props.pluginContributions ?? []),
    [props.pluginContributions],
  );
  if (contributions.length > 0 && props.pluginBridge === undefined) {
    throw new EditorBoundaryError("Declarative plugin controls require an injected plugin bridge.");
  }
  const pluginActionTimeoutMs = props.pluginActionTimeoutMs ?? DEFAULT_PLUGIN_ACTION_TIMEOUT_MS;
  if (!Number.isSafeInteger(pluginActionTimeoutMs) || pluginActionTimeoutMs < 1 || pluginActionTimeoutMs > 60_000) {
    throw new EditorBoundaryError("Plugin action timeout must be a safe integer from 1 through 60000 milliseconds.");
  }

  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [failure, setFailure] = useState("");
  const [renderRevision, setRenderRevision] = useState(0);
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const root = useRef<HTMLElement>(null);
  const busyLock = useRef(false);
  const pluginControllers = useRef(new Set<AbortController>());

  useEffect(() => () => {
    for (const controller of pluginControllers.current) controller.abort();
    pluginControllers.current.clear();
  }, []);

  useEffect(() => {
    if (focusKey === null) return;
    root.current?.querySelector<HTMLElement>(`[data-focus-key="${focusKey}"]`)?.focus();
    setFocusKey(null);
  }, [focusKey, renderRevision]);

  const commit = async (
    operation: () => Promise<void>,
    message: string,
    nextFocus: string | null = null,
  ): Promise<void> => {
    if (busyLock.current) return;
    busyLock.current = true;
    setBusy(true);
    setFailure("");
    setStatus("");
    try {
      await operation();
      setStatus(message);
      setRenderRevision((value) => value + 1);
      setFocusKey(nextFocus);
    } catch {
      setStatus("");
      setFailure("The change could not be saved.");
    } finally {
      busyLock.current = false;
      setBusy(false);
    }
  };

  const project = props.session.project;
  const active = project.documents.find((document) => document.id === project.activeDocumentId) ?? null;
  const dispatch = (command: AuthoringCommand, message: string, nextFocus: string | null = null): void => {
    void commit(() => props.session.dispatch(command), message, nextFocus);
  };
  const replace = (document: AuthoringDocument, body: AuthoringDocument["body"], message: string, nextFocus: string): void => {
    dispatch({ type: "replace-document-body", documentId: document.id, body }, message, nextFocus);
  };
  const activatePlugin = (contribution: EditorContribution, documentId: string | null, blockIndex: number | null): void => {
    void commit(async () => {
      const controller = new AbortController();
      pluginControllers.current.add(controller);
      const timeout = globalThis.setTimeout(() => controller.abort(), pluginActionTimeoutMs);
      try {
        const command = await Promise.race([
          props.pluginBridge!.execute(createEditorPluginRequest(
            contribution,
            props.session.project,
            { documentId, blockIndex },
          ), controller.signal),
          new Promise<never>((_resolve, reject) => {
            controller.signal.addEventListener("abort", () => reject(new DOMException(
              "The plugin action was aborted.",
              "AbortError",
            )), { once: true });
          }),
        ]);
        await props.session.dispatch(command, controller.signal);
      } finally {
        globalThis.clearTimeout(timeout);
        pluginControllers.current.delete(controller);
      }
    }, "Plugin action saved.");
  };

  return <section
    ref={root}
    className="forme-authoring-editor"
    role="region"
    aria-label="Forme authoring editor"
    aria-busy={busy}
  >
    <style>{AUTHORING_EDITOR_CSS}</style>
    <p className="forme-authoring-editor__status" role="status" aria-live="polite">{status}</p>
    {failure !== "" ? <p role="alert">{failure}</p> : null}

    <div className="forme-authoring-editor__actions">
      <button type="button" disabled={busy || !props.session.canUndo} onClick={() => void commit(() => props.session.undo(), "Change undone.")}>Undo</button>
      <button type="button" disabled={busy || !props.session.canRedo} onClick={() => void commit(() => props.session.redo(), "Change redone.")}>Redo</button>
    </div>

    <form key={`site:${props.session.storageRevision}:${renderRevision}`} onSubmit={(event) => {
      event.preventDefault();
      const form = event.currentTarget;
      const baseUrl = formValue(form, "baseUrl");
      dispatch({
        type: "configure-site",
        title: formValue(form, "title"),
        baseUrl: baseUrl === "" ? null : baseUrl,
        themeId: formValue(form, "themeId"),
      }, "Site settings saved.");
    }}>
      <fieldset disabled={busy}>
        <legend>Site settings</legend>
        <label>Site title<input name="title" defaultValue={project.title} /></label>
        <label>Base URL<input name="baseUrl" type="url" defaultValue={project.site.baseUrl ?? ""} /></label>
        <label>Theme<select name="themeId" defaultValue={project.site.themeId}>
          {themes.map((theme) => <option key={theme.id} value={theme.id}>{theme.label}</option>)}
        </select></label>
        <button>Save site settings</button>
      </fieldset>
    </form>
    <PluginButtons
      contributions={contributions}
      slot="site-toolbar"
      disabled={busy}
      activate={(item) => activatePlugin(item, null, null)}
    />

    <nav aria-label="Documents">
      <ul>{project.documents.map((document) => <li key={document.id}>
        <button
          type="button"
          disabled={busy || document.id === project.activeDocumentId}
          onClick={() => dispatch(
            { type: "set-active-document", documentId: document.id },
            "Document selected.",
            "document-heading",
          )}
        >Select {document.title}</button>
      </li>)}</ul>
    </nav>

    <form onSubmit={(event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      const form = event.currentTarget;
      const title = formValue(form, "title");
      const slug = formValue(form, "slug");
      void commit(async () => {
        const id = await props.createDocumentIdentity();
        await props.session.dispatch({
          type: "create-document",
          activate: true,
          document: { id, title, slug, status: "draft", body: { type: "document", children: [] } },
        });
        form.reset();
      }, "Document created.", "document-heading");
    }}>
      <fieldset disabled={busy}>
        <legend>New document</legend>
        <label>New document title<input name="title" /></label>
        <label>New document slug<input name="slug" /></label>
        <button>Create document</button>
      </fieldset>
    </form>

    {active === null ? <p>No document selected.</p> : <article>
      <h2 tabIndex={-1} data-focus-key="document-heading">Editing {active.title}</h2>
      <PluginButtons
        contributions={contributions}
        slot="document-toolbar"
        disabled={busy}
        activate={(item) => activatePlugin(item, active.id, null)}
      />
      <form key={`document:${active.id}:${props.session.storageRevision}:${renderRevision}`} onSubmit={(event) => {
        event.preventDefault();
        const form = event.currentTarget;
        dispatch({
          type: "update-document-metadata",
          documentId: active.id,
          title: formValue(form, "title"),
          slug: formValue(form, "slug"),
          status: formValue(form, "status") as AuthoringDocumentStatus,
        }, "Document settings saved.");
      }}>
        <fieldset disabled={busy}>
          <legend>Document settings</legend>
          <label>Document title<input name="title" defaultValue={active.title} /></label>
          <label>Document slug<input name="slug" defaultValue={active.slug} /></label>
          <label>Document status<select name="status" defaultValue={active.status}>
            <option value="draft">Draft</option><option value="published">Published</option>
          </select></label>
          <button>Save document settings</button>
        </fieldset>
      </form>
      <button
        type="button"
        disabled={busy}
        onClick={() => {
          const nextDocument = project.documents.find((document) => document.id !== active.id) ?? null;
          void commit(async () => {
            await props.session.dispatch({ type: "remove-document", documentId: active.id });
            if (nextDocument !== null) {
              await props.session.dispatch({ type: "set-active-document", documentId: nextDocument.id });
            }
          }, "Document removed.", nextDocument === null ? null : "document-heading");
        }}
      >Remove current document</button>

      <form onSubmit={(event) => {
        event.preventDefault();
        const kind = formValue(event.currentTarget, "kind") as DefaultBlockKind;
        const label = BLOCK_OPTIONS.find((item) => item.value === kind)!.label;
        const index = active.body.children.length;
        replace(active, insertBlock(active.body, index, createDefaultBlock(kind)), `${label} block added.`, `block-${index}`);
      }}>
        <fieldset disabled={busy}>
          <legend>Add block</legend>
          <label>Block type<select name="kind" defaultValue="paragraph">
            {BLOCK_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select></label>
          <button data-focus-key="block-add">Add block</button>
        </fieldset>
      </form>

      <div key={`blocks:${props.session.storageRevision}:${renderRevision}`}>
        {active.body.children.map((block, index) => {
          const classification = blockKind(block);
          const headingId = `forme-block-${index}`;
          return <section key={index} role="group" aria-labelledby={headingId}>
            <h3 id={headingId} tabIndex={-1} data-focus-key={`block-${index}`}>Block {index + 1}: {classification.label}</h3>
            <div className="forme-authoring-editor__actions">
              <button type="button" disabled={busy || index === 0} onClick={() => replace(
                active, moveBlock(active.body, index, index - 1), "Block moved.", `block-${index - 1}`,
              )}>Move up</button>
              <button type="button" disabled={busy || index + 1 === active.body.children.length} onClick={() => replace(
                active, moveBlock(active.body, index, index + 1), "Block moved.", `block-${index + 1}`,
              )}>Move down</button>
              <button type="button" disabled={busy} onClick={() => replace(
                active,
                removeBlock(active.body, index),
                "Block removed.",
                active.body.children.length === 1
                  ? "block-add"
                  : `block-${Math.min(index, active.body.children.length - 2)}`,
              )}>Remove block</button>
            </div>
            {blockForm(block, index, busy, (replacement, message) => {
              void commit(async () => {
                const blockValue = typeof replacement === "function" ? replacement() : replacement;
                await props.session.dispatch({
                  type: "replace-document-body",
                  documentId: active.id,
                  body: replaceBlock(active.body, index, blockValue),
                });
              }, message, `block-${index}`);
            })}
            <PluginButtons
              contributions={contributions}
              slot="block-toolbar"
              disabled={busy}
              activate={(item) => activatePlugin(item, active.id, index)}
            />
          </section>;
        })}
      </div>
    </article>}
  </section>;
}
