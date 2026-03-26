/**
 * csv-parser.test.ts — comprehensive vitest test suite for the CSV parser.
 *
 * Tests are organised by feature area:
 *  1. Basic parsing
 *  2. Quoted fields
 *  3. Empty fields
 *  4. Ragged rows
 *  5. Edge cases (empty/header-only input)
 *  6. Line endings (\n, \r\n, \r)
 *  7. Custom delimiters
 *  8. Error handling
 *  9. Whitespace preservation
 * 10. UnclosedQuoteError class
 */

import { describe, it, expect } from "vitest";
import {
  parseCSV,
  parseCSVWithDelimiter,
  UnclosedQuoteError,
} from "../src/index.js";

// =============================================================================
// 1. Basic parsing
// =============================================================================

describe("basic parsing", () => {
  it("parses a simple three-column table", () => {
    const csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(2);
    expect(rows[0]).toEqual({ name: "Alice", age: "30", city: "New York" });
    expect(rows[1]).toEqual({ name: "Bob", age: "25", city: "London" });
  });

  it("returns all values as strings, not numbers", () => {
    // Emphasise the type-agnostic contract: even numeric-looking fields are strings.
    const csv = "x,y\n1,2\n";
    const rows = parseCSV(csv);

    expect(rows[0]["x"]).toBe("1"); // "1", not 1
    expect(rows[0]["y"]).toBe("2");
    expect(typeof rows[0]["x"]).toBe("string");
  });

  it("handles no trailing newline (RFC 4180 allows this)", () => {
    const csv = "name,value\nhello,world";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(1);
    expect(rows[0]["name"]).toBe("hello");
    expect(rows[0]["value"]).toBe("world");
  });

  it("parses a single-column file", () => {
    const csv = "fruit\napple\nbanana\ncherry\n";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(3);
    expect(rows[0]["fruit"]).toBe("apple");
    expect(rows[1]["fruit"]).toBe("banana");
    expect(rows[2]["fruit"]).toBe("cherry");
  });

  it("parses many rows correctly", () => {
    const lines = ["id,value"];
    for (let i = 1; i <= 100; i++) {
      lines.push(`${i},item${i}`);
    }
    const csv = lines.join("\n") + "\n";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(100);
    expect(rows[0]["id"]).toBe("1");
    expect(rows[0]["value"]).toBe("item1");
    expect(rows[99]["id"]).toBe("100");
    expect(rows[99]["value"]).toBe("item100");
  });
});

// =============================================================================
// 2. Quoted fields
// =============================================================================

describe("quoted fields", () => {
  it("quoted field with embedded comma", () => {
    // The core use-case: commas inside quotes are NOT delimiters.
    const csv =
      'product,price,description\nWidget,9.99,"A small, round widget"\n';
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(1);
    expect(rows[0]["description"]).toBe("A small, round widget");
  });

  it("quoted field with embedded newline", () => {
    // Newlines inside quotes are literal characters, not row separators.
    const csv = 'id,note\n1,"Line one\nLine two"\n2,Single line\n';
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(2);
    expect(rows[0]["note"]).toBe("Line one\nLine two");
    expect(rows[1]["note"]).toBe("Single line");
  });

  it("escaped double-quote (\"\" inside quoted field → single \")", () => {
    const csv = 'id,value\n1,"She said ""hello"""\n2,plain\n';
    const rows = parseCSV(csv);

    expect(rows[0]["value"]).toBe('She said "hello"');
    expect(rows[1]["value"]).toBe("plain");
  });

  it("empty quoted field (\"\" → empty string)", () => {
    const csv = 'a,b,c\n1,"",3\n';
    const rows = parseCSV(csv);

    expect(rows[0]["b"]).toBe("");
  });

  it("all fields quoted", () => {
    const csv = '"name","age"\n"Alice","30"\n';
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(1);
    expect(rows[0]["name"]).toBe("Alice");
    expect(rows[0]["age"]).toBe("30");
  });

  it("quoted field at the start of a row", () => {
    const csv = 'a,b\n"quoted start",normal\n';
    const rows = parseCSV(csv);

    expect(rows[0]["a"]).toBe("quoted start");
    expect(rows[0]["b"]).toBe("normal");
  });

  it("quoted field at the end of a row", () => {
    const csv = 'a,b\nnormal,"quoted end"\n';
    const rows = parseCSV(csv);

    expect(rows[0]["b"]).toBe("quoted end");
  });

  it("quoted field containing only a double-quote (\"\"\"\" → \")", () => {
    // The field value is a single literal double-quote character.
    const csv = 'a,b\n1,""""""\n';
    // """""" is: open-quote, then "", "", close-quote → field value is ""
    const rows = parseCSV(csv);
    expect(rows[0]["b"]).toBe('""');
  });

  it("quoted field containing the delimiter character many times", () => {
    const csv = 'a,b\n1,"x,y,z,w"\n';
    const rows = parseCSV(csv);
    expect(rows[0]["b"]).toBe("x,y,z,w");
  });
});

