import { describe, expect, it } from "vitest";

import {
  AuthoringError,
  HARD_AUTHORING_LIMITS,
  canonicalAuthoringProject,
  createAuthoringProject,
  validateAuthoringProject,
  type AuthoringProject,
} from "../src/index.js";

const PROJECT_ID = "01952c0d-7e63-7000-8000-000000000001";
const DOCUMENT_ID = "01952c0d-7e63-7000-8000-000000000002";

function project(overrides: Partial<AuthoringProject> = {}): AuthoringProject {
  return {
    schemaVersion: 1,
    projectId: PROJECT_ID,
    title: "My site",
    site: { baseUrl: "https://example.com/blog", themeId: "forme-classless" },
    workflow: { lastPublication: null },
    documents: [
      {
        id: DOCUMENT_ID,
        slug: "hello-world",
        title: "Hello, world",
        status: "draft",
        body: {
          type: "document",
          children: [
            {
              type: "heading",
              level: 1,
              children: [{ type: "text", value: "Hello" }],
            },
            {
              type: "paragraph",
              children: [
                { type: "text", value: "A " },
                { type: "strong", children: [{ type: "text", value: "safe" }] },
                { type: "text", value: " link: " },
                {
                  type: "link",
                  destination: "/about",
                  title: null,
                  children: [{ type: "text", value: "about" }],
                },
              ],
            },
          ],
        },
      },
    ],
    activeDocumentId: DOCUMENT_ID,
    ...overrides,
  };
}

function invalid(value: unknown, code = "INVALID_PROJECT"): void {
  try {
    validateAuthoringProject(value);
    throw new Error("expected validation to fail");
  } catch (error) {
    expect(error).toBeInstanceOf(AuthoringError);
    expect((error as AuthoringError).code).toBe(code);
  }
}

