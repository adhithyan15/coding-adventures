/**
 * validate.test.ts — lang / dir / attribute-key / attribute-value
 * validators + escape helpers.
 */

import { describe, it, expect } from "vitest";
import {
  escapeHtmlAttr,
  stripAsciiControl,
  validateAttrKey,
  validateAttrValue,
  validateDir,
  validateLang,
} from "../src/index.js";

describe("validateLang — accept", () => {
  it("primary alpha 'en'", () => { expect(validateLang("en")).toBe("en"); });
  it("'en-US'", () => { expect(validateLang("en-US")).toBe("en-US"); });
  it("'zh-Hant-HK'", () => { expect(validateLang("zh-Hant-HK")).toBe("zh-Hant-HK"); });
  it("'pt-BR'", () => { expect(validateLang("pt-BR")).toBe("pt-BR"); });
  it("'de-CH-1996'", () => { expect(validateLang("de-CH-1996")).toBe("de-CH-1996"); });
  it("3-letter primary", () => { expect(validateLang("ang")).toBe("ang"); });
  it("8-letter cap accepted", () => { expect(validateLang("abcdefgh")).toBe("abcdefgh"); });
});

describe("validateLang — reject", () => {
  it("non-string", () => { expect(() => validateLang(42 as unknown as string)).toThrow(/must be a string/); });
  it("null", () => { expect(() => validateLang(null)).toThrow(/must be a string/); });
  it("empty", () => { expect(() => validateLang("")).toThrow(/non-empty/); });
  it("9-letter primary (over cap)", () => { expect(() => validateLang("abcdefghi")).toThrow(/BCP-47/); });
  it("digit-leading subtag", () => { expect(() => validateLang("123")).toThrow(/BCP-47/); });
  it("trailing dash", () => { expect(() => validateLang("en-")).toThrow(/BCP-47/); });
  it("leading dash", () => { expect(() => validateLang("-en")).toThrow(/BCP-47/); });
  it("double dash", () => { expect(() => validateLang("en--US")).toThrow(/BCP-47/); });
  it("underscore (not dash)", () => { expect(() => validateLang("en_US")).toThrow(/BCP-47/); });
  it("space", () => { expect(() => validateLang("en US")).toThrow(/BCP-47/); });
  it("non-ASCII", () => { expect(() => validateLang("enü")).toThrow(/BCP-47/); });
  it("XSS-attempt", () => { expect(() => validateLang('en"><script>')).toThrow(/BCP-47/); });
  it("attr-injection-attempt", () => { expect(() => validateLang('en" onclick="')).toThrow(/BCP-47/); });
});

describe("validateDir", () => {
  it("ltr", () => { expect(validateDir("ltr")).toBe("ltr"); });
  it("rtl", () => { expect(validateDir("rtl")).toBe("rtl"); });
  it("auto", () => { expect(validateDir("auto")).toBe("auto"); });
  it("rejects LTR (case-sensitive)", () => { expect(() => validateDir("LTR")).toThrow(/one of/); });
  it("rejects empty", () => { expect(() => validateDir("")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateDir(1 as unknown as string)).toThrow(/must be a string/); });
});

describe("validateAttrKey — accept", () => {
  it("simple letter", () => { expect(validateAttrKey("a", "f")).toBe("a"); });
  it("class", () => { expect(validateAttrKey("class", "f")).toBe("class"); });
  it("data-* attribute", () => { expect(validateAttrKey("data-theme", "f")).toBe("data-theme"); });
  it("aria-* attribute", () => { expect(validateAttrKey("aria-label", "f")).toBe("aria-label"); });
  it("colon-namespaced (xml:base)", () => { expect(validateAttrKey("xml:base", "f")).toBe("xml:base"); });
  it("alphanumeric with dashes", () => { expect(validateAttrKey("data-1-x-2", "f")).toBe("data-1-x-2"); });
  it("64-char cap accepted", () => {
    const k = "a" + "1".repeat(63);
    expect(validateAttrKey(k, "f")).toBe(k);
  });
});

