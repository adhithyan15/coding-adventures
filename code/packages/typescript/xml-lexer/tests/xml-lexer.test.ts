/**
 * Tests for the XML Lexer (TypeScript).
 *
 * These tests verify that the XML lexer correctly tokenizes XML documents
 * using pattern groups and the on-token callback. The callback switches
 * between pattern groups based on which delimiter token was just matched,
 * enabling context-sensitive lexing of XML's different syntactic regions.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Basic tags** — open/close tags, text content, namespaces
 *   2. **Self-closing tags** — `<br/>`, `<br />`
 *   3. **Attributes** — double-quoted, single-quoted, multiple
 *   4. **Comments** — `<!-- ... -->`, whitespace preservation
 *   5. **CDATA sections** — `<![CDATA[ ... ]]>`, raw text
 *   6. **Processing instructions** — `<?xml ... ?>`
 *   7. **Entity references** — `&amp;`, `&#65;`, `&#x41;`
 *   8. **Nested and mixed content** — tags within tags, text + elements
 *   9. **Edge cases** — empty input, text only, whitespace handling
 *  10. **Position tracking** — line and column numbers
 */

import { describe, it, expect } from "vitest";
import { tokenizeXML, createXMLLexer } from "../src/tokenizer.js";

// ---------------------------------------------------------------------------
// Helpers — extract token types and (type, value) pairs
// ---------------------------------------------------------------------------

/**
 * Tokenize XML and return (type, value) pairs, excluding EOF.
 *
 * This helper makes assertions concise — we compare arrays of tuples
 * instead of inspecting full Token objects with line/column metadata.
 */
function tokenPairs(source: string): [string, string][] {
  return tokenizeXML(source)
    .filter((t) => t.type !== "EOF")
    .map((t) => [t.type, t.value]);
}

/**
 * Tokenize XML and return just the type names, excluding EOF.
 *
 * Even more concise than tokenPairs — useful when we only care about
 * the sequence of token types, not their values.
 */
function tokenTypes(source: string): string[] {
  return tokenizeXML(source)
    .filter((t) => t.type !== "EOF")
    .map((t) => t.type);
}

// ===========================================================================
// Basic Tags
// ===========================================================================

describe("basic tags", () => {
  it("tokenizes a simple element: <p>text</p>", () => {
    /**
     * The simplest XML structure: an open tag, text content, and a
     * close tag. The callback pushes "tag" on OPEN_TAG_START and pops
     * on TAG_CLOSE, so the lexer uses the tag group's patterns for
     * TAG_NAME and the default group's patterns for TEXT.
     */
    const pairs = tokenPairs("<p>text</p>");
    expect(pairs).toEqual([
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "p"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "text"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "p"],
      ["TAG_CLOSE", ">"],
    ]);
  });

  it("tokenizes tags with namespace prefixes: <ns:tag>", () => {
    /**
     * XML namespace prefixes use colons: `<ns:tag>`. The TAG_NAME
     * regex `[a-zA-Z_][a-zA-Z0-9_:.-]*` allows colons, so the entire
     * `ns:tag` is captured as one TAG_NAME token.
     */
    const types = tokenTypes("<ns:tag>content</ns:tag>");
    expect(types).toEqual([
      "OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE",
      "TEXT",
      "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE",
    ]);
    const pairs = tokenPairs("<ns:tag>content</ns:tag>");
    expect(pairs[1]).toEqual(["TAG_NAME", "ns:tag"]);
  });

  it("tokenizes an explicitly empty element: <div></div>", () => {
    /**
     * An element with no content — the close tag immediately follows
     * the open tag. No TEXT token is emitted between them.
     */
    const pairs = tokenPairs("<div></div>");
    expect(pairs).toEqual([
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "div"],
      ["TAG_CLOSE", ">"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "div"],
      ["TAG_CLOSE", ">"],
    ]);
  });

  it("tokenizes a self-closing tag: <br/>", () => {
    /**
     * Self-closing tags end with `/>` instead of `>`. The SELF_CLOSE
     * token pops the tag group, just like TAG_CLOSE does.
     */
    const pairs = tokenPairs("<br/>");
    expect(pairs).toEqual([
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "br"],
      ["SELF_CLOSE", "/>"],
    ]);
  });

  it("tokenizes a self-closing tag with space: <br />", () => {
    /**
     * The space before `/>` is consumed by the skip pattern (whitespace
     * is insignificant inside tags), so the token sequence is the same
     * as `<br/>`.
     */
    const pairs = tokenPairs("<br />");
    expect(pairs).toEqual([
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "br"],
      ["SELF_CLOSE", "/>"],
    ]);
  });
});

