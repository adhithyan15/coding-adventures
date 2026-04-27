/**
 * @coding-adventures/format-doc
 *
 * This package defines a small document algebra for pretty-printers.
 *
 * A formatter should not emit strings directly. Instead it builds a semantic
 * `Doc` tree that records where text exists, where breaks are allowed, which
 * sections should stay together if possible, and which spans carry metadata for
 * later consumers.
 *
 * `layoutDoc()` is the first execution phase. It realizes the `Doc` tree into a
 * backend-neutral `DocLayoutTree`, choosing flat vs broken groups based on the
 * configured print width.
 */

// ============================================================================
// Public constants
// ============================================================================

/** Package version, kept in source for easy cross-package smoke tests. */
export const VERSION = "0.1.0";

// ============================================================================
// Types
// ============================================================================

/**
 * Lightweight metadata attached to emitted spans.
 *
 * We keep this open-ended so future formatter packages can attach node ids,
 * token classes, or source spans without changing the core algebra.
 */
export type DocAnnotation = string | number | boolean | null | Record<string, unknown>;

/** A backend-neutral pretty-printing document. */
export type Doc =
  | { kind: "nil" }
  | { kind: "text"; value: string }
  | { kind: "concat"; parts: readonly Doc[] }
  | { kind: "group"; content: Doc }
  | { kind: "indent"; levels: number; content: Doc }
  | { kind: "line"; mode: "soft" | "normal" | "hard" }
  | { kind: "if_break"; broken: Doc; flat: Doc }
  | { kind: "annotate"; annotation: DocAnnotation; content: Doc };

/** One realized span of text, annotated with the metadata active at emission time. */
export interface DocLayoutSpan {
  column: number;
  text: string;
  annotations: readonly DocAnnotation[];
}

/** One realized line in the layout tree. */
export interface DocLayoutLine {
  row: number;
  indentColumns: number;
  width: number;
  spans: readonly DocLayoutSpan[];
}

/** The backend-neutral result of realizing a `Doc` tree. */
export interface DocLayoutTree {
  printWidth: number;
  indentWidth: number;
  lineHeight: number;
  width: number;
  height: number;
  lines: readonly DocLayoutLine[];
}

/** Configuration for the width-aware realization pass. */
export interface LayoutOptions {
  printWidth: number;
  indentWidth?: number;
  lineHeight?: number;
}

type Mode = "flat" | "break";

interface Command {
  indentLevels: number;
  mode: Mode;
  annotations: readonly DocAnnotation[];
  doc: Doc;
}

interface MutableDocLayoutSpan {
  column: number;
  text: string;
  annotations: readonly DocAnnotation[];
}

interface MutableDocLayoutLine {
  row: number;
  indentColumns: number;
  spans: MutableDocLayoutSpan[];
}

// ============================================================================
// Primitive docs
// ============================================================================

const NIL_DOC: Doc = { kind: "nil" };
const LINE_DOC: Doc = { kind: "line", mode: "normal" };
const SOFTLINE_DOC: Doc = { kind: "line", mode: "soft" };
const HARDLINE_DOC: Doc = { kind: "line", mode: "hard" };

// ============================================================================
// Builders
// ============================================================================

/** Return the empty document. Useful as the neutral element of `concat()`. */
export function nil(): Doc {
  return NIL_DOC;
}

/** Wrap literal text. Empty strings collapse to `nil()` to keep doc trees tidy. */
export function text(value: string): Doc {
  return value.length === 0 ? NIL_DOC : { kind: "text", value };
}

/**
 * Concatenate docs in order.
 *
 * This builder flattens nested concats and drops empty docs so printers can
 * freely compose without producing deeply nested noise.
 */
export function concat(parts: readonly Doc[]): Doc {
  const flat: Doc[] = [];

  for (const part of parts) {
    if (part.kind === "nil") {
      continue;
    }
    if (part.kind === "concat") {
      for (const nested of part.parts) {
        if (nested.kind !== "nil") {
          flat.push(nested);
        }
      }
      continue;
    }
    flat.push(part);
  }

  if (flat.length === 0) {
    return NIL_DOC;
  }
  if (flat.length === 1) {
    return flat[0]!;
  }
  return { kind: "concat", parts: flat };
}

