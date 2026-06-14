/**
 * toml.test.ts — docs-frontmatter TOML adapter.
 *
 * The TOML adapter delegates lex+parse to @coding-adventures/toml-parser
 * and walks the resulting AST.  Surface-syntax errors therefore surface
 * as the upstream parser/lexer messages (with line:col); subset-violation
 * errors are our own `forme-doc-frontmatter:` messages.
 */

import { describe, it, expect } from "vitest";
import { parseToml } from "../src/index.js";

describe("parseToml — scalars", () => {
  it("basic string", () => {
    expect(parseToml(`title = "Hello"`)).toEqual({ title: "Hello" });
  });
  it("literal string", () => {
    expect(parseToml(`title = 'C:\\path'`)).toEqual({ title: "C:\\path" });
  });
  it("string with escape sequences", () => {
    expect(parseToml(`s = "a\\nb\\tc\\r"`)).toEqual({ s: "a\nb\tc\r" });
  });
  it("string with \\b and \\f escapes", () => {
    expect(parseToml(`s = "a\\bb\\fc"`)).toEqual({ s: "a\bb\fc" });
  });
  it("string with \\/ escape", () => {
    expect(parseToml(`s = "path\\/to"`)).toEqual({ s: "path/to" });
  });
  it("string with \\uXXXX escape", () => {
    expect(parseToml(`s = "\\u00e9"`)).toEqual({ s: "é" });
  });
  it("string with \\UXXXXXXXX escape", () => {
    expect(parseToml(`s = "\\U0001F600"`)).toEqual({ s: "😀" });
  });
  it("integer", () => {
    expect(parseToml("n = 42")).toEqual({ n: 42 });
  });
  it("positive-signed integer", () => {
    expect(parseToml("n = +42")).toEqual({ n: 42 });
  });
  it("negative integer", () => {
    expect(parseToml("n = -7")).toEqual({ n: -7 });
  });
  it("integer with underscores", () => {
    expect(parseToml("n = 1_000_000")).toEqual({ n: 1000000 });
  });
  it("hex integer", () => {
    expect(parseToml("n = 0xff")).toEqual({ n: 255 });
  });
  it("octal integer", () => {
    expect(parseToml("n = 0o17")).toEqual({ n: 15 });
  });
  it("binary integer", () => {
    expect(parseToml("n = 0b1010")).toEqual({ n: 10 });
  });
  it("float", () => {
    expect(parseToml("f = 3.14")).toEqual({ f: 3.14 });
  });
  it("float with scientific notation", () => {
    expect(parseToml("f = 1.5e3")).toEqual({ f: 1500 });
  });
  it("float +inf", () => {
    expect(parseToml("f = +inf")).toEqual({ f: Infinity });
  });
  it("float -inf", () => {
    expect(parseToml("f = -inf")).toEqual({ f: -Infinity });
  });
  it("float inf (no sign)", () => {
    expect(parseToml("f = inf")).toEqual({ f: Infinity });
  });
  it("float nan", () => {
    const out = parseToml("f = nan");
    expect(Number.isNaN(out.f as number)).toBe(true);
  });
  it("boolean true", () => {
    expect(parseToml("draft = true")).toEqual({ draft: true });
  });
  it("boolean false", () => {
    expect(parseToml("draft = false")).toEqual({ draft: false });
  });
  it("RFC 3339 date", () => {
    expect(parseToml("d = 2026-05-20")).toEqual({ d: "2026-05-20" });
  });
  it("RFC 3339 datetime with Z", () => {
    expect(parseToml("d = 2026-05-20T12:00:00Z")).toEqual({ d: "2026-05-20T12:00:00Z" });
  });
  it("LOCAL_DATETIME (no offset)", () => {
    expect(parseToml("d = 2026-05-20T12:00:00")).toEqual({ d: "2026-05-20T12:00:00" });
  });
  it("LOCAL_TIME", () => {
    expect(parseToml("t = 12:00:00")).toEqual({ t: "12:00:00" });
  });
  it("multi-line basic string", () => {
    expect(parseToml(`s = """hello\nworld"""`)).toEqual({ s: "hello\nworld" });
  });
  it("multi-line literal string", () => {
    expect(parseToml(`s = '''hello\nworld'''`)).toEqual({ s: "hello\nworld" });
  });
});