// ===========================================================================
// Attributes
// ===========================================================================

describe("attributes", () => {
  it("tokenizes a double-quoted attribute: class=\"main\"", () => {
    /**
     * Inside a tag, the lexer is in the "tag" group. It recognizes
     * TAG_NAME for both the tag name and attribute names, ATTR_EQUALS
     * for `=`, and ATTR_VALUE for the quoted value.
     *
     * Note: ATTR_VALUE_DQ is aliased to ATTR_VALUE in the grammar,
     * so the emitted token type is always ATTR_VALUE regardless of
     * whether double or single quotes were used.
     */
    const pairs = tokenPairs('<div class="main">');
    expect(pairs).toEqual([
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "div"],
      ["TAG_NAME", "class"],
      ["ATTR_EQUALS", "="],
      ["ATTR_VALUE", '"main"'],
      ["TAG_CLOSE", ">"],
    ]);
  });

  it("tokenizes a single-quoted attribute: class='main'", () => {
    /**
     * Single-quoted values also alias to ATTR_VALUE. XML allows both
     * quote styles, which is useful when the value contains one type
     * of quote character.
     */
    const pairs = tokenPairs("<div class='main'>");
    expect(pairs).toEqual([
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "div"],
      ["TAG_NAME", "class"],
      ["ATTR_EQUALS", "="],
      ["ATTR_VALUE", "'main'"],
      ["TAG_CLOSE", ">"],
    ]);
  });

  it("tokenizes multiple attributes on one tag", () => {
    /**
     * Multiple attributes are just repeated sequences of
     * TAG_NAME ATTR_EQUALS ATTR_VALUE within the tag group.
     */
    const pairs = tokenPairs('<a href="url" target="_blank">');
    const tagNames = pairs.filter(([t]) => t === "TAG_NAME").map(([, v]) => v);
    expect(tagNames).toEqual(["a", "href", "target"]);
    const attrValues = pairs.filter(([t]) => t === "ATTR_VALUE").map(([, v]) => v);
    expect(attrValues).toEqual(['"url"', '"_blank"']);
  });

  it("tokenizes an attribute on a self-closing tag", () => {
    /**
     * Self-closing tags can have attributes too. The SELF_CLOSE token
     * pops the tag group, just like TAG_CLOSE.
     */
    const types = tokenTypes('<img src="photo.jpg"/>');
    expect(types).toContain("SELF_CLOSE");
    expect(types).toContain("ATTR_VALUE");
  });
});

// ===========================================================================
// Comments
// ===========================================================================

