/** Match Style IR selectors against the HTML element tree emitted for a page. */

import type { Node } from "@coding-adventures/document-ast";
import type {
  Selector,
  StyleDocument,
  StyleRuleId,
} from "@coding-adventures/forme-style-ir";

interface MatchNode {
  readonly type: string;
  readonly level?: number;
  readonly role?: string;
  readonly id?: string;
  readonly tags: ReadonlySet<string>;
  readonly parent: MatchNode | null;
  readonly previous: MatchNode | null;
  readonly children: readonly MatchNode[];
}

interface MutableMatchNode {
  type: string;
  level?: number;
  role?: string;
  id?: string;
  tags: Set<string>;
  parent: MutableMatchNode | null;
  previous: MutableMatchNode | null;
  children: MutableMatchNode[];
}

export interface CollectUsedStyleOptions {
  readonly siteHeader: boolean;
  readonly frontmatter: Readonly<Record<string, unknown>>;
}

/**
 * Return matching rule ids in StyleDocument source order. Raw HTML is opaque to
 * the Document AST renderer, so a page containing it conservatively retains all
 * rules rather than risk slicing away CSS that the trusted raw fragment uses.
 */
export function collectUsedStyle(
  document: Node,
  style: StyleDocument,
  options: CollectUsedStyleOptions,
): readonly StyleRuleId[] {
  const tags = frontmatterTags(options.frontmatter);
  const root = element("html", null);
  const body = append(root, "body");
  if (options.siteHeader) {
    const header = append(body, "header", { role: "banner" });
    append(header, "a");
  }
  const main = append(body, "main", { role: "main", tags });
  const opaqueRawHtml = appendDocumentNode(document, main, false);
  if (opaqueRawHtml) return style.rules.map((rule) => rule.id);

  const nodes = flatten(root);
  return style.rules
    .filter((rule) => nodes.some((node) => matches(rule.selector, node)))
    .map((rule) => rule.id);
}

function element(type: string, parent: MutableMatchNode | null, extra: {
  readonly level?: number;
  readonly role?: string;
  readonly id?: string;
  readonly tags?: ReadonlySet<string>;
} = {}): MutableMatchNode {
  return {
    type,
    ...(extra.level === undefined ? {} : { level: extra.level }),
    ...(extra.role === undefined ? {} : { role: extra.role }),
    ...(extra.id === undefined ? {} : { id: extra.id }),
    tags: new Set(extra.tags ?? []),
    parent,
    previous: null,
    children: [],
  };
}

function append(parent: MutableMatchNode, type: string, extra: Parameters<typeof element>[2] = {}): MutableMatchNode {
  const child = element(type, parent, extra);
  child.previous = parent.children.at(-1) ?? null;
  parent.children.push(child);
  return child;
}