describe("the authoring project codec", () => {
  it("validates, clones, and deeply freezes a complete project", () => {
    const source = project();
    const validated = validateAuthoringProject(source);

    expect(validated).toEqual(source);
    expect(validated).not.toBe(source);
    expect(Object.isFrozen(validated)).toBe(true);
    expect(Object.isFrozen(validated.documents)).toBe(true);
    expect(Object.isFrozen(validated.documents[0]!.body.children)).toBe(true);
  });

  it("creates the smallest valid first-run project", () => {
    expect(createAuthoringProject({ projectId: PROJECT_ID, title: "New site" })).toEqual({
      schemaVersion: 1,
      projectId: PROJECT_ID,
      title: "New site",
      site: { baseUrl: null, themeId: "forme-classless" },
      workflow: { lastPublication: null },
      documents: [],
      activeDocumentId: null,
    });
  });

  it("serializes recursively sorted canonical JSON", () => {
    const first = canonicalAuthoringProject(project());
    const second = canonicalAuthoringProject({
      activeDocumentId: DOCUMENT_ID,
      documents: project().documents,
      site: { themeId: "forme-classless", baseUrl: "https://example.com/blog" },
      workflow: { lastPublication: null },
      title: "My site",
      projectId: PROJECT_ID,
      schemaVersion: 1,
    });

    expect(first).toBe(second);
    expect(first.startsWith('{"activeDocumentId"')).toBe(true);
    expect(first).not.toContain("\n");
  });

  it("accepts the complete authorable block and inline vocabulary", () => {
    const rich = project({
      documents: [{
        ...project().documents[0]!,
        body: {
          type: "document",
          children: [
            { type: "blockquote", children: [{ type: "paragraph", children: [{ type: "emphasis", children: [{ type: "text", value: "quoted" }] }] }] },
            { type: "list", ordered: true, start: 3, tight: false, children: [
              { type: "list_item", children: [{ type: "paragraph", children: [{ type: "strikethrough", children: [{ type: "text", value: "old" }] }] }] },
              { type: "task_item", checked: true, children: [{ type: "paragraph", children: [{ type: "code_span", value: "done" }] }] },
            ] },
            { type: "code_block", language: "ts", value: "const x = 1;\n" },
            { type: "thematic_break" },
            { type: "table", align: ["left"], children: [
              { type: "table_row", isHeader: true, children: [
                { type: "table_cell", children: [{ type: "text", value: "Name" }] },
              ] },
            ] },
            { type: "paragraph", children: [
              { type: "image", destination: "images/cat.png", title: "Cat", alt: "cat" },
              { type: "soft_break" },
              { type: "autolink", destination: "https://example.com", isEmail: false },
              { type: "autolink", destination: "hello@example.com", isEmail: true },
              { type: "hard_break" },
            ] },
          ],
        },
      }],
    });

    expect(validateAuthoringProject(rich)).toEqual(rich);
  });

  it("rejects non-plain objects, accessors, sparse arrays, and unknown fields", () => {
    invalid(Object.create({ schemaVersion: 1 }));

    const accessor = project() as unknown as Record<string, unknown>;
    Object.defineProperty(accessor, "title", { enumerable: true, get: () => "trap" });
    invalid(accessor);

    const sparse = project();
    const documents = new Array(1);
    invalid({ ...sparse, documents });
    invalid({ ...project(), surprise: true });
    invalid({ ...project(), site: { ...project().site, token: "secret" } });

    let reads = 0;
    const text = new Proxy({ type: "text", value: "safe" }, {
      get(target, key, receiver) { reads += 1; return Reflect.get(target, key, receiver); },
    });
    const proxied = project({
      documents: [{ ...project().documents[0]!, body: { type: "document", children: [{ type: "paragraph", children: [text] }] } }],
    });
    expect(validateAuthoringProject(proxied).documents[0]!.body.children).toHaveLength(1);
    expect(reads).toBe(0);
  });

  it("rejects malformed identities, strings, slugs, URLs, and references", () => {
    invalid({ ...project(), projectId: "not-a-uuid" });
    invalid({ ...project(), title: " padded " });
    invalid({ ...project(), title: "bad\u0007title" });
    invalid({ ...project(), title: "bad\ud800title" });
    invalid({ ...project(), title: "bad\udc00title" });
    invalid({ ...project(), title: 42 });
    invalid({ ...project(), title: "" });
    invalid({ ...project(), site: { ...project().site, baseUrl: "javascript:alert(1)" } });
    invalid({ ...project(), site: { ...project().site, baseUrl: "/relative" } });
    invalid({ ...project(), site: { ...project().site, baseUrl: "https://user:pass@example.com" } });
    invalid({ ...project(), site: { ...project().site, themeId: "../theme" } });
    invalid({ ...project(), activeDocumentId: "01952c0d-7e63-7000-8000-000000000099" });
    invalid({ ...project(), documents: [{ ...project().documents[0]!, slug: "Not Portable" }] });
  });

  it("rejects duplicate document identities and slugs", () => {
    const original = project().documents[0]!;
    invalid({ ...project(), documents: [original, { ...original, slug: "other" }] });
    invalid({ ...project(), documents: [original, { ...original, id: "01952c0d-7e63-7000-8000-000000000003" }] });
  });

  it("validates, freezes, and bounds exact publication workflow metadata", () => {
    const published = validateAuthoringProject({
      ...project(),
      workflow: {
        lastPublication: {
          authoringRevision: "revision-42",
          manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
          targetId: "github-pages",
        },
      },
    });
    expect(published.workflow.lastPublication).toEqual({
      authoringRevision: "revision-42",
      manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      targetId: "github-pages",
    });
    expect(Object.isFrozen(published.workflow)).toBe(true);
    expect(Object.isFrozen(published.workflow.lastPublication)).toBe(true);

    invalid({ ...project(), workflow: {} });
    invalid({ ...project(), workflow: { lastPublication: { authoringRevision: "revision", manifestSha256: "bad", targetId: "github-pages" } } });
    invalid({ ...project(), workflow: { lastPublication: { authoringRevision: "revision", manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB=", targetId: "github-pages" } } });
    invalid({ ...project(), workflow: { lastPublication: { authoringRevision: "revision", manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", targetId: "../secret" } } });
    invalid({ ...project(), workflow: { lastPublication: { authoringRevision: "bad\u202erevision", manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", targetId: "github-pages" } } });
    invalid({ ...project(), workflow: { lastPublication: { authoringRevision: "revision", manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", targetId: "github-pages", token: "secret" } } });
  });

  it("rejects raw nodes, unsafe links, malformed trees, and unknown nodes", () => {
    const withBody = (body: unknown) => ({
      ...project(),
      documents: [{ ...project().documents[0]!, body }],
    });
    invalid(withBody({ type: "document", children: [{ type: "raw_block", format: "html", value: "<script>" }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "raw_inline", format: "html", value: "x" }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "link", destination: "data:text/html,x", title: null, children: [] }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "link", destination: "//example.com", title: null, children: [] }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "image", destination: "images\\cat.png", title: null, alt: "cat" }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "autolink", destination: "not-an-email", isEmail: true }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "autolink", destination: "https://example.com", isEmail: "no" }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [null] }] }));
    invalid(withBody({ type: "document", children: [{ type: "paragraph", children: [{ type: "link", destination: "/a", title: null, children: [{ type: "link", destination: "/b", title: null, children: [] }] }] }] }));
    invalid(withBody({ type: "document", children: [{ type: "heading", level: 7, children: [] }] }));
    invalid(withBody({ type: "document", children: [null] }));
    invalid(withBody({ type: "document", children: [{ type: "document", children: [] }] }));
    invalid(withBody({ type: "document", children: [{ type: "mystery" }] }));
    invalid(withBody({ type: "paragraph", children: [] }));
  });

  it("rejects malformed list, code, table, status, and shared-tree shapes", () => {
    const withBlock = (block: unknown) => ({
      ...project(),
      documents: [{ ...project().documents[0]!, body: { type: "document", children: [block] } }],
    });
    invalid(withBlock({ type: "code_block", language: null, value: "missing newline" }));
    invalid(withBlock({ type: "task_item", checked: "yes", children: [] }));
    invalid(withBlock({ type: "list", ordered: "yes", start: 1, tight: true, children: [] }));
    invalid(withBlock({ type: "list", ordered: true, start: 0, tight: true, children: [] }));
    invalid(withBlock({ type: "list", ordered: false, start: 1, tight: true, children: [] }));
    invalid(withBlock({ type: "list", ordered: false, start: null, tight: "yes", children: [] }));
    invalid(withBlock({ type: "list", ordered: false, start: null, tight: true, children: [{ type: "paragraph", children: [] }] }));
    invalid(withBlock({ type: "table", align: ["diagonal"], children: [] }));
    invalid(withBlock({ type: "table", align: ["left"], children: [{ type: "paragraph", isHeader: true, children: [] }] }));
    invalid(withBlock({ type: "table", align: ["left"], children: [{ type: "table_row", isHeader: "yes", children: [] }] }));
    invalid(withBlock({ type: "table", align: ["left"], children: [{ type: "table_row", isHeader: true, children: [{ type: "paragraph", children: [] }] }] }));
    invalid(withBlock({ type: "table", align: ["left", "right"], children: [{ type: "table_row", isHeader: true, children: [{ type: "table_cell", children: [] }] }] }));
    invalid({ ...project(), documents: [{ ...project().documents[0]!, status: "archived" }] });

    const shared = { type: "text", value: "shared" };
    invalid(withBlock({ type: "paragraph", children: [shared, shared] }));
    const cycle: unknown[] = [];
    cycle.push(cycle);
    invalid(withBlock({ type: "paragraph", children: cycle }));
  });

  it("enforces caller-lowered document, depth, node, string, URL, and byte limits", () => {
    invalid({ ...project(), documents: [] }, "INVALID_PROJECT");
    expect(() => validateAuthoringProject(project(), { maxDocuments: 0 })).toThrow(/documents/i);
    expect(() => validateAuthoringProject(project(), { maxDepth: 1 })).toThrow(/depth/i);
    expect(() => validateAuthoringProject(project(), { maxNodesPerDocument: 2 })).toThrow(/node/i);
    expect(() => validateAuthoringProject(project({ title: "abcd" }), { maxTitleScalars: 3 })).toThrow(/title/i);
    expect(() => validateAuthoringProject(project(), { maxUrlScalars: 5 })).toThrow(/url|destination/i);
    expect(() => validateAuthoringProject(project(), { maxJsonBytes: 16 })).toThrow(/bytes/i);
  });

  it("rejects limits above the hard contract", () => {
    expect(() => validateAuthoringProject(project(), {
      maxDocuments: HARD_AUTHORING_LIMITS.maxDocuments + 1,
    })).toThrow(/limit/i);
    expect(() => validateAuthoringProject(project(), null as never)).toThrow(/plain object/i);
    expect(() => validateAuthoringProject(project(), { surprise: 1 } as never)).toThrow(/unknown field/i);
  });

  it("bounds hostile descriptor and canonical-byte preflight shapes", () => {
    const withSymbol = project() as unknown as Record<PropertyKey, unknown>;
    withSymbol[Symbol("hidden")] = true;
    invalid(withSymbol);

    const hidden = project() as unknown as Record<string, unknown>;
    Object.defineProperty(hidden, "hidden", { value: true });
    invalid(hidden);

    invalid({ ...project(), title: (() => "not JSON") as never });
    invalid({ ...project(), schemaVersion: Number.NaN });
    invalid({ ...project(), title: "line\nbreak" });

    const decorated = project().documents.slice() as unknown[] & { extra?: boolean };
    decorated.extra = true;
    invalid({ ...project(), documents: decorated });

    const revoked = Proxy.revocable(project(), {});
    revoked.revoke();
    invalid(revoked.proxy);
    invalid(new Proxy(project(), { ownKeys() { throw new Error("trap"); } }));
    invalid(new Proxy(project(), { getOwnPropertyDescriptor() { throw new Error("trap"); } }));
    invalid(new Proxy({}, {
      ownKeys() { return ["schemaVersion"]; },
      getOwnPropertyDescriptor() { return undefined; },
    }));
    invalid({ ...project(), documents: new Proxy([], {
      getOwnPropertyDescriptor(target, key) {
        if (key === "length") throw new Error("trap");
        return Reflect.getOwnPropertyDescriptor(target, key);
      },
    }) });

    let nested: unknown = { value: true };
    for (let index = 0; index < 150; index += 1) nested = { nested };
    invalid({ ...project(), extra: nested });

    const unicode = project({ title: "A\"\\©€😀" });
    expect(validateAuthoringProject(unicode).title).toBe("A\"\\©€😀");
    expect(() => validateAuthoringProject(project({ title: "x".repeat(300) }), { maxJsonBytes: 200 })).toThrow(/bytes/i);

    const input = {} as Record<string, unknown>;
    Object.defineProperty(input, "projectId", { enumerable: true, value: PROJECT_ID });
    Object.defineProperty(input, "title", { enumerable: true, get: () => "trap" });
    expect(() => createAuthoringProject(input as never)).toThrow(/accessor/i);

    const limitAccessor = {} as Record<string, unknown>;
    Object.defineProperty(limitAccessor, "maxDepth", { enumerable: true, get: () => 1 });
    expect(() => validateAuthoringProject(project(), limitAccessor as never)).toThrow(/enumerable data/i);
    const revokedLimits = Proxy.revocable({}, {});
    revokedLimits.revoke();
    expect(() => validateAuthoringProject(project(), revokedLimits.proxy)).toThrow(/inspected safely/i);
    expect(() => validateAuthoringProject(project(), Object.create(null))).toThrow(/plain string-keyed/i);
    expect(() => validateAuthoringProject(project(), new Proxy({ maxDepth: 1 }, {
      getOwnPropertyDescriptor() { throw new Error("trap"); },
    }))).toThrow(/inspected safely/i);
    expect(() => createAuthoringProject(null as never)).toThrow(/plain object/i);
    expect(() => createAuthoringProject({ projectId: PROJECT_ID } as never)).toThrow(/missing/i);
    invalid({ ...project(), documents: [{ ...project().documents[0]!, body: { type: "document", children: [{ type: "paragraph", children: [{ type: "mystery" }] }] } }] });
  });
});