describe("comments", () => {
  it("tokenizes a simple comment: <!-- text -->", () => {
    /**
     * Comments push the "comment" group and disable skip patterns.
     * The COMMENT_TEXT regex matches everything that isn't `-->`.
     * When `-->` is found, COMMENT_END pops the group and re-enables skip.
     */
    const pairs = tokenPairs("<!-- hello -->");
    expect(pairs).toEqual([
      ["COMMENT_START", "<!--"],
      ["COMMENT_TEXT", " hello "],
      ["COMMENT_END", "-->"],
    ]);
  });

  it("preserves whitespace inside comments (skip disabled)", () => {
    /**
     * The callback disables skip patterns when entering the comment
     * group. This means spaces, tabs, and other whitespace are NOT
     * silently consumed — they become part of the COMMENT_TEXT token.
     */
    const pairs = tokenPairs("<!--  spaces  and\ttabs  -->");
    const text = pairs.filter(([t]) => t === "COMMENT_TEXT").map(([, v]) => v);
    expect(text).toEqual(["  spaces  and\ttabs  "]);
  });

  it("allows single dashes inside comments", () => {
    /**
     * The COMMENT_TEXT regex `([^-]|-(?!->))+` uses a negative
     * lookahead to allow single dashes. Only the sequence `-->`
     * ends the comment.
     */
    const pairs = tokenPairs("<!-- a-b-c -->");
    const text = pairs.filter(([t]) => t === "COMMENT_TEXT").map(([, v]) => v);
    expect(text).toEqual([" a-b-c "]);
  });

  it("tokenizes a comment between elements", () => {
    /**
     * Comments can appear anywhere in the document — between tags,
     * inside content, etc. The group stack handles this correctly
     * because comments push/pop independently.
     */
    const types = tokenTypes("<a/><!-- mid --><b/>");
    expect(types).toContain("COMMENT_START");
    expect(types).toContain("COMMENT_END");
  });
});

// ===========================================================================
// CDATA Sections
// ===========================================================================

describe("CDATA sections", () => {
  it("tokenizes a simple CDATA section", () => {
    /**
     * CDATA sections contain raw text — no entity processing and no
     * tag recognition. The callback pushes "cdata" and disables skip.
     * Everything between `<![CDATA[` and `]]>` becomes CDATA_TEXT.
     */
    const pairs = tokenPairs("<![CDATA[raw text]]>");
    expect(pairs).toEqual([
      ["CDATA_START", "<![CDATA["],
      ["CDATA_TEXT", "raw text"],
      ["CDATA_END", "]]>"],
    ]);
  });

  it("preserves angle brackets inside CDATA (no tag recognition)", () => {
    /**
     * Inside CDATA, `<` and `>` are just regular characters. They
     * don't trigger the tag-opening logic because the lexer is in
     * the "cdata" group, which only recognizes CDATA_TEXT and CDATA_END.
     */
    const pairs = tokenPairs("<![CDATA[<not a tag>]]>");
    const text = pairs.filter(([t]) => t === "CDATA_TEXT").map(([, v]) => v);
    expect(text).toEqual(["<not a tag>"]);
  });

  it("preserves whitespace inside CDATA (skip disabled)", () => {
    /**
     * Like comments, CDATA sections have skip patterns disabled.
     * Spaces, newlines, and tabs are preserved verbatim.
     */
    const pairs = tokenPairs("<![CDATA[  hello\n  world  ]]>");
    const text = pairs.filter(([t]) => t === "CDATA_TEXT").map(([, v]) => v);
    expect(text).toEqual(["  hello\n  world  "]);
  });

  it("handles single brackets inside CDATA (needs ]]> to end)", () => {
    /**
     * The CDATA_TEXT regex `([^\]]|\](?!\]>))+` uses a negative
     * lookahead. A single `]` doesn't end the section — only `]]>`
     * terminates it.
     */
    const pairs = tokenPairs("<![CDATA[a]b]]>");
    const text = pairs.filter(([t]) => t === "CDATA_TEXT").map(([, v]) => v);
    expect(text).toEqual(["a]b"]);
  });
});

// ===========================================================================
// Processing Instructions
// ===========================================================================

