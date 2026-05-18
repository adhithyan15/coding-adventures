/**
 * rss.test.ts — RSS 2.0 generator.
 */

import { describe, it, expect } from "vitest";
import {
  generateRssFeed, type ChannelMeta, type FeedItem,
} from "../src/index.js";

const CHANNEL: ChannelMeta = {
  title: "My Blog",
  link: "https://example.com/",
  description: "Cool stuff happens here",
};

function item(overrides: Partial<FeedItem> & Pick<FeedItem, "id" | "title" | "link">): FeedItem {
  return overrides as FeedItem;
}

describe("generateRssFeed — structure", () => {
  it("emits a valid RSS 2.0 envelope with XML declaration", () => {
    const xml = generateRssFeed(CHANNEL, []);
    expect(xml.startsWith(`<?xml version="1.0" encoding="utf-8"?>\n`)).toBe(true);
    expect(xml).toContain(`<rss version="2.0">`);
    expect(xml).toContain(`<channel>`);
    expect(xml).toContain(`</channel>`);
    expect(xml).toContain(`</rss>`);
  });

  it("includes channel title / link / description", () => {
    const xml = generateRssFeed(CHANNEL, []);
    expect(xml).toContain(`<title>My Blog</title>`);
    expect(xml).toContain(`<link>https://example.com/</link>`);
    expect(xml).toContain(`<description>Cool stuff happens here</description>`);
  });

  it("includes <language> when provided", () => {
    const xml = generateRssFeed({ ...CHANNEL, language: "en-US" }, []);
    expect(xml).toContain(`<language>en-US</language>`);
  });

  it("omits <language> when not provided", () => {
    const xml = generateRssFeed(CHANNEL, []);
    expect(xml).not.toContain(`<language>`);
  });

  it("includes <lastBuildDate> (converted to RFC 822) when provided", () => {
    const xml = generateRssFeed(
      { ...CHANNEL, lastBuildDate: "2026-05-17T00:00:00Z" },
      [],
    );
    expect(xml).toMatch(/<lastBuildDate>Sun, 17 May 2026 00:00:00 \+0000<\/lastBuildDate>/);
  });

  it("passes through RFC 822-shaped lastBuildDate verbatim", () => {
    const xml = generateRssFeed(
      { ...CHANNEL, lastBuildDate: "Mon, 18 May 2026 12:34:56 +0000" },
      [],
    );
    expect(xml).toContain(`<lastBuildDate>Mon, 18 May 2026 12:34:56 +0000</lastBuildDate>`);
  });
});

describe("generateRssFeed — items", () => {
  it("emits <item> with title / link / guid", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "https://example.com/a", title: "Post A", link: "https://example.com/a" }),
    ]);
    expect(xml).toContain(`<title>Post A</title>`);
    expect(xml).toContain(`<link>https://example.com/a</link>`);
    expect(xml).toContain(`<guid isPermaLink="true">https://example.com/a</guid>`);
  });

  it("marks non-URL ids as isPermaLink=false", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "tag:example.com,2026:post-1", title: "x", link: "https://example.com/x" }),
    ]);
    expect(xml).toContain(`<guid isPermaLink="false">tag:example.com,2026:post-1</guid>`);
  });

  it("renders plain `content` as escaped <description>", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "id", title: "t", link: "l", content: "<p>hi & bye</p>" }),
    ]);
    expect(xml).toContain(`<description>&lt;p&gt;hi &amp; bye&lt;/p&gt;</description>`);
  });

  it("renders `contentHtml` wrapped in CDATA", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "id", title: "t", link: "l", contentHtml: "<p>hi</p>" }),
    ]);
    expect(xml).toContain(`<description><![CDATA[<p>hi</p>]]></description>`);
  });

  it("prefers contentHtml when both content and contentHtml are supplied", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({
        id: "id", title: "t", link: "l",
        content: "plain text", contentHtml: "<p>html</p>",
      }),
    ]);
    expect(xml).toContain(`<description><![CDATA[<p>html</p>]]></description>`);
    expect(xml).not.toContain("plain text");
  });

  it("emits <pubDate> in RFC 822 format", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "id", title: "t", link: "l", pubDate: "2026-05-17T00:00:00Z" }),
    ]);
    expect(xml).toMatch(/<pubDate>Sun, 17 May 2026 00:00:00 \+0000<\/pubDate>/);
  });

  it("emits <author> as `email (Name)` when email is present", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({
        id: "id", title: "t", link: "l",
        author: { name: "Jane Doe", email: "jane@example.com" },
      }),
    ]);
    expect(xml).toContain(`<author>jane@example.com (Jane Doe)</author>`);
  });

  it("emits <author> as bare name when email is absent", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "id", title: "t", link: "l", author: { name: "Jane Doe" } }),
    ]);
    expect(xml).toContain(`<author>Jane Doe</author>`);
  });

  it("empty items list still produces a valid empty feed", () => {
    const xml = generateRssFeed(CHANNEL, []);
    expect(xml).not.toContain("<item>");
    expect(xml).toContain("</channel>");
  });
});

describe("generateRssFeed — XML escaping in metadata", () => {
  it("escapes special chars in channel title", () => {
    const xml = generateRssFeed(
      { ...CHANNEL, title: "AT&T <Blog>" },
      [],
    );
    expect(xml).toContain(`<title>AT&amp;T &lt;Blog&gt;</title>`);
  });

  it("escapes special chars in item title", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "id", title: `Quote: "a < b"`, link: "l" }),
    ]);
    expect(xml).toContain(`<title>Quote: &quot;a &lt; b&quot;</title>`);
  });

  it("strips invalid XML chars from title", () => {
    const xml = generateRssFeed(CHANNEL, [
      item({ id: "id", title: "Hello\x00World", link: "l" }),
    ]);
    expect(xml).toContain(`<title>HelloWorld</title>`);
  });
});

describe("generateRssFeed — reproducibility", () => {
  it("same input → byte-identical output", () => {
    const items: FeedItem[] = [
      item({ id: "a", title: "A", link: "l", content: "x" }),
      item({ id: "b", title: "B", link: "l", content: "y" }),
    ];
    expect(generateRssFeed(CHANNEL, items)).toBe(generateRssFeed(CHANNEL, items));
  });
});

describe("generateRssFeed — RFC 822 date passthrough on invalid input", () => {
  it("passes unparseable date through unchanged", () => {
    const xml = generateRssFeed(
      { ...CHANNEL, lastBuildDate: "not a date" },
      [],
    );
    expect(xml).toContain(`<lastBuildDate>not a date</lastBuildDate>`);
  });
});