/** Join docs with a separator document. */
export function join(separator: Doc, parts: readonly Doc[]): Doc {
  if (parts.length === 0) {
    return NIL_DOC;
  }

  const out: Doc[] = [];
  for (let i = 0; i < parts.length; i += 1) {
    if (i > 0) {
      out.push(separator);
    }
    out.push(parts[i]!);
  }
  return concat(out);
}

/** Mark a subtree as a unit that should stay flat if it fits. */
export function group(content: Doc): Doc {
  return { kind: "group", content };
}

/** Increase indentation by one logical level, or a caller-specified number of levels. */
export function indent(content: Doc, levels = 1): Doc {
  return levels === 0 ? content : { kind: "indent", levels, content };
}

/** Emit a space when flat and a newline when broken. */
export function line(): Doc {
  return LINE_DOC;
}

/** Emit nothing when flat and a newline when broken. */
export function softline(): Doc {
  return SOFTLINE_DOC;
}

/** Always emit a newline, even inside otherwise flat content. */
export function hardline(): Doc {
  return HARDLINE_DOC;
}

/** Emit one doc in broken mode and another doc in flat mode. */
export function ifBreak(broken: Doc, flat: Doc = NIL_DOC): Doc {
  return { kind: "if_break", broken, flat };
}

/** Attach metadata to the spans emitted by the wrapped document. */
export function annotate(annotation: DocAnnotation, content: Doc): Doc {
  return { kind: "annotate", annotation, content };
}

// ============================================================================
// Realization
// ============================================================================

/**
 * Realize a `Doc` tree into a backend-neutral layout tree.
 *
 * This is the core interpreter for the document algebra. It walks the doc with
 * a stack of commands, decides when groups fit, and emits positioned text spans
 * arranged into lines in monospace cell units.
 */
export function layoutDoc(doc: Doc, options: LayoutOptions): DocLayoutTree {
  if (options.printWidth <= 0) {
    throw new Error("layoutDoc() requires printWidth > 0");
  }

  const indentWidth = options.indentWidth ?? 2;
  const lineHeight = options.lineHeight ?? 1;
  const lines: MutableDocLayoutLine[] = [{ row: 0, indentColumns: 0, spans: [] }];
  let current = lines[0]!;
  let column = 0;
  let maxColumn = 0;

  const stack: Command[] = [
    { indentLevels: 0, mode: "break", annotations: [], doc },
  ];

  const pushText = (value: string, annotations: readonly DocAnnotation[]): void => {
    if (value.length === 0) {
      return;
    }

    const last = current.spans[current.spans.length - 1];
    if (last && sameAnnotations(last.annotations, annotations) && last.column + last.text.length === column) {
      last.text += value;
    } else {
      current.spans.push({ column, text: value, annotations: [...annotations] });
    }

    column += value.length;
    if (column > maxColumn) {
      maxColumn = column;
    }
  };

  const pushLineBreak = (indentLevels: number): void => {
    const indentColumns = indentLevels * indentWidth;
    current = { row: lines.length, indentColumns, spans: [] };
    lines.push(current);
    column = indentColumns;
    if (column > maxColumn) {
      maxColumn = column;
    }
  };

  while (stack.length > 0) {
    const cmd = stack.pop()!;

    switch (cmd.doc.kind) {
      case "nil":
        break;

      case "text":
        pushText(cmd.doc.value, cmd.annotations);
        break;

      case "concat":
        pushDocs(stack, cmd, cmd.doc.parts);
        break;

      case "group":
        if (
          cmd.mode === "flat" ||
          fits(
            options.printWidth - column,
            stack,
            {
              indentLevels: cmd.indentLevels,
              mode: "flat",
              annotations: cmd.annotations,
              doc: cmd.doc.content,
            }
          )
        ) {
          stack.push({
            indentLevels: cmd.indentLevels,
            mode: "flat",
            annotations: cmd.annotations,
            doc: cmd.doc.content,
          });
        } else {
          stack.push({
            indentLevels: cmd.indentLevels,
            mode: "break",
            annotations: cmd.annotations,
            doc: cmd.doc.content,
          });
        }
        break;

      case "indent":
        stack.push({
          indentLevels: cmd.indentLevels + cmd.doc.levels,
          mode: cmd.mode,
          annotations: cmd.annotations,
          doc: cmd.doc.content,
        });
        break;

      case "line":
        if (cmd.doc.mode === "hard") {
          pushLineBreak(cmd.indentLevels);
          break;
        }

        if (cmd.mode === "flat") {
          if (cmd.doc.mode === "normal") {
            pushText(" ", cmd.annotations);
          }
          break;
        }

        pushLineBreak(cmd.indentLevels);
        break;

      case "if_break":
        stack.push({
          indentLevels: cmd.indentLevels,
          mode: cmd.mode,
          annotations: cmd.annotations,
          doc: cmd.mode === "flat" ? cmd.doc.flat : cmd.doc.broken,
        });
        break;

      case "annotate":
        stack.push({
          indentLevels: cmd.indentLevels,
          mode: cmd.mode,
          annotations: [...cmd.annotations, cmd.doc.annotation],
          doc: cmd.doc.content,
        });
        break;

      default: {
        const unreachable: never = cmd.doc;
        throw new Error(`Unknown doc kind: ${(unreachable as { kind?: string }).kind ?? "unknown"}`);
      }
    }
  }

  const finalizedLines: DocLayoutLine[] = lines.map((lineData) => ({
    row: lineData.row,
    indentColumns: lineData.indentColumns,
    width: lineWidth(lineData),
    spans: lineData.spans,
  }));

  return {
    printWidth: options.printWidth,
    indentWidth,
    lineHeight,
    width: maxColumn,
    height: finalizedLines.length * lineHeight,
    lines: finalizedLines,
  };
}