describe("processing instructions", () => {
  it("tokenizes the XML declaration: <?xml version=\"1.0\"?>", () => {
    /**
     * The XML declaration is the most common processing instruction.
     * PI_START pushes the "pi" group and disables skip. PI_TARGET
     * matches the target name ("xml"), and PI_TEXT matches everything
     * until `?>`.
     */
    const pairs = tokenPairs('<?xml version="1.0"?>');
    expect(pairs).toEqual([
      ["PI_START", "<?"],
      ["PI_TARGET", "xml"],
      ["PI_TEXT", ' version="1.0"'],
      ["PI_END", "?>"],
    ]);
  });

  it("tokenizes a stylesheet processing instruction", () => {
    /**
     * Any valid XML name can be a PI target. The `xml-stylesheet`
     * target is commonly used to link XSLT stylesheets.
     */
    const types = tokenTypes('<?xml-stylesheet type="text/xsl"?>');
    expect(types[0]).toBe("PI_START");
    expect(types[1]).toBe("PI_TARGET");
    expect(types[types.length - 1]).toBe("PI_END");
  });
});

// ===========================================================================
// Entity and Character References
// ===========================================================================

describe("entity and character references", () => {
  it("tokenizes a named entity reference: &amp;", () => {
    /**
     * Entity references like `&amp;`, `&lt;`, `&gt;` are recognized
     * in the default group. Text before and after the entity becomes
     * separate TEXT tokens.
     */
    const pairs = tokenPairs("a&amp;b");
    expect(pairs).toEqual([
      ["TEXT", "a"],
      ["ENTITY_REF", "&amp;"],
      ["TEXT", "b"],
    ]);
  });

  it("tokenizes a decimal character reference: &#65;", () => {
    /**
     * Decimal character references use the format `&#digits;`.
     * For example, `&#65;` represents the letter 'A' (ASCII 65).
     */
    const pairs = tokenPairs("&#65;");
    expect(pairs).toEqual([["CHAR_REF", "&#65;"]]);
  });

  it("tokenizes a hexadecimal character reference: &#x41;", () => {
    /**
     * Hexadecimal character references use `&#xHEX;` format.
     * `&#x41;` is also 'A' (0x41 = 65 decimal).
     */
    const pairs = tokenPairs("&#x41;");
    expect(pairs).toEqual([["CHAR_REF", "&#x41;"]]);
  });

  it("tokenizes multiple entity references in text", () => {
    /**
     * When multiple entities appear in text, the lexer alternates
     * between ENTITY_REF and TEXT tokens as needed.
     */
    const types = tokenTypes("&lt;hello&gt;");
    expect(types).toEqual(["ENTITY_REF", "TEXT", "ENTITY_REF"]);
  });
});

// ===========================================================================
// Nested and Mixed Content
// ===========================================================================

describe("nested and mixed content", () => {
  it("tokenizes nested elements: <a><b>text</b></a>", () => {
    /**
     * Nested elements push and pop the "tag" group multiple times.
     * The group stack handles this correctly because each open tag
     * pushes and each close tag pops independently.
     */
    const types = tokenTypes("<a><b>text</b></a>");
    expect(types.filter((t) => t === "OPEN_TAG_START").length).toBe(2);
    expect(types.filter((t) => t === "CLOSE_TAG_START").length).toBe(2);
  });

  it("tokenizes mixed content: text interspersed with elements", () => {
    /**
     * Mixed content is the hallmark of XML documents: text and child
     * elements coexist. Each text segment becomes its own TEXT token.
     */
    const pairs = tokenPairs("<p>Hello <b>world</b>!</p>");
    const texts = pairs.filter(([t]) => t === "TEXT").map(([, v]) => v);
    expect(texts).toEqual(["Hello ", "world", "!"]);
  });

  it("tokenizes a complete XML document with PI, comments, and entities", () => {
    /**
     * A realistic XML document exercises all five pattern groups:
     * default (text + entities), tag (names + attributes),
     * comment, and pi. This test verifies they all work together.
     */
    const source =
      '<?xml version="1.0"?>' +
      "<!-- A greeting -->" +
      '<root lang="en">' +
      "<greeting>Hello &amp; welcome</greeting>" +
      "</root>";
    const tokens = tokenizeXML(source);
    const types = tokens.map((t) => t.type);

    // PI present
    expect(types).toContain("PI_START");
    expect(types).toContain("PI_END");

    // Comment present
    expect(types).toContain("COMMENT_START");
    expect(types).toContain("COMMENT_END");

    // Tags present (root + greeting = 2 opens, 2 closes)
    expect(types.filter((t) => t === "OPEN_TAG_START").length).toBe(2);
    expect(types.filter((t) => t === "CLOSE_TAG_START").length).toBe(2);

    // Entity ref present
    expect(types).toContain("ENTITY_REF");

    // Ends with EOF
    expect(types[types.length - 1]).toBe("EOF");
  });

  it("tokenizes CDATA inside an element", () => {
    /**
     * CDATA sections can appear anywhere content is allowed. Inside
     * an element, the lexer transitions from default -> cdata -> default
     * seamlessly.
     */
    const source = "<script><![CDATA[x < y]]></script>";
    const types = tokenTypes(source);
    expect(types).toContain("CDATA_START");
    expect(types).toContain("CDATA_TEXT");
    expect(types).toContain("CDATA_END");
  });
});