describe("parseToml — arrays", () => {
  it("empty", () => {
    expect(parseToml("tags = []")).toEqual({ tags: [] });
  });
  it("strings", () => {
    expect(parseToml(`tags = ["a", "b"]`)).toEqual({ tags: ["a", "b"] });
  });
  it("integers", () => {
    expect(parseToml("ns = [1, 2, 3]")).toEqual({ ns: [1, 2, 3] });
  });
  it("multi-line array with trailing comma", () => {
    expect(parseToml(`tags = [\n  "a",\n  "b",\n]`)).toEqual({ tags: ["a", "b"] });
  });
  it("array of mixed scalar types (legal in TOML)", () => {
    expect(parseToml(`xs = [1, "two", true]`)).toEqual({ xs: [1, "two", true] });
  });
});

describe("parseToml — multiple keys + comments", () => {
  it("multiple", () => {
    expect(parseToml(`title = "Hello"\ndate = 2026-05-20\ntags = ["a"]`))
      .toEqual({ title: "Hello", date: "2026-05-20", tags: ["a"] });
  });
  it("ignores blank lines", () => {
    expect(parseToml(`a = 1\n\n\nb = 2`)).toEqual({ a: 1, b: 2 });
  });
  it("ignores comment lines", () => {
    expect(parseToml(`# c\na = 1\n# c2`)).toEqual({ a: 1 });
  });
  it("strips inline comment", () => {
    expect(parseToml(`a = 1 # inline`)).toEqual({ a: 1 });
  });
  it("inline # inside string preserved", () => {
    expect(parseToml(`a = "hello # world"`)).toEqual({ a: "hello # world" });
  });
  it("empty document", () => {
    expect(parseToml("")).toEqual({});
  });
});

describe("parseToml — security defences", () => {
  it("__proto__ rejected", () => {
    expect(() => parseToml("__proto__ = 1")).toThrow(/prototype-pollution/);
  });
  it("constructor rejected", () => {
    expect(() => parseToml(`constructor = "bad"`)).toThrow(/prototype-pollution/);
  });
  it("prototype rejected", () => {
    expect(() => parseToml("prototype = 1")).toThrow(/prototype-pollution/);
  });
  it("null prototype output", () => {
    expect(Object.getPrototypeOf(parseToml("a = 1"))).toBeNull();
  });
  it("duplicate key rejected", () => {
    expect(() => parseToml("a = 1\na = 2")).toThrow(/duplicated/);
  });
  it("toString rejected (widened reserved list)", () => {
    expect(() => parseToml("toString = 1")).toThrow(/prototype-pollution/);
  });
  it("valueOf rejected", () => {
    expect(() => parseToml("valueOf = 1")).toThrow(/prototype-pollution/);
  });
  it("hasOwnProperty rejected", () => {
    expect(() => parseToml("hasOwnProperty = 1")).toThrow(/prototype-pollution/);
  });
  it("__lookupGetter__ rejected", () => {
    expect(() => parseToml("__lookupGetter__ = 1")).toThrow(/prototype-pollution/);
  });
  it("source > 1MB rejected", () => {
    const huge = "a = 1\n".repeat(300_000);
    expect(() => parseToml(huge)).toThrow(/1048576-byte cap/);
  });
  it("> 1000 keys rejected", () => {
    const many = Array.from({ length: 1001 }, (_, i) => `k${i} = ${i}`).join("\n");
    expect(() => parseToml(many)).toThrow(/1000-key cap/);
  });
  it("> 64 KB value rejected", () => {
    expect(() => parseToml(`x = "${"a".repeat(70_000)}"`)).toThrow(/65536-byte cap/);
  });
});

