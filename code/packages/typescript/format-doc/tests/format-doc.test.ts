import { describe, expect, it } from "vitest";
import {
  VERSION,
  annotate,
  concat,
  group,
  hardline,
  ifBreak,
  indent,
  join,
  layoutDoc,
  line,
  nil,
  softline,
  text,
} from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("builders", () => {
  it("collapses empty text to nil", () => {
    expect(text("")).toEqual(nil());
  });

  it("flattens nested concat nodes", () => {
    const doc = concat([text("a"), concat([text("b"), text("c")])]);
    expect(doc).toEqual({
      kind: "concat",
      parts: [
        { kind: "text", value: "a" },
        { kind: "text", value: "b" },
        { kind: "text", value: "c" },
      ],
    });
  });

  it("joins docs with a separator", () => {
    const doc = join(text(", "), [text("a"), text("b"), text("c")]);
    expect(doc).toEqual(
      concat([text("a"), text(", "), text("b"), text(", "), text("c")])
    );
  });
});

describe("layoutDoc()", () => {
  it("keeps a group flat when it fits", () => {
    const doc = group(
      concat([
        text("foo("),
        indent(concat([softline(), text("bar,"), line(), text("baz")])),
        softline(),
        text(")"),
      ])
    );

    const layout = layoutDoc(doc, { printWidth: 80 });
    expect(layout.width).toBe(13);
    expect(layout.height).toBe(1);
    expect(layout.lines).toEqual([
      {
        row: 0,
        indentColumns: 0,
        width: 13,
        spans: [{ column: 0, text: "foo(bar, baz)", annotations: [] }],
      },
    ]);
  });

  it("breaks a group when it does not fit", () => {
    const doc = group(
      concat([
        text("foo("),
        indent(concat([softline(), text("bar,"), line(), text("baz")])),
        softline(),
        text(")"),
      ])
    );

    const layout = layoutDoc(doc, { printWidth: 10, indentWidth: 2 });
    expect(layout.lines).toEqual([
      { row: 0, indentColumns: 0, width: 4, spans: [{ column: 0, text: "foo(", annotations: [] }] },
      { row: 1, indentColumns: 2, width: 6, spans: [{ column: 2, text: "bar,", annotations: [] }] },
      { row: 2, indentColumns: 2, width: 5, spans: [{ column: 2, text: "baz", annotations: [] }] },
      { row: 3, indentColumns: 0, width: 1, spans: [{ column: 0, text: ")", annotations: [] }] },
    ]);
  });

  it("treats hardline as an unconditional break", () => {
    const doc = group(concat([text("a"), hardline(), text("b")]));
    const layout = layoutDoc(doc, { printWidth: 80 });
    expect(layout.lines).toEqual([
      { row: 0, indentColumns: 0, width: 1, spans: [{ column: 0, text: "a", annotations: [] }] },
      { row: 1, indentColumns: 0, width: 1, spans: [{ column: 0, text: "b", annotations: [] }] },
    ]);
  });

  it("selects the right branch for ifBreak()", () => {
    const doc = group(
      concat([
        text("["),
        indent(concat([softline(), text("value"), ifBreak(text(","))])),
        softline(),
        text("]"),
      ])
    );

    expect(layoutDoc(doc, { printWidth: 40 }).lines).toEqual([
      { row: 0, indentColumns: 0, width: 7, spans: [{ column: 0, text: "[value]", annotations: [] }] },
    ]);

    expect(layoutDoc(doc, { printWidth: 3 }).lines).toEqual([
      { row: 0, indentColumns: 0, width: 1, spans: [{ column: 0, text: "[", annotations: [] }] },
      { row: 1, indentColumns: 2, width: 8, spans: [{ column: 2, text: "value,", annotations: [] }] },
      { row: 2, indentColumns: 0, width: 1, spans: [{ column: 0, text: "]", annotations: [] }] },
    ]);
  });

  it("preserves annotation stacks on emitted spans", () => {
    const doc = concat([
      annotate("lhs", text("alpha")),
      text(" "),
      annotate({ role: "rhs" }, text("beta")),
    ]);

    const layout = layoutDoc(doc, { printWidth: 80 });
    expect(layout.lines[0]).toEqual({
      row: 0,
      indentColumns: 0,
      width: 10,
      spans: [
        { column: 0, text: "alpha", annotations: ["lhs"] },
        { column: 5, text: " ", annotations: [] },
        { column: 6, text: "beta", annotations: [{ role: "rhs" }] },
      ],
    });
  });

  it("tracks the widest realized column", () => {
    const doc = concat([text("abc"), hardline(), text("abcdef")]);
    const layout = layoutDoc(doc, { printWidth: 80 });
    expect(layout.width).toBe(6);
  });

  it("uses lineHeight to compute the layout tree height", () => {
    const doc = concat([text("x"), hardline(), text("y"), hardline(), text("z")]);
    const layout = layoutDoc(doc, { printWidth: 80, lineHeight: 2 });
    expect(layout.height).toBe(6);
  });
});
