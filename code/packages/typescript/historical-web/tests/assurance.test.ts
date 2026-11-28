import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, it } from "node:test";
import {
  decodeHttp09Response,
  encodeHttp09Request,
  Http09Address,
  parseHtmlTags1992,
  parseHttp09Request,
} from "../src/index.ts";
import { CoverageBitmap, fuzzTarget } from "@coding-adventures/test-bootstrap/fuzz";
import type { FuzzOutcome } from "@coding-adventures/test-bootstrap/fuzz";

const here = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(here, "../../../../specs/fixtures/historical-web-v1/vectors.json");

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type ArenaNode =
  | { kind: "text"; text: string }
  | { kind: "element"; tag: string; attributes: Record<string, string | null>; children: number[] };
type ArenaDocument = { nodes: ArenaNode[]; roots: number[] };
type Fixture = {
  "http-0.9": {
    requests: { address: string; wire_hex: string }[];
    invalid_addresses: string[];
    invalid_request_hex: string[];
    responses: { wire_hex: string; body: string }[];
    invalid_response_hex: string[];
  };
  "html-tags-1992": { id: string; source: string; tree: Json[] }[];
  exchanges: { address: string; request_hex: string; response_hex: string; tree: Json[] }[];
};

function readFixture(): Fixture {
  const parsed: unknown = JSON.parse(readFileSync(fixturePath, "utf8"));
  assert.equal(typeof parsed, "object");
  assert.ok(parsed !== null);
  const fixture = parsed as Fixture;
  assert.ok(Array.isArray(fixture["http-0.9"]?.requests) && fixture["http-0.9"].requests.length > 0);
  assert.ok(Array.isArray(fixture["http-0.9"]?.responses) && fixture["http-0.9"].responses.length > 0);
  assert.ok(Array.isArray(fixture["html-tags-1992"]) && fixture["html-tags-1992"].length > 0);
  assert.ok(Array.isArray(fixture.exchanges) && fixture.exchanges.length > 0);
  return fixture;
}

function hexBytes(hex: string): Uint8Array {
  assert.equal(hex.length % 2, 0, `odd-length fixture hex: ${hex}`);
  assert.match(hex, /^(?:[0-9a-fA-F]{2})*$/, `invalid fixture hex: ${hex}`);
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) bytes[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return bytes;
}

function mustValue<T, E>(result: { ok: true; value: T } | { ok: false; error: E }, label: string): T {
  if (!result.ok) throw new Error(`${label}: ${JSON.stringify(result.error)}`);
  return result.value;
}

function children(node: { kind: string; children?: number[] }): number[] {
  return node.kind === "element" ? (node.children ?? []) : [];
}

function assertArenaWellFormed(doc: ArenaDocument): void {
  const owners = new Array<number>(doc.nodes.length).fill(0);
  const seen = new Array<boolean>(doc.nodes.length).fill(false);
  const queue = [...doc.roots];
  for (const root of doc.roots) {
    assert.ok(Number.isInteger(root) && root >= 0 && root < doc.nodes.length, "root index bounds");
    owners[root]++;
  }
  for (let head = 0; head < queue.length; head++) {
    const id = queue[head];
    assert.ok(!seen[id], `cycle or multiply reached node ${id}`);
    seen[id] = true;
    const node = doc.nodes[id];
    assert.ok(node && (node.kind === "text" || node.kind === "element"), `unknown node kind at ${id}`);
    if (node.kind === "text") {
      assert.equal(typeof node.text, "string");
    } else {
      assert.ok(node.tag.length > 0);
      assert.equal(node.tag, node.tag.toLowerCase(), `non-lowercase tag at ${id}`);
      assert.ok(node.attributes && typeof node.attributes === "object");
      assert.ok(!Array.isArray(node.attributes));
      for (const [name, value] of Object.entries(node.attributes)) {
        assert.ok(node.tag === "a" && ["name", "href", "type"].includes(name), `unrecognized attribute ${name}`);
        assert.ok(value === null || typeof value === "string", `invalid attribute value for ${name}`);
      }
    }
    for (const child of children(node)) {
      assert.ok(Number.isInteger(child) && child >= 0 && child < doc.nodes.length, "child index bounds");
      owners[child]++;
      queue.push(child);
    }
  }
  assert.ok(seen.every(Boolean), "all arena nodes must be reachable");
  assert.ok(owners.every((count) => count === 1), "each node must have exactly one owner");
}

