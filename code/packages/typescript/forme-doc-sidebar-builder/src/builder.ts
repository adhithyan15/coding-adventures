/**
 * builder.ts — main `buildSidebar` entry.
 *
 * =============================================================================
 * THE ALGORITHM
 * =============================================================================
 *
 * The transformation is conceptually:
 *
 *   pages: PageInput[]  ──►  SidebarEntry[]
 *
 * but the interesting bit is the three-phase pipeline:
 *
 *   Phase 1 — Filter + Normalise
 *     Drop drafts (`frontmatter.draft === true`).  Normalise each
 *     remaining page's path to `{ parts: string[], isIndex: bool }`
 *     and (if a `root` option was given) strip the root prefix.
 *     Pages outside the root are dropped.
 *
 *   Phase 2 — Trie build
 *     Build a directory trie keyed by normalised parts.  Each trie
 *     node holds:
 *       - `pages`: leaf pages whose parts terminate AT this node
 *       - `subdirs`: child trie nodes keyed by the next path part
 *       - `indexPage`: a page whose `isIndex` was true and whose
 *                       parts ended exactly here.  At most one.
 *     Duplicate non-index pages at the same path throw; duplicate
 *     indexes throw too (catches misconfigurations early).
 *
 *   Phase 3 — Emit
 *     Recursively walk the trie depth-first, emitting:
 *       - `SidebarPageEntry` for each `pages` entry
 *       - `SidebarGroupEntry` for each `subdirs` entry (recursing)
 *     At each level, sort by `(position ?? +Infinity, label)`.
 *     A group with an index page inherits the index's title,
 *     position, and path.
 *
 * =============================================================================
 * INVARIANTS
 * =============================================================================
 *
 *   - Output is JSON-safe: only strings, numbers, booleans, nulls,
 *     and arrays/objects of the same.  No AST references, no
 *     `Date`s, no symbols.
 *   - Same input → identical output (sort is stable + total).
 *   - No mutation of input — the caller's `pages` array and
 *     `frontmatter` objects are never written to.
 *
 * @module builder
 */

