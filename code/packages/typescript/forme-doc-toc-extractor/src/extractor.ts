/**
 * extractor.ts — DocumentNode → hierarchical table-of-contents tree.
 *
 * =============================================================================
 * WHAT IS A TABLE OF CONTENTS?
 * =============================================================================
 *
 * A table of contents (TOC) is the outline you see in a documentation
 * sidebar, the floating "On this page" widget, or the bookmarks pane
 * of a PDF reader.  Structurally it's a hierarchical tree built from
 * the FLAT sequence of heading levels in the source:
 *
 *     # Introduction          (level 1)  ┐
 *     ## Setup                 (level 2)  ├─ Introduction
 *     ### Prerequisites        (level 3)  │  ├─ Setup
 *     ### Install              (level 3)  │  │  ├─ Prerequisites
 *     ## Quick start           (level 2)  │  │  └─ Install
 *     # Reference              (level 1)  │  └─ Quick start
 *     ## API                   (level 2)  └─ Reference
 *                                            └─ API
 *
 * The interesting bit is the level→parent mapping: a heading of level
 * N belongs under the most recent heading of level < N.  When a level
 * SKIPS (level 1 → level 3 with no intervening level 2), the level-3
 * heading still nests directly under the level-1 heading.  When a
 * level DROPS back (level 3 → level 1), the level-1 heading becomes
 * a sibling of the original level-1 heading, not a child of anything.
 *
 * =============================================================================
 * THE STACK-BASED ALGORITHM
 * =============================================================================
 *
 * The textbook algorithm uses a stack of "currently open" ancestors:
 *
 *   1. Push a virtual root with level=0 onto the stack.
 *   2. For each heading h:
 *        a. While stack.top.level >= h.level: pop.
 *           (Anyone at our level or deeper is a sibling or descendant,
 *           not an ancestor — they're done receiving children.)
 *        b. Create a new entry for h with empty children[].
 *        c. Push it onto the stack.top's children array.
 *        d. Push h onto the stack (it's now the deepest open ancestor).
 *   3. Return root.children.
 *
 * Complexity: O(N) total — each heading is pushed and popped once.
 *
 * Worked example for the document above:
 *
 *   Stack starts: [root(0)]
 *
 *   #1 "Introduction" (1):
 *     Pop while top>=1: nothing pops (root is 0 < 1).
 *     Push under root: root.children = [Intro]
 *     Push Intro: [root, Intro(1)]
 *
 *   #2 "Setup" (2):
 *     Pop while top>=2: nothing pops.
 *     Push under Intro: Intro.children = [Setup]
 *     Push Setup: [root, Intro, Setup(2)]
 *
 *   #3 "Prerequisites" (3):
 *     Pop while top>=3: nothing pops.
 *     Push under Setup: Setup.children = [Prereq]
 *     Push Prereq: [root, Intro, Setup, Prereq(3)]
 *
 *   #4 "Install" (3):
 *     Pop while top>=3: pop Prereq.  Stack: [root, Intro, Setup].
 *     Push under Setup: Setup.children = [Prereq, Install]
 *     Push Install: [root, Intro, Setup, Install(3)]
 *
 *   #5 "Quick start" (2):
 *     Pop while top>=2: pop Install, pop Setup.  Stack: [root, Intro].
 *     Push under Intro: Intro.children = [Setup, Quick]
 *     Push Quick: [root, Intro, Quick(2)]
 *
 *   #6 "Reference" (1):
 *     Pop while top>=1: pop Quick, pop Intro.  Stack: [root].
 *     Push under root: root.children = [Intro, Reference]
 *     Push Reference: [root, Reference(1)]
 *
 *   #7 "API" (2):
 *     Pop while top>=2: nothing pops.
 *     Push under Reference: Reference.children = [API]
 *     Push API: [root, Reference, API(2)]
 *
 *   Return root.children = [Intro(with subtree), Reference(with subtree)].
 *
 * @module extractor
 */

