/**
 * types.ts — meta-record types for the three generators.
 *
 * @module types
 */

/**
 * OpenGraph metadata per https://ogp.me/.  The four required
 * fields (title, type, image, url) are mandatory; the rest are
 * optional.  Image/url/video MUST be absolute http(s) URLs —
 * `generateOpenGraphTags` throws TypeError on relative paths
 * (Facebook's scraper requires absolute) or `javascript:`/`data:`
 * schemes (injection vectors).
 */
export interface OpenGraphMeta {
  /** Page / object title. */
  readonly title: string;
  /** `og:type` — `"website"`, `"article"`, `"video.movie"`, etc. */
  readonly type: string;
  /** Cover image URL.  MUST be absolute http(s). */
  readonly image: string;
  /** Canonical object URL.  MUST be absolute http(s). */
  readonly url: string;
  readonly description?: string;
  /** `og:site_name`. */
  readonly siteName?: string;
  /** `og:locale` — e.g. `"en_US"`. */
  readonly locale?: string;
  /** `og:video` URL.  MUST be absolute http(s) if supplied. */
  readonly video?: string;
}

/**
 * Twitter Card metadata per
 * https://developer.twitter.com/en/docs/twitter-for-websites/cards/overview/markup.
 * Only `card` is mandatory; the rest fall back to og:* tags when
 * a Twitter-specific value isn't supplied (the consumer's choice —
 * `generateTwitterCardTags` only emits what you give it).
 */
export interface TwitterCardMeta {
  /** Twitter card type. */
  readonly card: "summary" | "summary_large_image" | "player" | "app";
  readonly title?: string;
  readonly description?: string;
  /** Image URL.  MUST be absolute http(s) if supplied. */
  readonly image?: string;
  /** `@site` Twitter handle for the publishing site. */
  readonly site?: string;
  /** `@creator` Twitter handle for the content author. */
  readonly creator?: string;
}

/**
 * Basic HTML head metadata — `<title>`, `<meta name="description">`,
 * `<link rel="canonical">`.  Independent of OpenGraph / Twitter
 * because a page may want all three concerns served from one place
 * (and the basic tags drive search-engine snippets, not just
 * social previews).  Canonical URL MUST be absolute http(s).
 */
export interface BasicMeta {
  readonly title?: string;
  readonly description?: string;
  readonly canonical?: string;
}