// ============================================================================
// Internal helpers
// ============================================================================

function pushDocs(stack: Command[], base: Command, docs: readonly Doc[]): void {
  for (let i = docs.length - 1; i >= 0; i -= 1) {
    stack.push({
      indentLevels: base.indentLevels,
      mode: base.mode,
      annotations: base.annotations,
      doc: docs[i]!,
    });
  }
}

function sameAnnotations(
  left: readonly DocAnnotation[],
  right: readonly DocAnnotation[]
): boolean {
  if (left.length !== right.length) {
    return false;
  }
  for (let i = 0; i < left.length; i += 1) {
    if (left[i] !== right[i]) {
      return false;
    }
  }
  return true;
}

function lineWidth(line: MutableDocLayoutLine): number {
  if (line.spans.length === 0) {
    return line.indentColumns;
  }
  const last = line.spans[line.spans.length - 1]!;
  return last.column + last.text.length;
}

/**
 * Look ahead to see whether the pending document stack can stay on the current
 * line if we continue in flat mode.
 *
 * The algorithm is intentionally conservative: any hard line in a flat
 * candidate forces failure, while any already-broken line outside the candidate
 * ends the check successfully because the current line may break there.
 */
function fits(
  remaining: number,
  stack: readonly Command[],
  next: Command
): boolean {
  const pending: Command[] = [...stack, next];

  while (remaining >= 0) {
    const cmd = pending.pop();
    if (!cmd) {
      return true;
    }

    switch (cmd.doc.kind) {
      case "nil":
        break;

      case "text":
        remaining -= cmd.doc.value.length;
        break;

      case "concat":
        pushDocs(pending, cmd, cmd.doc.parts);
        break;

      case "group":
        pending.push({
          indentLevels: cmd.indentLevels,
          mode: "flat",
          annotations: cmd.annotations,
          doc: cmd.doc.content,
        });
        break;

      case "indent":
        pending.push({
          indentLevels: cmd.indentLevels + cmd.doc.levels,
          mode: cmd.mode,
          annotations: cmd.annotations,
          doc: cmd.doc.content,
        });
        break;

      case "line":
        if (cmd.doc.mode === "hard") {
          return false;
        }
        if (cmd.mode === "flat") {
          if (cmd.doc.mode === "normal") {
            remaining -= 1;
          }
          break;
        }
        return true;

      case "if_break":
        pending.push({
          indentLevels: cmd.indentLevels,
          mode: cmd.mode,
          annotations: cmd.annotations,
          doc: cmd.mode === "flat" ? cmd.doc.flat : cmd.doc.broken,
        });
        break;

      case "annotate":
        pending.push({
          indentLevels: cmd.indentLevels,
          mode: cmd.mode,
          annotations: cmd.annotations,
          doc: cmd.doc.content,
        });
        break;

      default: {
        const unreachable: never = cmd.doc;
        throw new Error(`Unknown doc kind in fits(): ${(unreachable as { kind?: string }).kind ?? "unknown"}`);
      }
    }
  }

  return false;
}