// =============================================================================
// 3. Empty fields
// =============================================================================

describe("empty fields", () => {
  it("empty field in the middle (a,,b → three fields)", () => {
    const csv = "a,b,c\n1,,3\n";
    const rows = parseCSV(csv);

    expect(rows[0]["a"]).toBe("1");
    expect(rows[0]["b"]).toBe("");
    expect(rows[0]["c"]).toBe("3");
  });

  it("empty leading and trailing fields (,2, → three fields)", () => {
    const csv = "a,b,c\n,2,\n";
    const rows = parseCSV(csv);

    expect(rows[0]["a"]).toBe("");
    expect(rows[0]["b"]).toBe("2");
    expect(rows[0]["c"]).toBe("");
  });

  it("all empty fields (,, → three empty strings)", () => {
    const csv = "a,b,c\n,,\n";
    const rows = parseCSV(csv);

    expect(rows[0]["a"]).toBe("");
    expect(rows[0]["b"]).toBe("");
    expect(rows[0]["c"]).toBe("");
  });

  it("single empty field (just a newline after header)", () => {
    const csv = "a\n\n";
    // The empty line after the header is a row with one empty field.
    const rows = parseCSV(csv);
    // The empty line produces an empty row; since it has no fields at all,
    // it gets padded to {"a": ""}
    expect(rows).toHaveLength(1);
    expect(rows[0]["a"]).toBe("");
  });
});

// =============================================================================
// 4. Ragged rows
// =============================================================================

describe("ragged rows", () => {
  it("short row is padded with empty strings", () => {
    // Row has 2 fields but header has 3. Missing "city" should be "".
    const csv = "name,age,city\nAlice,30\n";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(1);
    expect(rows[0]["name"]).toBe("Alice");
    expect(rows[0]["age"]).toBe("30");
    expect(rows[0]["city"]).toBe("");
  });

  it("very short row (only one field) padded for remaining columns", () => {
    const csv = "a,b,c,d\nonly\n";
    const rows = parseCSV(csv);

    expect(rows[0]["a"]).toBe("only");
    expect(rows[0]["b"]).toBe("");
    expect(rows[0]["c"]).toBe("");
    expect(rows[0]["d"]).toBe("");
  });

  it("long row is truncated to header length", () => {
    const csv = "a,b,c\n1,2,3,4,5\n";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(1);
    expect(rows[0]["a"]).toBe("1");
    expect(rows[0]["b"]).toBe("2");
    expect(rows[0]["c"]).toBe("3");
    // Fields "4" and "5" have no column name; they are silently discarded.
    expect(Object.keys(rows[0])).toHaveLength(3);
  });

  it("mixed ragged rows", () => {
    const csv = "a,b,c\n1\n2,two\n3,three,THREE\n4,four,FOUR,extra\n";
    const rows = parseCSV(csv);

    expect(rows).toHaveLength(4);
    expect(rows[0]).toEqual({ a: "1", b: "", c: "" });
    expect(rows[1]).toEqual({ a: "2", b: "two", c: "" });
    expect(rows[2]).toEqual({ a: "3", b: "three", c: "THREE" });
    expect(rows[3]).toEqual({ a: "4", b: "four", c: "FOUR" });
  });
});

// =============================================================================
// 5. Edge cases
// =============================================================================

describe("edge cases", () => {
  it("empty string returns empty array", () => {
    expect(parseCSV("")).toEqual([]);
  });

  it("header-only with trailing newline returns empty array", () => {
    expect(parseCSV("name,age,city\n")).toEqual([]);
  });

  it("header-only without trailing newline returns empty array", () => {
    expect(parseCSV("name,age")).toEqual([]);
  });

  it("single-cell CSV (one column, one row)", () => {
    const csv = "x\nhello\n";
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(1);
    expect(rows[0]["x"]).toBe("hello");
  });

  it("single-cell CSV without trailing newline", () => {
    const csv = "x\nhello";
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(1);
    expect(rows[0]["x"]).toBe("hello");
  });
});

// =============================================================================
// 6. Line endings
// =============================================================================