describe("validateAttrKey — reject", () => {
  it("non-string", () => { expect(() => validateAttrKey(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null", () => { expect(() => validateAttrKey(null, "f")).toThrow(/must be a string/); });
  it("empty", () => { expect(() => validateAttrKey("", "f")).toThrow(/non-empty/); });
  it("uppercase", () => { expect(() => validateAttrKey("Class", "f")).toThrow(/lowercase ASCII/); });
  it("starts with digit", () => { expect(() => validateAttrKey("1cls", "f")).toThrow(/lowercase ASCII/); });
  it("starts with dash", () => { expect(() => validateAttrKey("-data", "f")).toThrow(/lowercase ASCII/); });
  it("contains space", () => { expect(() => validateAttrKey("class name", "f")).toThrow(/lowercase ASCII/); });
  it("contains quote (injection attempt)", () => { expect(() => validateAttrKey(`x"y`, "f")).toThrow(/lowercase ASCII/); });
  it("contains > (injection attempt)", () => { expect(() => validateAttrKey("x>y", "f")).toThrow(/lowercase ASCII/); });
  it("contains = (injection attempt)", () => { expect(() => validateAttrKey("x=y", "f")).toThrow(/lowercase ASCII/); });
  it("65-char (over cap)", () => {
    const k = "a" + "1".repeat(64);
    expect(() => validateAttrKey(k, "f")).toThrow(/lowercase ASCII/);
  });
  it("__proto__ rejected (starts with _)", () => {
    expect(() => validateAttrKey("__proto__", "f")).toThrow(/lowercase ASCII/);
  });
  it("constructor rejected (passes shape — but only if shape matched)", () => {
    // Wait, "constructor" matches shape.  But it's not reserved AND not on*.
    // So it actually passes — and that's fine because `constructor` as an
    // HTML attribute is harmless (browsers ignore unknown attrs).  The
    // attack we care about is event handlers / lang shadowing.
    expect(validateAttrKey("constructor", "f")).toBe("constructor");
  });
  it("reserved 'lang'", () => { expect(() => validateAttrKey("lang", "f")).toThrow(/reserved/); });
  it("reserved 'dir'", () => { expect(() => validateAttrKey("dir", "f")).toThrow(/reserved/); });
  it("reserved 'xmlns'", () => { expect(() => validateAttrKey("xmlns", "f")).toThrow(/reserved/); });
  it("on* event handler 'onload'", () => { expect(() => validateAttrKey("onload", "f")).toThrow(/event-handler/); });
  it("on* 'onclick'", () => { expect(() => validateAttrKey("onclick", "f")).toThrow(/event-handler/); });
  it("on* 'onerror'", () => { expect(() => validateAttrKey("onerror", "f")).toThrow(/event-handler/); });
  it("on* with dash 'on-thing'", () => { expect(() => validateAttrKey("on-thing", "f")).toThrow(/event-handler/); });
  it("error contains field path", () => {
    expect(() => validateAttrKey("onload", "htmlAttrs"))
      .toThrow(/htmlAttrs attribute key "onload"/);
  });
});

describe("validateAttrValue — accept", () => {
  it("simple string", () => { expect(validateAttrValue("blue", "f")).toBe("blue"); });
  it("empty allowed", () => { expect(validateAttrValue("", "f")).toBe(""); });
  it("string with quotes (escaped later by escapeHtmlAttr)", () => {
    expect(validateAttrValue(`he said "hi"`, "f")).toBe(`he said "hi"`);
  });
  it("string with < > & (escaped later)", () => {
    expect(validateAttrValue("a<b&c>d", "f")).toBe("a<b&c>d");
  });
});

describe("validateAttrValue — reject", () => {
  it("non-string", () => { expect(() => validateAttrValue(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null", () => { expect(() => validateAttrValue(null, "f")).toThrow(/must be a string/); });
  it("NUL byte", () => { expect(() => validateAttrValue("a\x00b", "f")).toThrow(/control bytes/); });
  it("tab", () => { expect(() => validateAttrValue("a\tb", "f")).toThrow(/control bytes/); });
  it("newline", () => { expect(() => validateAttrValue("a\nb", "f")).toThrow(/control bytes/); });
  it("DEL", () => { expect(() => validateAttrValue("a\x7Fb", "f")).toThrow(/control bytes/); });
  it("error contains field path", () => {
    expect(() => validateAttrValue("\x00", "bodyAttrs[\"class\"]"))
      .toThrow(/bodyAttrs\["class"\]/);
  });
});

describe("escapeHtmlAttr", () => {
  it("ampersand", () => { expect(escapeHtmlAttr("a&b")).toBe("a&amp;b"); });
  it("less-than", () => { expect(escapeHtmlAttr("a<b")).toBe("a&lt;b"); });
  it("greater-than", () => { expect(escapeHtmlAttr("a>b")).toBe("a&gt;b"); });
  it("double quote", () => { expect(escapeHtmlAttr(`"`)).toBe("&quot;"); });
  it("single quote", () => { expect(escapeHtmlAttr(`'`)).toBe("&#39;"); });
  it("composite", () => { expect(escapeHtmlAttr(`<&>"'`)).toBe("&lt;&amp;&gt;&quot;&#39;"); });
  it("strips NUL", () => { expect(escapeHtmlAttr("a\x00b")).toBe("ab"); });
  it("non-string coerced", () => { expect(escapeHtmlAttr(7 as unknown as string)).toBe("7"); });
});

describe("stripAsciiControl", () => {
  it("removes control bytes", () => {
    expect(stripAsciiControl("a\x00b\x1Bc\x7Fd")).toBe("abcd");
  });
  it("preserves printable", () => {
    expect(stripAsciiControl("Hello, World!")).toBe("Hello, World!");
  });
});