import type { DocumentNode } from "@coding-adventures/document-ast";
import {
  generateHeadingAnchors,
  type HeadingAnchor,
} from "@coding-adventures/forme-doc-heading-anchors";

import type { TocEntry, TocResult } from "./types.js";

// ─────────────────────────────────────────────────────────────────────────
// Internal mutable type — `TocEntry.children` is `readonly` in the public
// API, but we build it imperatively during the walk.  Cast to the public
// `TocEntry` shape at the boundary.
// ─────────────────────────────────────────────────────────────────────────

interface MutableTocEntry {
  readonly text: string;
  readonly id: string;
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: MutableTocEntry[];
}

// ─────────────────────────────────────────────────────────────────────────
// Public entries
// ─────────────────────────────────────────────────────────────────────────

/**
 * Build a hierarchical TOC tree from a flat in-document-order list of
 * heading anchors.  Pure list-to-tree transformation — no AST needed.
 *
 * Useful when the caller has ALREADY run
 * `generateHeadingAnchors(doc)` and just wants the tree.  For the
 * full one-shot pipeline (anchor + TOC), see `extractToc(doc)` below.
 *
 * @param anchors - In-document-order heading list (as returned by
 *                  `generateHeadingAnchors`).
 * @returns The hierarchical TOC tree.
 */
export function buildTocTree(anchors: readonly HeadingAnchor[]): readonly TocEntry[] {
  // Virtual root with level 0 — every real heading has level >= 1, so
  // root always stays on the stack as the ultimate parent.  Using
  // `Object.create(null)` for parity with our sibling packages'
  // defence-in-depth idiom: no shadowing of any inherited accessor in
  // case a caller's TOC consumer later iterates with `for...in`.
  const root: MutableTocEntry = Object.assign(Object.create(null), {
    text: "",
    id: "",
    level: 0 as unknown as 1 | 2 | 3 | 4 | 5 | 6,
    children: [] as MutableTocEntry[],
  });
  const stack: MutableTocEntry[] = [root];

  for (const a of anchors) {
    // Pop until the stack's top is a strict ancestor of `a`.
    // We use a `while` instead of recursion — both for clarity and to
    // avoid stack overflow on documents with thousands of headings.
    while (stack[stack.length - 1]!.level >= a.level) {
      stack.pop();
    }
    // Build the entry as a null-prototype object — same defence as
    // above, and it keeps the JSON-shape clean.
    const entry: MutableTocEntry = Object.assign(Object.create(null), {
      text: a.text,
      id: a.id,
      level: a.level,
      children: [] as MutableTocEntry[],
    });
    // Splice into the parent's children + push onto the stack as the
    // new deepest open ancestor.
    stack[stack.length - 1]!.children.push(entry);
    stack.push(entry);
  }

  return root.children as readonly TocEntry[];
}

/**
 * Walk a `DocumentNode`, anchor every heading (via
 * `@coding-adventures/forme-doc-heading-anchors`), and build a
 * hierarchical TOC tree from the resulting heading sequence.
 *
 * Returns all three projections so downstream consumers don't have to
 * re-walk:
 *   - `toc`: the hierarchical tree (sidebar, in-page TOC, PDF outline)
 *   - `document`: the anchored AST (HTML renderer reads `heading.id`)
 *   - `anchors`: the flat in-document-order list (sequential consumers)
 *
 * All three share the same slugs / collision suffixes — they're
 * derived from the same single pass.
 *
 * @param doc - The input DocumentNode.
 * @returns `{ toc, document, anchors }`.
 */
export function extractToc(doc: DocumentNode): TocResult {
  // Stage 1: anchor every heading.  Reuses heading-anchors' tested
  // walker — no duplication of slug logic.
  const { document, anchors } = generateHeadingAnchors(doc);
  // Stage 2: nest the flat list by level.
  const toc = buildTocTree(anchors);
  return { toc, document, anchors };
}