import { humanise } from "./labels.js";
import { normalisePath, stripRoot } from "./path-utils.js";
import type {
  PageInput,
  BuildSidebarOptions,
  SidebarEntry,
  SidebarPageEntry,
  SidebarGroupEntry,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────
// Internal trie types
// ─────────────────────────────────────────────────────────────────────

interface TrieNode {
  /** Sub-directory name → child trie node. */
  readonly subdirs: Map<string, TrieNode>;
  /** File-slug → leaf page descriptor (non-index pages). */
  readonly pages: Map<string, NormalisedPage>;
  /** Index page for this directory, or `null` if none. */
  indexPage: NormalisedPage | null;
}

/** A page after normalisation, with the four well-known frontmatter
 *  fields extracted defensively. */
interface NormalisedPage {
  /** Original path verbatim (for output). */
  readonly path: string;
  /** Whether this is an `index.md`-style page. */
  readonly isIndex: boolean;
  /** Normalised directory parts (incl. file slug for non-index). */
  readonly parts: readonly string[];
  /** Effective label: `sidebar_label ?? title ?? humanised slug`. */
  readonly label: string;
  /** Numeric `sidebar_position` or `null`. */
  readonly position: number | null;
}

// ─────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────

/**
 * Build a hierarchical sidebar tree from a flat list of pages.
 *
 * @param pages - One entry per `.md` page in the site.  Order
 *                doesn't matter; the builder sorts deterministically.
 * @param options - `{ root?: string }`.  Default: no root stripping.
 * @returns A `SidebarEntry[]` ready to render or `JSON.stringify`.
 * @throws `TypeError` on duplicate non-index pages at the same
 *         path, duplicate index pages for the same directory, or
 *         empty `path` strings.
 */
export function buildSidebar(
  pages: readonly PageInput[],
  options: BuildSidebarOptions = {},
): readonly SidebarEntry[] {
  const root = options.root ?? "";

  // ─── Phase 1: filter + normalise ─────────────────────────────────
  const normalised: NormalisedPage[] = [];
  for (const page of pages) {
    if (isDraft(page.frontmatter)) continue;
    const np = normalisePath(page.path);
    const remaining = stripRoot(np.parts, root);
    if (remaining === null) continue;
    normalised.push({
      path: page.path,
      isIndex: np.isIndex,
      parts: remaining,
      label: deriveLabel(page.frontmatter, remaining, np.isIndex),
      position: readPosition(page.frontmatter),
    });
  }

  // ─── Phase 2: trie build ─────────────────────────────────────────
  const rootNode = makeNode();
  for (const np of normalised) {
    insertIntoTrie(rootNode, np);
  }

  // ─── Phase 3: emit ───────────────────────────────────────────────
  return emit(rootNode);
}

// ─────────────────────────────────────────────────────────────────────
// Phase-1 helpers
// ─────────────────────────────────────────────────────────────────────

/**
 * Read an own (non-inherited) property from frontmatter.  Returns
 * `undefined` if the key isn't directly present on the object,
 * EVEN IF a prototype-chain ancestor would have provided a value.
 *
 * This matters when the caller hand-crafts a frontmatter literal
 * like `{ __proto__: { title: "Polluted" } }` — the JS object
 * literal syntax treats `__proto__` specially and sets the
 * prototype rather than creating an own property.  A naive
 * `frontmatter.title` lookup would then traverse the prototype
 * and return `"Polluted"`, which is a prototype-pollution-style
 * surprise.  `Object.hasOwn` bypasses the chain.
 *
 * In practice `forme-doc-frontmatter` returns null-prototype
 * objects (via `Object.create(null)`), so the chain is always
 * empty.  This is defence-in-depth for callers using arbitrary
 * frontmatter sources.
 */
function readOwn(
  frontmatter: Readonly<Record<string, unknown>>,
  key: string,
): unknown {
  return Object.hasOwn(frontmatter, key) ? frontmatter[key] : undefined;
}

/**
 * True iff frontmatter has `draft: true` as an OWN property.
 * Defensive against unknown shapes — any non-boolean-`true` value
 * (incl. truthy strings like `"true"`) is treated as non-draft.
 * v0 keeps the contract strict; relaxation can come later if needed.
 */
function isDraft(frontmatter: Readonly<Record<string, unknown>>): boolean {
  return readOwn(frontmatter, "draft") === true;
}

/**
 * Compute the effective display label for a page:
 *   `sidebar_label` (string) ?? `title` (string) ?? humanised slug.
 *
 * For index pages, the "slug" is the parent directory's name (the
 * group label).  For empty `parts` (root index), it's `"Home"`.
 */
function deriveLabel(
  frontmatter: Readonly<Record<string, unknown>>,
  parts: readonly string[],
  isIndex: boolean,
): string {
  const sb = readOwn(frontmatter, "sidebar_label");
  if (typeof sb === "string" && sb.length > 0) return sb;
  const t = readOwn(frontmatter, "title");
  if (typeof t === "string" && t.length > 0) return t;
  if (parts.length === 0) {
    // Root index page — sensible default.  (Only reachable for
    // `index.md` at the site root; the non-index branch is
    // guarded against in `insertIntoTrie`, which rejects
    // slug-less non-index pages.)
    void isIndex;
    return "Home";
  }
  return humanise(parts[parts.length - 1]!);
}

/**
 * Read `sidebar_position` defensively.  Accepts only finite
 * numbers; everything else (string, NaN, ±Infinity, undefined,
 * non-numeric) becomes `null`.
 */
function readPosition(frontmatter: Readonly<Record<string, unknown>>): number | null {
  const p = readOwn(frontmatter, "sidebar_position");
  if (typeof p !== "number") return null;
  if (!Number.isFinite(p)) return null;
  return p;
}

// ─────────────────────────────────────────────────────────────────────
// Phase-2 helpers
// ─────────────────────────────────────────────────────────────────────

function makeNode(): TrieNode {
  return {
    subdirs: new Map(),
    pages: new Map(),
    indexPage: null,
  };
}

function insertIntoTrie(root: TrieNode, page: NormalisedPage): void {
  if (page.isIndex) {
    // Walk to the directory the index belongs to.
    const dir = walkTo(root, page.parts);
    if (dir.indexPage !== null) {
      throw new TypeError(
        `forme-doc-sidebar-builder: duplicate index page for directory ` +
          `${JSON.stringify(page.parts.join("/"))} ` +
          `(saw ${JSON.stringify(dir.indexPage.path)} and ${JSON.stringify(page.path)})`,
      );
    }
    dir.indexPage = page;
    return;
  }
  // Non-index: last part is the file slug, prior parts are dir.
  if (page.parts.length === 0) {
    // path normalised to "" with isIndex=false — shouldn't happen
    // (the normaliser only produces empty parts for index), but
    // guard defensively.
    throw new TypeError(
      `forme-doc-sidebar-builder: page ${JSON.stringify(page.path)} has no slug`,
    );
  }
  const dir = walkTo(root, page.parts.slice(0, -1));
  const slug = page.parts[page.parts.length - 1]!;
  if (dir.pages.has(slug)) {
    throw new TypeError(
      `forme-doc-sidebar-builder: duplicate page at path ${JSON.stringify(page.path)} ` +
        `(slug ${JSON.stringify(slug)} already present at ${JSON.stringify(dir.pages.get(slug)!.path)})`,
    );
  }
  dir.pages.set(slug, page);
}

/**
 * Walk the trie, creating intermediate subdir nodes as needed.
 * Returns the node at `parts`.  Empty `parts` returns `root`.
 */
function walkTo(root: TrieNode, parts: readonly string[]): TrieNode {
  let cur = root;
  for (const seg of parts) {
    let next = cur.subdirs.get(seg);
    if (next === undefined) {
      next = makeNode();
      cur.subdirs.set(seg, next);
    }
    cur = next;
  }
  return cur;
}

// ─────────────────────────────────────────────────────────────────────
// Phase-3 helpers
// ─────────────────────────────────────────────────────────────────────

/**
 * Emit a sorted `SidebarEntry[]` for the given trie node.
 *
 * Sort key: `(position ?? +Infinity, label)` ascending.  Numeric
 * positions tie-break by label alphabetical; absent positions all
 * sink to the end and tie-break by label too.  This gives a
 * predictable order even for fully-unannotated sites.
 */
function emit(node: TrieNode): readonly SidebarEntry[] {
  const entries: SidebarEntry[] = [];

  // Pages (non-index leaves) — one entry each.
  for (const p of node.pages.values()) {
    entries.push({
      kind: "page",
      label: p.label,
      path: p.path,
      position: p.position,
    });
  }

  // Subdirs — recurse, attach index metadata if present.
  for (const [seg, child] of node.subdirs) {
    entries.push(emitGroup(seg, child));
  }

  // Sort.
  entries.sort(compareEntries);
  return entries;
}

function emitGroup(seg: string, child: TrieNode): SidebarGroupEntry {
  const idx = child.indexPage;
  const childEntries = emit(child);
  return {
    kind: "group",
    label: idx !== null ? idx.label : humanise(seg),
    path: idx !== null ? idx.path : null,
    position: idx !== null ? idx.position : null,
    children: childEntries,
  };
}

function compareEntries(a: SidebarEntry, b: SidebarEntry): number {
  // Primary: numeric position ascending.  `null` sorts last (treat
  // as +Infinity).
  const ap = a.position ?? Number.POSITIVE_INFINITY;
  const bp = b.position ?? Number.POSITIVE_INFINITY;
  if (ap !== bp) return ap - bp;
  // Secondary: label alphabetical (locale-independent).
  if (a.label < b.label) return -1;
  if (a.label > b.label) return 1;
  return 0;
}
