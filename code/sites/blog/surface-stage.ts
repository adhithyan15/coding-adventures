/** Build the collection-derived public blog surface: index, feeds, sitemap. */

import {
  Kinds,
  streamOf,
  type Collection,
  type CollectionEntry,
  type LogicalId,
  type PageMeta,
  type RenderedPage,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { renderIndexPage, type IndexItem } from "@coding-adventures/forme-index-renderer";
import { generateRssFeed, generateAtomFeed, type FeedItem } from "@coding-adventures/forme-feeds";
import { generateSitemap } from "@coding-adventures/forme-aot-sitemap-emitter";
import { generateMetaLinkTags } from "@coding-adventures/forme-aot-meta-link-tags";
import { generateFeedDiscoveryLinks } from "@coding-adventures/forme-aot-rss-discovery-link";
import { renderHtmlDocument, publicUrl } from "@coding-adventures/forme-render-static";

export interface BlogSurfaceConfig {
  readonly siteTitle: string;
  readonly siteDescription: string;
  readonly siteUrl: string;
  readonly indexRoute: string;
  readonly rssRoute: string;
  readonly atomRoute: string;
  readonly sitemapRoute: string;
}

const AGGREGATE_SOURCE = "00000000-0000-7000-8000-000000000000" as LogicalId;

const blogSurface = defineStage({
  name: "@coding-adventures/blog-surface",
  version: "0.1.0",
  apiVersion: 1,
  description: "Render a chronological collection as an index, RSS, Atom, and sitemap.",
  consumes: Kinds.Collection,
  produces: streamOf(Kinds.RenderedPage),
  capabilities: [],
  configSchema: {
    type: "object",
    required: [
      "siteTitle", "siteDescription", "siteUrl", "indexRoute",
      "rssRoute", "atomRoute", "sitemapRoute",
    ],
    properties: {
      siteTitle:       { type: "string", minLength: 1 },
      siteDescription: { type: "string", minLength: 1 },
      siteUrl:         { type: "string", minLength: 1 },
      indexRoute:      { type: "string", minLength: 1 },
      rssRoute:        { type: "string", minLength: 1 },
      atomRoute:       { type: "string", minLength: 1 },
      sitemapRoute:    { type: "string", minLength: 1 },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    ctx.cancellation.throwIfCancelled();
    const collection = rawInput as Collection;
    const config = rawConfig as unknown as BlogSurfaceConfig;
    const posts = collection.entries.map((entry) => postFromEntry(entry, config.siteUrl));
    const latestDate = posts[0]?.dateTime ?? "1970-01-01T00:00:00Z";
    const indexUrl = publicUrl(config.siteUrl, config.indexRoute);
    const rssUrl = publicUrl(config.siteUrl, config.rssRoute);
    const atomUrl = publicUrl(config.siteUrl, config.atomRoute);

    const indexItems: IndexItem[] = posts.map((post) => ({
      id: post.id,
      title: post.title,
      url: post.url,
      pubDate: post.date,
      ...(post.excerpt === undefined ? {} : { summary: post.excerpt }),
    }));
    const feedItems: FeedItem[] = posts.map((post) => ({
      id: post.url,
      title: post.title,
      link: post.url,
      pubDate: post.dateTime,
      ...(post.excerpt === undefined ? {} : { summary: post.excerpt, content: post.excerpt }),
    }));
    const discovery = generateFeedDiscoveryLinks([
      { href: rssUrl, type: "application/rss+xml", title: `${config.siteTitle} RSS` },
      { href: atomUrl, type: "application/atom+xml", title: `${config.siteTitle} Atom` },
    ]);
    const indexHead = [
      generateMetaLinkTags({
        canonical: indexUrl,
        meta: [{ name: "description", content: config.siteDescription }],
      }),
      discovery,
    ].join("\n");
    const indexBody = [
      `<h1>${escapeText(config.siteTitle)}</h1>`,
      `<p>${escapeText(config.siteDescription)}</p>`,
      renderIndexPage(indexItems, {
        sortBy: "pubDate-desc",
        showDate: true,
        showSummary: true,
      }),
    ].join("\n");

    yield page(
      config.indexRoute,
      renderHtmlDocument({
        title: config.siteTitle,
        siteTitle: config.siteTitle,
        siteHref: indexUrl,
        headHtml: indexHead,
        bodyHtml: indexBody,
      }),
      meta(config.siteTitle, config.siteDescription, indexUrl),
    ) as never;

    yield page(
      config.rssRoute,
      generateRssFeed({
        title: config.siteTitle,
        link: indexUrl,
        description: config.siteDescription,
        language: "en-US",
        lastBuildDate: latestDate,
      }, feedItems),
      meta(`${config.siteTitle} RSS`, config.siteDescription, rssUrl),
    ) as never;

    yield page(
      config.atomRoute,
      generateAtomFeed({
        id: atomUrl,
        title: config.siteTitle,
        updated: latestDate,
        link: atomUrl,
        subtitle: config.siteDescription,
      }, feedItems),
      meta(`${config.siteTitle} Atom`, config.siteDescription, atomUrl),
    ) as never;

    yield page(
      config.sitemapRoute,
      generateSitemap([
        { url: indexUrl, lastmod: posts[0]?.date, changefreq: "weekly", priority: 1 },
        ...posts.map((post) => ({
          url: post.url,
          lastmod: post.date,
          changefreq: "monthly" as const,
          priority: 0.8,
        })),
      ], config.siteUrl),
      meta(`${config.siteTitle} sitemap`, null, publicUrl(config.siteUrl, config.sitemapRoute)),
    ) as never;

    ctx.logger.debug("blog-surface: rendered collection outputs", {
      posts: posts.length,
      pages: 4,
    });
  },
});

interface SurfacePost {
  readonly id: string;
  readonly title: string;
  readonly url: string;
  readonly date: string;
  readonly dateTime: string;
  readonly excerpt?: string;
}

function postFromEntry(entry: CollectionEntry, siteUrl: string): SurfacePost {
  if (entry.route === null) {
    throw new Error(`Blog surface entry ${entry.identity} has no canonical route`);
  }
  if (entry.orderKey.kind !== "date") {
    throw new Error(`Blog surface entry ${entry.identity} is not date-ordered`);
  }
  const title = entry.overlay.title;
  const excerpt = entry.overlay.excerpt;
  const date = entry.orderKey.value;
  return {
    id: entry.identity,
    title: typeof title === "string" ? title : entry.identity,
    url: publicUrl(siteUrl, entry.route),
    date,
    dateTime: /^\d{4}-\d{2}-\d{2}$/.test(date) ? `${date}T00:00:00Z` : date,
    ...(typeof excerpt === "string" ? { excerpt } : {}),
  };
}

function page(route: string, html: string, pageMeta: PageMeta): RenderedPage {
  return {
    route,
    html,
    usedStyle: [],
    usedIslands: [],
    usedAssets: [],
    meta: pageMeta,
    // The current RenderedPage contract models one source. Collection-derived
    // artifacts need aggregate provenance; FM-B021 tracks that kernel gap.
    source: AGGREGATE_SOURCE,
  };
}

function meta(title: string, description: string | null, canonicalUrl: string): PageMeta {
  return {
    title,
    description,
    canonicalUrl,
    openGraph: {},
    structured: [],
    extra: {},
  };
}

function escapeText(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export default blogSurface;
export { blogSurface, postFromEntry };
