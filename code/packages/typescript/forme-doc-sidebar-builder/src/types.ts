/**
 * types.ts — public signatures for the sidebar builder.
 *
 * @module types
 */

/**
 * One input page — what the caller already has after running the
 * `commonmark-parser` (for the body) and `forme-doc-frontmatter` (for
 * the metadata) on every `.md` file under the site root.
 *
 * The sidebar builder only needs the PATH and the FRONTMATTER — it
 * never sees the body content.  That keeps it cheap to re-run when
 * adding/removing pages (no need to re-parse every file).
 */
export interface PageInput {
  /**
   * The page's path, relative to the site root.  May be a file
   * path (`"guide/setup.md"`), a URL path (`"/guide/setup"`), or
   * an extensionless slug (`"guide/setup"`).  The builder
   * normalises by stripping leading `/`, trailing `.md` / `.mdx` /
   * `.html`, and trailing `/index` / `/index.md` etc. before
   * building the directory tree.
   *
   * Empty string is rejected (throws `TypeError`) — every page
   * needs a place in the tree.
   */
  readonly path: string;

  /**
   * The parsed frontmatter object — any string-keyed `unknown`
   * map.  The builder reads four well-known keys (others are
   * ignored):
   *
   *   - `title: string` — overrides the auto-derived label.
   *   - `sidebar_label: string` — alternative to `title`, used
   *     only for the sidebar entry (not for `<title>` etc.).
   *     Takes precedence over `title` when both are present.
   *   - `sidebar_position: number` — sort key within the
   *     directory (ascending).  Missing or non-numeric →
   *     `+Infinity` (alphabetical fallback).
   *   - `draft: boolean` — when `true`, the page is omitted
   *     from the sidebar entirely (and any group that becomes
   *     empty as a result is also omitted).
   *
   * Frontmatter is supplied as `Record<string, unknown>` because
   * `forme-doc-frontmatter` returns null-prototype objects with
   * arbitrary user-defined keys.  We read only the four above
   * defensively — unknown keys never affect output.
   */
  readonly frontmatter: Readonly<Record<string, unknown>>;
}

/**
 * Options for `buildSidebar`.
 */
export interface BuildSidebarOptions {
  /**
   * Optional prefix to strip from every page's normalised path
   * before building the tree.  Useful when the docs live under a
   * subdirectory of a larger site (e.g. `root: "docs"` for paths
   * like `"docs/guide/setup.md"`).
   *
   * The prefix is matched after path normalisation (leading `/`,
   * trailing extensions, etc. already removed).  Pages whose
   * normalised path doesn't start with the prefix are silently
   * skipped — the assumption being the caller passed a wider
   * input set and only wants this subset in the sidebar.
   *
   * Default: `""` (no prefix stripping).
   */
  readonly root?: string;
}

/**
 * A single navigable page entry in the sidebar tree.
 */
export interface SidebarPageEntry {
  readonly kind: "page";
  /** Display label — `sidebar_label` ?? `title` ?? humanised filename. */
  readonly label: string;
  /** The original (non-normalised) path from the input. */
  readonly path: string;
  /**
   * The numeric `sidebar_position` from frontmatter, or `null` if
   * absent / non-numeric.  Exposed so renderers can show a
   * position badge or assert on ordering.
   */
  readonly position: number | null;
}

/**
 * A directory grouping containing pages and/or nested groups.
 *
 * `path` is set iff the directory has an `index.md` (or
 * `index.mdx` / `index.html`) page.  Sidebar widgets typically
 * make the group label clickable in that case — clicking
 * navigates to the index page.
 */
export interface SidebarGroupEntry {
  readonly kind: "group";
  /** Humanised directory name, or `title` override if the group has an index page with a `title`. */
  readonly label: string;
  /** Original path of the index page, or `null` if the directory has no index. */
  readonly path: string | null;
  /**
   * The numeric `sidebar_position` from the GROUP itself — taken
   * from the index page's frontmatter, or `null` if no index page
   * or the index has no `sidebar_position`.  Used by the parent
   * directory to order this group among its siblings.
   */
  readonly position: number | null;
  /** Pages and sub-groups within this directory. */
  readonly children: readonly SidebarEntry[];
}

/** Union of the two entry kinds. */
export type SidebarEntry = SidebarPageEntry | SidebarGroupEntry;