describe("parseToml — subset enforcement (rejections)", () => {
  it("table header [section] rejected", () => {
    expect(() => parseToml("[section]\na = 1")).toThrow(/tables \(\[section\]\).*not supported/);
  });
  it("array-of-tables [[section]] rejected", () => {
    expect(() => parseToml("[[products]]\na = 1")).toThrow(/arrays-of-tables.*not supported/);
  });
  it("dotted key rejected", () => {
    expect(() => parseToml("a.b = 1")).toThrow(/dotted keys.*not supported/);
  });
  it("quoted (basic-string) key rejected", () => {
    expect(() => parseToml(`"127.0.0.1" = 1`)).toThrow(/non-bare key.*not supported/);
  });
  it("quoted (literal-string) key rejected", () => {
    expect(() => parseToml(`'foo bar' = 1`)).toThrow(/non-bare key.*not supported/);
  });
  it("inline table value rejected", () => {
    expect(() => parseToml(`pt = { x = 1, y = 2 }`)).toThrow(/inline tables.*not supported/);
  });
  it("array-of-arrays rejected", () => {
    expect(() => parseToml(`xs = [[1, 2], [3, 4]]`)).toThrow(/arrays-of-arrays.*not supported/);
  });
  it("array-of-inline-tables rejected", () => {
    expect(() => parseToml(`xs = [{a = 1}]`)).toThrow(/inline tables.*not supported/);
  });
  it("bare key exceeding 128-char cap rejected", () => {
    // Toml lexer accepts any-length BARE_KEY, but our docs subset caps at 128.
    const longKey = "a".repeat(300);
    expect(() => parseToml(`${longKey} = 1`)).toThrow(/bare-key pattern/);
  });
  it("bare key starting with hyphen rejected", () => {
    // TOML lexes `-foo` as a BARE_KEY (INTEGER requires a digit after sign),
    // but our subset requires the first char to be a letter or underscore.
    expect(() => parseToml(`-foo = 1`)).toThrow(/bare-key pattern/);
  });
});

describe("parseToml — error matrix (upstream parser/lexer)", () => {
  it("non key=value line surfaces parser error", () => {
    // toml-parser ParseError; the message includes line:col.
    expect(() => parseToml("just text")).toThrow(/Parse error|Expected/);
  });
  it("unrecognised character surfaces lexer error", () => {
    expect(() => parseToml("a = ???")).toThrow(/Lexer error|Unexpected character/);
  });
  it("safe-integer overflow", () => {
    expect(() => parseToml("n = 99999999999999999999")).toThrow(/safe integer/);
  });
  it("unsupported escape", () => {
    expect(() => parseToml(`a = "\\z"`)).toThrow(/unsupported escape/);
  });
  it("invalid \\u escape", () => {
    expect(() => parseToml(`a = "\\uZZZZ"`)).toThrow(/invalid \\u escape/);
  });
  it("invalid \\U escape", () => {
    expect(() => parseToml(`a = "\\UZZZZZZZZ"`)).toThrow(/invalid \\U escape/);
  });
  it("unterminated string surfaces lexer error", () => {
    expect(() => parseToml(`a = "unterminated`)).toThrow(/Lexer error|Unexpected character/);
  });
  it("missing value after = surfaces parser error", () => {
    expect(() => parseToml("a =")).toThrow(/Parse error|Expected/);
  });
  it("inline array with escaped chars", () => {
    expect(parseToml(`x = ["a\\nb", "c"]`)).toEqual({ x: ["a\nb", "c"] });
  });
  it("string ending in escaped char on inline-comment scan", () => {
    expect(parseToml(`a = "a\\\\b" # comment`)).toEqual({ a: "a\\b" });
  });
});