// ===========================================================================
// Edge Cases
// ===========================================================================

describe("edge cases", () => {
  it("produces only EOF for empty input", () => {
    /**
     * An empty string has no tokens to produce. The lexer immediately
     * appends the EOF sentinel and returns.
     */
    const tokens = tokenizeXML("");
    expect(tokens.length).toBe(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("tokenizes plain text with no tags", () => {
    /**
     * XML doesn't require tags — plain text is valid content in the
     * default group. The TEXT pattern `[^<&]+` matches everything
     * that isn't `<` or `&`.
     */
    const pairs = tokenPairs("just text");
    expect(pairs).toEqual([["TEXT", "just text"]]);
  });

  it("skips whitespace between tags (consumed by skip pattern)", () => {
    /**
     * The XML grammar has a skip pattern for whitespace (`/[ \t\r\n]+/`).
     * In the default group (between tags), whitespace is silently consumed.
     * No TEXT tokens are emitted for inter-tag whitespace.
     *
     * Note: Inside comments, CDATA, and PIs, skip is disabled so
     * whitespace IS preserved. But in the default group, it's skipped.
     */
    const pairs = tokenPairs("<a> <b> </b> </a>");
    const texts = pairs.filter(([t]) => t === "TEXT").map(([, v]) => v);
    expect(texts).toEqual([]);
  });

  it("always ends with an EOF token", () => {
    /**
     * Every token stream ends with an EOF sentinel. This is a contract
     * of the GrammarLexer that parsers rely on.
     */
    const tokens = tokenizeXML("<root/>");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });
});

// ===========================================================================
// Position Tracking
// ===========================================================================

describe("position tracking", () => {
  it("tracks line and column for tokens", () => {
    /**
     * Every token includes position information: the line number and
     * column number where it starts. This is essential for error
     * reporting in downstream parsers.
     */
    const tokens = tokenizeXML("<a>text</a>");

    // First token: '<' is at line 1, column 1
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);
  });

  it("tracks column positions across a line", () => {
    /**
     * As we move right through the source text, column numbers
     * increase. The TAG_NAME 'div' starts at column 2 (after '<').
     */
    const tokens = tokenizeXML("<div/>");
    // '<' at column 1
    expect(tokens[0].column).toBe(1);
    // 'div' at column 2
    expect(tokens[1].column).toBe(2);
  });
});

// ===========================================================================
// createXMLLexer API
// ===========================================================================

describe("createXMLLexer", () => {
  it("returns a GrammarLexer that can be tokenized", () => {
    /**
     * The createXMLLexer function returns a configured GrammarLexer
     * instance. Calling tokenize() on it produces the token stream.
     * This is useful when you need access to the lexer object itself
     * (e.g., for inspecting group state).
     */
    const lexer = createXMLLexer("<p>hello</p>");
    const tokens = lexer.tokenize();
    expect(tokens.length).toBeGreaterThan(1);
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });
});