function nodeJson(doc: ArenaDocument, id: number): Json {
  const node = doc.nodes[id];
  if (node.kind === "text") return node.text;
  const childrenJson = node.children.map((child: number) => nodeJson(doc, child));
  return { tag: node.tag, attributes: { ...node.attributes }, children: childrenJson };
}

function rootsJson(doc: ArenaDocument): Json[] {
  return doc.roots.map((root) => nodeJson(doc, root));
}

function wireOracle(wire: Uint8Array): Uint8Array | null {
  if (wire.length < 6 || wire[0] !== 0x47 || wire[1] !== 0x45 || wire[2] !== 0x54 || wire[3] !== 0x20 || wire[wire.length - 1] !== 0x0a) return null;
  let end = wire.length - 1;
  // A trailing CR immediately before LF is the optional CRLF framing byte,
  // including when the caller supplied a bare LF after a payload CR.
  if (end > 0 && wire[end - 1] === 0x0d) end--;
  const address = wire.slice(4, end);
  if (address.length === 0 || address.some((byte) => byte < 0x21 || byte > 0x7e)) return null;
  return address;
}

function seedRand(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
  };
}

describe("historical-web independent assurance", () => {
  it("replays every shared HTTP, HTML, and exchange vector fail-closed", () => {
    const v = readFixture();
    const http = v["http-0.9"];
    for (const tc of http.requests) {
      const wire = hexBytes(tc.wire_hex);
      const address = mustValue(Http09Address.create(tc.address), "valid address");
      const canonical = new TextEncoder().encode(`GET ${tc.address}\r\n`);
      assert.deepEqual(encodeHttp09Request(address), canonical, `canonical encoder output for ${tc.address}`);
      assert.deepEqual(wire, new TextEncoder().encode(`GET ${tc.address}${tc.wire_hex.endsWith("0a") && !tc.wire_hex.endsWith("0d0a") ? "\n" : "\r\n"}`));
      assert.equal(mustValue(parseHttp09Request(wire), `request ${tc.wire_hex}`).value, tc.address);
    }
    for (const address of http.invalid_addresses) {
      const result = Http09Address.create(address);
      assert.equal(result.ok, false, `accepted invalid address ${JSON.stringify(address)}`);
      if (!result.ok) assert.equal(result.error.kind, "invalid-address");
    }
    for (const raw of http.invalid_request_hex) {
      const result = parseHttp09Request(hexBytes(raw));
      assert.equal(result.ok, false, `accepted invalid request ${raw}`);
      if (!result.ok) assert.equal(result.error.kind, "invalid-request");
    }
    for (const tc of http.responses) {
      assert.equal(mustValue(decodeHttp09Response(hexBytes(tc.wire_hex)), "response"), tc.body);
    }
    for (const raw of http.invalid_response_hex) {
      const result = decodeHttp09Response(hexBytes(raw));
      assert.equal(result.ok, false, `accepted non-ASCII response ${raw}`);
      if (!result.ok) assert.equal(result.error.kind, "non-ascii");
    }
    for (const tc of v["html-tags-1992"]) {
      const doc = mustValue(parseHtmlTags1992(tc.source), `HTML ${tc.id}`);
      assertArenaWellFormed(doc);
      assert.deepEqual(rootsJson(doc), tc.tree, tc.id);
    }
    for (const tc of v.exchanges) {
      assert.equal(mustValue(parseHttp09Request(hexBytes(tc.request_hex)), "exchange request").value, tc.address);
      const body = mustValue(decodeHttp09Response(hexBytes(tc.response_hex)), "exchange response");
      const doc = mustValue(parseHtmlTags1992(body), "exchange HTML");
      assertArenaWellFormed(doc);
      assert.deepEqual(rootsJson(doc), tc.tree, tc.address);
    }
  });

  it("exhaustively checks 131,586 short HTTP payload and framing pairs", () => {
    let cases = 0;
    for (let length = 0; length <= 2; length++) {
      const count = 256 ** length;
      for (let value = 0; value < count; value++) {
        const payload = new Uint8Array(length);
        for (let i = 0; i < length; i++) payload[i] = (value >>> (8 * i)) & 0xff;
        for (const ending of [Uint8Array.of(0x0a), Uint8Array.of(0x0d, 0x0a)]) {
          const wire = new Uint8Array(4 + payload.length + ending.length);
          wire.set([0x47, 0x45, 0x54, 0x20]);
          wire.set(payload, 4);
          wire.set(ending, 4 + payload.length);
          const expected = wireOracle(wire);
          const actual = parseHttp09Request(wire);
          if (expected === null) {
            assert.equal(actual.ok, false, `accepted ${Array.from(wire)}`);
          } else {
            assert.equal(actual.ok, true, `rejected ${Array.from(wire)}`);
            if (actual.ok) assert.deepEqual(new TextEncoder().encode(actual.value.value), expected);
          }
          cases++;
        }
      }
    }
    assert.equal(cases, 131_586);
  });

  it("checks deterministic request properties and response identity", () => {
    const random = seedRand(0x19921103);
    for (let trial = 0; trial < 1_000; trial++) {
      const size = 1 + (random() % 64);
      let address = "";
      for (let i = 0; i < size; i++) address += String.fromCharCode(0x21 + (random() % 0x5e));
      const value = mustValue(Http09Address.create(address), "generated printable address");
      const wire = encodeHttp09Request(value);
      assert.equal(new TextDecoder().decode(wire), `GET ${address}\r\n`);
      assert.equal(mustValue(parseHttp09Request(wire), "generated request").value, address);
    }
    for (let length = 0; length <= 256; length++) {
      const body = Uint8Array.from({ length }, (_, i) => (i * 37) & 0x7f);
      assert.equal(mustValue(decodeHttp09Response(body), "ASCII body"), new TextDecoder().decode(body));
    }
  });

  it("keeps the HTTP encoder boundary safe against forged JavaScript values", () => {
    // Node's type stripping removes TypeScript `private`; the runtime class
    // brand and encoder validation must still reject direct/structural forgery.
    let direct: unknown;
    try {
      direct = Reflect.construct(Http09Address, ["/ok\r\nGET /evil"]);
    } catch (error) {
      assert.ok(error instanceof TypeError);
    }
    if (direct !== undefined) {
      assert.throws(() => encodeHttp09Request(direct as Http09Address), TypeError);
    }
    assert.throws(
      () => encodeHttp09Request({ value: "/ok\r\nGET /evil" } as unknown as Http09Address),
      TypeError,
    );
    const valid = mustValue(Http09Address.create("/safe"), "safe address");
    assert.equal(new TextDecoder().decode(encodeHttp09Request(valid)), "GET /safe\r\n");
  });

  it("returns a typed error for an untyped non-string HTML source", () => {
    const result = parseHtmlTags1992(17 as unknown as string);
    assert.equal(result.ok, false);
    if (!result.ok) assert.equal(result.error.kind, "invalid-source");
  });

  it("preserves early SGML, empty marker, UTF-16, and scanner parity behavior", () => {
    const source = "<UL><LI>one<LI>two</UL><P>café &amp; &lt; &unknown;</P>";
    const doc = mustValue(parseHtmlTags1992(source), "SGML list source");
    assertArenaWellFormed(doc);
    assert.deepEqual(rootsJson(doc), [
      { tag: "ul", attributes: {}, children: [
        { tag: "li", attributes: {}, children: ["one"] },
        { tag: "li", attributes: {}, children: ["two"] },
      ] },
      { tag: "p", attributes: {}, children: [] },
      "café & < &unknown;",
    ]);

    const attrs = mustValue(parseHtmlTags1992(`<A HREF HREF=/later NAME='a &amp; b' TYPE="UP>DOWN">x</A>`), "anchor attributes");
    const anchor = attrs.nodes[attrs.roots[0]];
    assert.equal(anchor.kind, "element");
    if (anchor.kind === "element") {
      assert.deepEqual(anchor.attributes, { href: null, name: "a & b", type: "UP>DOWN" });
    }

    const loneSurrogate = "lone-\ud800-end";
    const unicode = mustValue(parseHtmlTags1992(loneSurrogate), "UTF-16 lone surrogate");
    assert.deepEqual(rootsJson(unicode), [loneSurrogate]);

    const raw = mustValue(parseHtmlTags1992("<XMP>aaaaaé</XMP><P>after</P><LISTING>aaaaaaaé</LISTING><P>end</P>"), "UTF-8 raw text");
    assert.deepEqual(rootsJson(raw), [
      { tag: "xmp", attributes: {}, children: ["aaaaaé"] },
      { tag: "p", attributes: {}, children: [] }, "after",
      { tag: "listing", attributes: {}, children: ["aaaaaaaé"] },
      { tag: "p", attributes: {}, children: [] }, "end",
    ]);

    const attrCrLf = mustValue(parseHtmlTags1992("<A HREF=\"line1\r\nline2\">x</A>"), "CRLF in quoted attr");
    const crlfAnchor = attrCrLf.nodes[attrCrLf.roots[0]];
    assert.equal(crlfAnchor.kind, "element");
    if (crlfAnchor.kind === "element") assert.equal(crlfAnchor.attributes.href, "line1\nline2");
  });

  it("handles hostile depth and malformed-but-recoverable source iteratively", () => {
    const depth = 12_000;
    const nested = "<ADDRESS>".repeat(depth) + "x" + "</ADDRESS>".repeat(depth);
    const doc = mustValue(parseHtmlTags1992(nested), "deep nesting");
    assert.equal(doc.nodes.length, depth + 1);
    assertArenaWellFormed(doc);
    // Drop the arena after a successful iterative walk; it contains only flat
    // node records and numeric edges, with no recursive child ownership.

    const malformed = "<1".repeat(20_000) + "tail";
    const recovered = mustValue(parseHtmlTags1992(malformed), "malformed plain text");
    assert.deepEqual(rootsJson(recovered), [malformed]);
    const unmatched = mustValue(parseHtmlTags1992("</ADDRESS><P>text"), "unmatched end + open marker");
    assertArenaWellFormed(unmatched);
  });

  it("runs the repository VER06 engine with independent wire and HTML oracles", () => {
    const seeds = [
      new TextEncoder().encode("GET /\r\n"),
      new TextEncoder().encode("GET /bad path\n"),
      new TextEncoder().encode("POST /\n"),
      new TextEncoder().encode("<A HREF='x'>ok</A>"),
      new TextEncoder().encode("<!--x--><UL><LI>one<LI>two</UL>"),
      Uint8Array.of(0xff, 0x3c, 0x41, 0x3e),
    ];
    const report = fuzzTarget(seeds, { maxIterations: 2_048, seed: 0x485745423034, maxInputLen: 256, baseEnergy: 10 }, (input, coverage): FuzzOutcome => {
      coverage.recordEdge(Math.min(input.length, 255));
      coverage.recordEdge(256);
      const expected = wireOracle(input);
      const actual = parseHttp09Request(input);
      const wireGood = expected === null ? !actual.ok : actual.ok && new TextEncoder().encode(actual.value.value).every((b, i) => b === expected[i]) && actual.value.value.length === expected.length;
      if (!wireGood) return { status: "crash", kind: "wire-oracle-mismatch", frames: ["assurance"] };

      let markup: string | undefined;
      try {
        markup = new TextDecoder("utf-8", { fatal: true }).decode(input);
      } catch (error) {
        if (!(error instanceof TypeError)) throw error;
      }
      if (markup !== undefined) {
        coverage.recordEdge(257);
        const parsed = parseHtmlTags1992(markup);
        if (!parsed.ok) return { status: "crash", kind: "valid-utf8-html-error", frames: ["assurance"] };
        assertArenaWellFormed(parsed.value);
      }
      return { status: "pass" };
    });
    assert.equal(report.iterations, 2_048);
    assert.equal(report.crashes.length, 0, JSON.stringify(report.crashes));
    assert.equal(report.triage.crash + report.mutation.crash, 0);
    assert.equal(report.unrecordedCrashOutcomes, 0);
    assert.equal(report.clean, true);
    assert.equal(report.triage.timeout + report.mutation.timeout, 0);
    assert.equal(report.triage.oom + report.mutation.oom, 0);
  });
});