function appendDocumentNode(node: Node, parent: MutableMatchNode, tight: boolean): boolean {
  switch (node.type) {
    case "document":
      return node.children.some((child) => appendDocumentNode(child, parent, false));
    case "heading": {
      const current = append(parent, `h${node.level}`, { level: node.level });
      return node.children.some((child) => appendDocumentNode(child, current, false));
    }
    case "paragraph":
      if (tight) return node.children.some((child) => appendDocumentNode(child, parent, false));
      return node.children.some((child) => appendDocumentNode(child, append(parent, "p"), false));
    case "code_block": {
      append(append(parent, "pre"), "code");
      return false;
    }
    case "blockquote": {
      const current = append(parent, "blockquote");
      return node.children.some((child) => appendDocumentNode(child, current, false));
    }
    case "list": {
      const current = append(parent, node.ordered ? "ol" : "ul");
      return node.children.some((child) => appendDocumentNode(child, current, node.tight));
    }
    case "list_item":
    case "task_item": {
      const current = append(parent, "li");
      if (node.type === "task_item") append(current, "input");
      return node.children.some((child) => appendDocumentNode(child, current, tight));
    }
    case "thematic_break":
      append(parent, "hr");
      return false;
    case "raw_block":
      return node.format === "html";
    case "table": {
      const table = append(parent, "table");
      const headerRows = node.children.filter((row) => row.isHeader);
      const bodyRows = node.children.filter((row) => !row.isHeader);
      const thead = headerRows.length === 0 ? null : append(table, "thead");
      const tbody = bodyRows.length === 0 ? null : append(table, "tbody");
      return headerRows.some((row) => appendDocumentNode(row, thead!, false))
        || bodyRows.some((row) => appendDocumentNode(row, tbody!, false));
    }
    case "table_row": {
      const row = append(parent, "tr");
      return node.children.some((child) => appendTableCell(child, row, node.isHeader));
    }
    case "table_cell":
      return appendTableCell(node, parent, false);
    case "text":
    case "code_span":
    case "hard_break":
    case "soft_break":
      if (node.type === "code_span") append(parent, "code");
      if (node.type === "hard_break") append(parent, "br");
      return false;
    case "emphasis":
    case "strong":
    case "strikethrough": {
      const tag = node.type === "emphasis" ? "em" : node.type === "strong" ? "strong" : "del";
      const current = append(parent, tag);
      return node.children.some((child) => appendDocumentNode(child, current, false));
    }
    case "link": {
      const current = append(parent, "a");
      return node.children.some((child) => appendDocumentNode(child, current, false));
    }
    case "image":
      append(parent, "img");
      return false;
    case "autolink":
      append(parent, "a");
      return false;
    case "raw_inline":
      return node.format === "html";
  }
}

function appendTableCell(node: Extract<Node, { type: "table_cell" }>, parent: MutableMatchNode, header: boolean): boolean {
  const cell = append(parent, header ? "th" : "td");
  return node.children.some((child) => appendDocumentNode(child, cell, false));
}

function flatten(root: MatchNode): readonly MatchNode[] {
  const out: MatchNode[] = [];
  const visit = (node: MatchNode): void => {
    out.push(node);
    node.children.forEach(visit);
  };
  visit(root);
  return out;
}

function matches(selector: Selector, node: MatchNode): boolean {
  switch (selector.kind) {
    case "node-type": return node.type === selector.type;
    case "node-type-level": return node.level === selector.level;
    case "custom-kind": return false;
    case "tag": return node.tags.has(selector.tag) || ancestors(node).some((item) => item.tags.has(selector.tag));
    case "id": return node.id === selector.id;
    case "role": return node.role === selector.role;
    case "nth": {
      if (!matches(selector.of, node) || node.parent === null) return false;
      const peers = node.parent.children.filter((peer) => matches(selector.of, peer));
      const index = peers.indexOf(node);
      if (typeof selector.n === "number") return index === selector.n;
      const candidate = selector.n.fromEnd ? peers.length - index : index + 1;
      return matchesFormula(candidate, selector.n.a, selector.n.b);
    }
    case "child-of": return matches(selector.child, node)
      && node.parent !== null && matches(selector.parent, node.parent);
    case "descendant-of": return matches(selector.descendant, node)
      && ancestors(node).some((ancestor) => matches(selector.ancestor, ancestor));
    case "adjacent": return matches(selector.following, node)
      && node.previous !== null && matches(selector.previous, node.previous);
    case "and": return selector.all.every((inner) => matches(inner, node));
    case "or": return selector.any.some((inner) => matches(inner, node));
    case "not": return !matches(selector.inner, node);
  }
}

function ancestors(node: MatchNode): readonly MatchNode[] {
  const out: MatchNode[] = [];
  let current = node.parent;
  while (current !== null) {
    out.push(current);
    current = current.parent;
  }
  return out;
}

function matchesFormula(position: number, a: number, b: number): boolean {
  if (a === 0) return position === b;
  const quotient = (position - b) / a;
  return Number.isInteger(quotient) && quotient >= 0;
}

function frontmatterTags(frontmatter: Readonly<Record<string, unknown>>): ReadonlySet<string> {
  const value = frontmatter["tags"];
  if (Array.isArray(value)) {
    return new Set(value.filter((tag): tag is string => typeof tag === "string"));
  }
  return typeof value === "string"
    ? new Set(value.split(",").map((tag) => tag.trim()).filter(Boolean))
    : new Set();
}