describe("line endings", () => {
  it("Unix LF (\\n)", () => {
    const csv = "name,age\nAlice,30\nBob,25\n";
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(2);
    expect(rows[0]["name"]).toBe("Alice");
  });

  it("Windows CRLF (\\r\\n)", () => {
    const csv = "name,age\r\nAlice,30\r\nBob,25\r\n";
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(2);
    expect(rows[0]["name"]).toBe("Alice");
    expect(rows[1]["name"]).toBe("Bob");
  });

  it("old Mac CR (\\r only)", () => {
    const csv = "name,age\rAlice,30\rBob,25\r";
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(2);
    expect(rows[0]["name"]).toBe("Alice");
    expect(rows[1]["name"]).toBe("Bob");
  });

  it("CRLF does not produce empty rows between data rows", () => {
    const csv = "a,b\r\n1,2\r\n3,4\r\n";
    const rows = parseCSV(csv);
    // Should be exactly 2 data rows, not 4 (which would happen if \r\n → 2 newlines).
    expect(rows).toHaveLength(2);
  });

  it("embedded \\n inside quoted field is preserved", () => {
    const csv = 'id,note\n1,"first\nsecond"\n';
    const rows = parseCSV(csv);
    expect(rows[0]["note"]).toBe("first\nsecond");
  });

  it("embedded CRLF inside quoted field is preserved", () => {
    const csv = 'id,note\n1,"first\r\nsecond"\n';
    const rows = parseCSV(csv);
    // The \r\n inside the quoted field is literal (not a row separator).
    expect(rows[0]["note"]).toBe("first\r\nsecond");
  });
});

// =============================================================================
// 7. Custom delimiters
// =============================================================================

describe("custom delimiters", () => {
  it("tab delimiter (TSV)", () => {
    const tsv = "name\tage\nAlice\t30\nBob\t25\n";
    const rows = parseCSVWithDelimiter(tsv, "\t");

    expect(rows).toHaveLength(2);
    expect(rows[0]["name"]).toBe("Alice");
    expect(rows[0]["age"]).toBe("30");
    expect(rows[1]["name"]).toBe("Bob");
  });

  it("semicolon delimiter (European CSV)", () => {
    const csv = "name;age;city\nAlice;30;Paris\n";
    const rows = parseCSVWithDelimiter(csv, ";");

    expect(rows).toHaveLength(1);
    expect(rows[0]["name"]).toBe("Alice");
    expect(rows[0]["city"]).toBe("Paris");
  });

  it("pipe delimiter", () => {
    const csv = "a|b|c\n1|2|3\n";
    const rows = parseCSVWithDelimiter(csv, "|");

    expect(rows).toHaveLength(1);
    expect(rows[0]["a"]).toBe("1");
    expect(rows[0]["b"]).toBe("2");
    expect(rows[0]["c"]).toBe("3");
  });

  it("comma is still treated as a literal when delimiter is tab", () => {
    // With tab as delimiter, commas are just normal characters.
    const tsv = "a\tb\n1,2\t3,4\n";
    const rows = parseCSVWithDelimiter(tsv, "\t");

    // Field "a" should be "1,2" (comma is not a delimiter here).
    expect(rows[0]["a"]).toBe("1,2");
    expect(rows[0]["b"]).toBe("3,4");
  });
});

// =============================================================================
// 8. Error handling
// =============================================================================

