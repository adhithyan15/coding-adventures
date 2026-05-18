/**
 * types.ts — shared types for RSS + Atom generators.
 *
 * Both formats accept the same `FeedItem` shape (translator decides
 * which fields it ships).  `ChannelMeta` is RSS-specific;
 * `FeedMeta` is Atom-specific.
 *
 * @module types
 */

/** A single item in either an RSS `<channel>` or an Atom `<feed>`. */
export interface FeedItem {
  /**
   * Globally-unique, stable identifier for the item.  Used as the
   * Atom `<id>` and as the RSS `<guid>` (with `isPermaLink="false"`
   * unless the id starts with `http://` or `https://`).  Mandatory.
   */
  readonly id: string;
  /** Plain-text title. */
  readonly title: string;
  /** Canonical URL.  Mandatory (Atom feeds without `<link>` are valid
   *  but rare; RSS requires `<link>`). */
  readonly link: string;
  /**
   * Plain-text content body.  RSS: rendered as `<description>` text
   * (escaped).  Atom: rendered as `<content type="text">`.  Mutually
   * exclusive with `contentHtml` — supply ONE per item.  If both are
   * supplied, `contentHtml` wins.
   */
  readonly content?: string;
  /**
   * Pre-rendered HTML content.  RSS: wrapped in CDATA inside
   * `<description>`.  Atom: rendered as `<content type="html">` with
   * the body wrapped in CDATA.
   */
  readonly contentHtml?: string;
  /** Plain-text summary (Atom `<summary>` / RSS often elides). */
  readonly summary?: string;
  /**
   * Item publish date.  Caller supplies as an ISO-8601 string
   * (e.g. `"2026-05-17T10:00:00Z"`).  Atom uses it verbatim;
   * RSS converts to RFC 822 format.
   */
  readonly pubDate?: string;
  /** Author name + optional email. */
  readonly author?: {
    readonly name: string;
    readonly email?: string;
  };
}

/** RSS 2.0 `<channel>` metadata. */
export interface ChannelMeta {
  readonly title: string;
  readonly link: string;
  readonly description: string;
  /** Language code (e.g. `"en-US"`).  Optional. */
  readonly language?: string;
  /** RFC 822 datetime; if absent we omit `<lastBuildDate>`. */
  readonly lastBuildDate?: string;
}

/** Atom 1.0 `<feed>` metadata. */
export interface FeedMeta {
  /** Mandatory Atom `<id>`.  Conventionally the canonical feed URL. */
  readonly id: string;
  readonly title: string;
  /** Atom `<updated>` — ISO 8601 datetime. */
  readonly updated: string;
  /** Canonical feed URL.  Atom recommends `<link rel="self">`. */
  readonly link?: string;
  readonly author?: {
    readonly name: string;
    readonly email?: string;
  };
  readonly subtitle?: string;
}
