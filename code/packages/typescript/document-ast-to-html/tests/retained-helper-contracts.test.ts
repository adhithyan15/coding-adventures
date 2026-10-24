import { expect, it } from "vitest";
import { Scanner, isAsciiPunctuation, isUnicodePunctuation, isAsciiWhitespace, isUnicodeWhitespace, isDigit, normalizeLinkLabel, normalizeUrl } from "../src/scanner.js";
import { decodeEntity, decodeEntities, escapeHtml } from "../src/entities.js";
it("retains scanner cursor and failure-position contracts", () => {
 const s=new Scanner(" \tabc123\n<>");
 expect(s.countIndent()).toBe(4); expect(s.pos).toBe(0); expect(s.skipSpaces()).toBe(2); expect(s.remaining).toBe(9);
 expect(s.peek()).toBe("a"); expect(s.peek(1)).toBe("b"); expect(s.peekSlice(3)).toBe("abc");
 expect(s.match("xyz")).toBe(false); expect(s.pos).toBe(2);
 expect(s.matchRegex(/[a-z]+/)).toBe("abc"); expect(s.matchRegex(/[a-z]+/y)).toBeNull(); expect(s.pos).toBe(5);
 expect(s.match("12")).toBe(true); expect(s.consumeWhile(ch => ch >= "0" && ch <= "9")).toBe("3");
 expect(s.consumeLine()).toBe(""); expect(s.advance()).toBe("\n"); expect(s.sliceFrom(2)).toBe("abc123\n"); expect(s.rest()).toBe("<>");
 expect(s.consumeWhile(() => false)).toBe(""); s.skip(100); expect(s.done).toBe(true); expect(s.peek()).toBe(""); expect(s.advance()).toBe("");
 const line=new Scanner("one\ntwo",4); expect(line.consumeLine()).toBe("two"); expect(line.done).toBe(true);
 const sticky=new Scanner("a12",1); expect(sticky.matchRegex(/\d+/y)).toBe("12");
 const spaces=new Scanner("  X"); spaces.skipIndent(1); expect(spaces.pos).toBe(1); spaces.skipIndent(1); expect(spaces.pos).toBe(2); spaces.skipIndent(1); expect(spaces.pos).toBe(2);
 const tabs=new Scanner("\t X"); tabs.skipIndent(2); expect(tabs.pos).toBe(0); tabs.skipIndent(4); expect(tabs.pos).toBe(1); tabs.skipIndent(0); expect(tabs.pos).toBe(1);
 const end=new Scanner("  "); end.skipIndent(8); expect(end.done).toBe(true);
});
it("classifies characters and normalizes labels/URLs with literals", () => {
 expect(isAsciiPunctuation("!")).toBe(true); expect(isAsciiPunctuation("a")).toBe(false);
 for (const c of ["§","€","😀","!"]) expect(isUnicodePunctuation(c)).toBe(true);
 for (const c of ["","a"]) expect(isUnicodePunctuation(c)).toBe(false);
 for (const c of [" ","\t","\n","\r","\f"]) expect(isAsciiWhitespace(c)).toBe(true);
 expect(isAsciiWhitespace("\v")).toBe(false);
 for (const c of [" ","\u00a0","\u2003"]) expect(isUnicodeWhitespace(c)).toBe(true);
 for (const c of ["","a","\u0085"]) expect(isUnicodeWhitespace(c)).toBe(false);
 for (const c of ["0","9"]) expect(isDigit(c)).toBe(true);
 for (const c of ["/",":","a"]) expect(isDigit(c)).toBe(false);
 expect(normalizeLinkLabel(" \tSTRAẞE\nß ")).toBe("strasse ss");
 expect(normalizeUrl("/a b?q=λ&x=%20")).toBe("/a%20b?q=%CE%BB&x=%20");
});
it("decodes entities once and preserves unknown names", () => {
 for (const s of ["plain","&amp","amp;","&bogus;","&constructor;","&toString;"]) expect(decodeEntity(s)).toBe(s);
 for (const [s,out] of [["&amp;","&"],["&lt;","<"],["&quot;",'"'],["&#65;","A"],["&#x41;","A"],["&#X41;","A"],["&#0;","�"],["&#1114112;","�"],["&#xZZ;","�"]]) expect(decodeEntity(s!)).toBe(out);
 expect(decodeEntities("Tom &amp; Jerry &#65; &#x41; &bogus;")).toBe("Tom & Jerry A A &bogus;");
 expect(decodeEntities("plain &amp")).toBe("plain &amp"); expect(decodeEntities("plain")).toBe("plain");
 expect(escapeHtml('&<>"')).toBe("&amp;&lt;&gt;&quot;");
});
