/**
 * atom.test.ts — Atom 1.0 generator.
 */

import { describe, it, expect } from "vitest";
import {
  generateAtomFeed, type FeedMeta, type FeedItem,
} from "../src/index.js";

const FEED: FeedMeta = {
  id: "https://example.com/atom.xml",
  title: "My Blog",
  updated: "2026-05-17T00:00:00Z",
};

function item(overrides: Partial<FeedItem> & Pick<FeedItem, "id" | "title" | "link">): FeedItem {
  return overrides as FeedItem;
}

describe("generateAtomFeed — structure", () => {
  it("emits a valid Atom 1.0 envelope with XML declaration and namespace", () => {
    const xml = generateAtomFeed(FEED, []);
    expect(xml.startsWith(`<?xml version="1.0" encoding="utf-8"?>\n`)).toBe(true);
    expect(xml).toContain(`<feed xmlns="http://www.w3.org/2005/Atom">`);
    expect(xml).toContain(`</feed>`);
  });

  it("includes feed-level id / title / updated", () => {
    const xml = generateAtomFeed(FEED, []);
    expect(xml).toContain(`<id>https://example.com/atom.xml</id>`);
    expect(xml).toContain(`<title>My Blog</title>`);
    expect(xml).toContain(`<updated>2026-05-17T00:00:00Z</updated>`);
  });

  it("emits <link rel=\"self\"> when link provided", () => {
    const xml = generateAtomFeed({ ...FEED, link: "https://example.com/atom.xml" }, []);
    expect(xml).toContain(`<link rel="self" href="https://example.com/atom.xml"/>`);
  });

  it("emits <author><name>…</name></author>", () => {
    const xml = generateAtomFeed(
      { ...FEED, author: { name: "Jane" } },
      [],
    );
    expect(xml).toContain(`<author>`);
    expect(xml).toContain(`<name>Jane</name>`);
    expect(xml).toContain(`</author>`);
  });

  it("includes <email> when present in author", () => {
    const xml = generateAtomFeed(
      { ...FEED, author: { name: "Jane", email: "jane@example.com" } },
      [],
    );
    expect(xml).toContain(`<email>jane@example.com</email>`);
  });

  it("includes <subtitle> when provided", () => {
    const xml = generateAtomFeed({ ...FEED, subtitle: "Cool stuff" }, []);
    expect(xml).toContain(`<subtitle>Cool stuff</subtitle>`);
  });
});

describe("generateAtomFeed — entries", () => {
  it("emits <entry> with id / title / link / updated (fallback to feed.updated)", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "https://example.com/a", title: "Post A", link: "https://example.com/a" }),
    ]);
    expect(xml).toContain(`<entry>`);
    expect(xml).toContain(`<id>https://example.com/a</id>`);
    expect(xml).toContain(`<title>Post A</title>`);
    expect(xml).toContain(`<link href="https://example.com/a"/>`);
    // Per RFC 4287 §4.2.15: <updated> mandatory; we fall back to feed.updated.
    expect(xml).toContain(`<updated>2026-05-17T00:00:00Z</updated>`);
  });

  it("uses item.pubDate as <updated> when supplied", () => {
    const xml = generateAtomFeed(FEED, [
      item({
        id: "a", title: "A", link: "l",
        pubDate: "2026-04-01T12:00:00Z",
      }),
    ]);
    // The entry's updated is the pubDate, NOT the feed-level one.
    expect(xml).toContain(`    <updated>2026-04-01T12:00:00Z</updated>`);
  });

  it("renders plain content as <content type=\"text\">", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "t", link: "l", content: "plain & text" }),
    ]);
    expect(xml).toContain(`<content type="text">plain &amp; text</content>`);
  });

  it("renders contentHtml as <content type=\"html\"> in CDATA", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "t", link: "l", contentHtml: "<p>hi</p>" }),
    ]);
    expect(xml).toContain(`<content type="html"><![CDATA[<p>hi</p>]]></content>`);
  });

  it("prefers contentHtml when both supplied", () => {
    const xml = generateAtomFeed(FEED, [
      item({
        id: "id", title: "t", link: "l",
        content: "plain", contentHtml: "<p>html</p>",
      }),
    ]);
    expect(xml).toContain(`<content type="html">`);
    expect(xml).not.toContain(`plain</content>`);
  });

  it("renders <summary> when provided", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "t", link: "l", summary: "TL;DR" }),
    ]);
    expect(xml).toContain(`<summary>TL;DR</summary>`);
  });

  it("renders per-entry author block when provided", () => {
    const xml = generateAtomFeed(FEED, [
      item({
        id: "id", title: "t", link: "l",
        author: { name: "Alice", email: "alice@example.com" },
      }),
    ]);
    expect(xml).toContain(`<name>Alice</name>`);
    expect(xml).toContain(`<email>alice@example.com</email>`);
  });

  it("renders per-entry author with name only", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "t", link: "l", author: { name: "Bob" } }),
    ]);
    expect(xml).toContain(`<name>Bob</name>`);
    // No email element when email is undefined.
    expect(xml).not.toMatch(/<email>.*Bob/);
  });

  it("empty items list produces valid empty feed", () => {
    const xml = generateAtomFeed(FEED, []);
    expect(xml).not.toContain("<entry>");
    expect(xml).toContain("</feed>");
  });
});

describe("generateAtomFeed — XML escaping", () => {
  it("escapes feed title", () => {
    const xml = generateAtomFeed({ ...FEED, title: "AT&T \"Atom\"" }, []);
    expect(xml).toContain(`<title>AT&amp;T &quot;Atom&quot;</title>`);
  });

  it("escapes entry link attribute value", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "t", link: `https://example.com/?a=1&b=2` }),
    ]);
    expect(xml).toContain(`<link href="https://example.com/?a=1&amp;b=2"/>`);
  });

  it("strips invalid XML chars from entry title", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "Hello\x00World", link: "l" }),
    ]);
    expect(xml).toContain(`<title>HelloWorld</title>`);
  });
});

describe("generateAtomFeed — reproducibility", () => {
  it("same input → byte-identical output", () => {
    const items: FeedItem[] = [
      item({ id: "a", title: "A", link: "l", content: "x" }),
      item({ id: "b", title: "B", link: "l", contentHtml: "<p>y</p>" }),
    ];
    expect(generateAtomFeed(FEED, items)).toBe(generateAtomFeed(FEED, items));
  });
});

describe("generateAtomFeed — CDATA termination defence", () => {
  it("contentHtml containing `]]>` is split across CDATA boundaries", () => {
    const xml = generateAtomFeed(FEED, [
      item({ id: "id", title: "t", link: "l", contentHtml: `<script>x = a]]>b;</script>` }),
    ]);
    // The literal `]]>` does NOT appear inside the CDATA payload.
    expect(xml).toContain(`]]]]><![CDATA[>`);
  });
});