describe("error handling", () => {
  it("throws UnclosedQuoteError for unclosed quoted field", () => {
    const csv = 'name,value\n1,"unclosed\n';

    expect(() => parseCSV(csv)).toThrow(UnclosedQuoteError);
  });

  it("thrown error has the correct message", () => {
    const csv = 'a,b\n1,"never closed';

    expect(() => parseCSV(csv)).toThrow(
      "Unclosed quoted field: EOF reached inside a quoted field"
    );
  });

  it("thrown error has name UnclosedQuoteError", () => {
    const csv = 'a,b\n1,"never closed';

    try {
      parseCSV(csv);
      expect.fail("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(UnclosedQuoteError);
      expect((e as UnclosedQuoteError).name).toBe("UnclosedQuoteError");
    }
  });

  it("unclosed quote at the very start of input throws", () => {
    expect(() => parseCSV('"never closed')).toThrow(UnclosedQuoteError);
  });
});

// =============================================================================
// 9. Whitespace preservation
// =============================================================================

describe("whitespace preservation", () => {
  it("spaces around unquoted fields are preserved", () => {
    // Per spec: whitespace is significant. "  hello  " stays "  hello  ".
    const csv = "key,value\nspaced,  hello  \n";
    const rows = parseCSV(csv);

    expect(rows[0]["value"]).toBe("  hello  ");
  });

  it("spaces inside quoted fields are preserved", () => {
    const csv = 'key,value\nspaced,"  hello  "\n';
    const rows = parseCSV(csv);

    expect(rows[0]["value"]).toBe("  hello  ");
  });

  it("tab characters inside unquoted fields are preserved", () => {
    const csv = "key,value\nwith tab,\there\n";
    const rows = parseCSV(csv);
    expect(rows[0]["value"]).toBe("\there");
  });
});

// =============================================================================
// 10. UnclosedQuoteError class
// =============================================================================

describe("UnclosedQuoteError", () => {
  it("is an instance of Error", () => {
    const e = new UnclosedQuoteError();
    expect(e).toBeInstanceOf(Error);
  });

  it("is an instance of UnclosedQuoteError", () => {
    const e = new UnclosedQuoteError();
    expect(e).toBeInstanceOf(UnclosedQuoteError);
  });

  it("has the correct name property", () => {
    const e = new UnclosedQuoteError();
    expect(e.name).toBe("UnclosedQuoteError");
  });

  it("has the correct message", () => {
    const e = new UnclosedQuoteError();
    expect(e.message).toBe(
      "Unclosed quoted field: EOF reached inside a quoted field"
    );
  });
});

// =============================================================================
// 11. Integration / realistic data tests
// =============================================================================

describe("integration tests", () => {
  it("parses a realistic products table", () => {
    const csv = [
      "product,price,description,in_stock",
      'Widget,9.99,"A small, round widget",true',
      "Gadget,19.99,Electronic device,false",
      'Doohickey,4.50,"Says ""hello""",true',
    ].join("\n") + "\n";

    const rows = parseCSV(csv);

    expect(rows).toHaveLength(3);
    expect(rows[0]).toEqual({
      product: "Widget",
      price: "9.99",
      description: "A small, round widget",
      in_stock: "true",
    });
    expect(rows[1]).toEqual({
      product: "Gadget",
      price: "19.99",
      description: "Electronic device",
      in_stock: "false",
    });
    expect(rows[2]["description"]).toBe('Says "hello"');
  });

  it("spec example 1 — simple three-column table", () => {
    const csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
    const rows = parseCSV(csv);

    expect(rows[0]).toEqual({ name: "Alice", age: "30", city: "New York" });
    expect(rows[1]).toEqual({ name: "Bob", age: "25", city: "London" });
  });

  it("spec example 3 — quoted field with embedded newline", () => {
    const csv = 'id,note\n1,"Line one\nLine two"\n2,Single line\n';
    const rows = parseCSV(csv);

    expect(rows[0]["note"]).toBe("Line one\nLine two");
    expect(rows[1]["note"]).toBe("Single line");
  });

  it("spec example 4 — escaped double-quote", () => {
    const csv = 'id,value\n1,"She said ""hello"""\n2,plain\n';
    const rows = parseCSV(csv);

    expect(rows[0]["value"]).toBe('She said "hello"');
  });

  it("spec example 5 — empty fields", () => {
    const csv = "a,b,c\n1,,3\n,2,\n";
    const rows = parseCSV(csv);

    expect(rows[0]).toEqual({ a: "1", b: "", c: "3" });
    expect(rows[1]).toEqual({ a: "", b: "2", c: "" });
  });

  it("spec example 6 — tab-delimited (TSV)", () => {
    const tsv = "name\tage\nAlice\t30\n";
    const rows = parseCSVWithDelimiter(tsv, "\t");

    expect(rows[0]).toEqual({ name: "Alice", age: "30" });
  });

  it("quoted field ending at EOF without trailing newline", () => {
    // Tests the IN_QUOTED_MAYBE_END state at EOF path.
    // The last character is '"' which closes the quoted field.
    const csv = 'a,b\n1,"hello"';
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(1);
    expect(rows[0]["b"]).toBe("hello");
  });

  it("lenient mode: quoted field followed by unexpected character", () => {
    // Tests the lenient fallthrough in IN_QUOTED_MAYBE_END.
    // "hello"world is malformed but we handle it gracefully.
    const csv = 'a,b\n1,"hello"world\n';
    const rows = parseCSV(csv);
    expect(rows).toHaveLength(1);
    // The parser closes the quote at '"', then appends 'world' in unquoted mode.
    expect(rows[0]["b"]).toBe("helloworld");
  });
});
