import { describe, expect, it } from "vitest";
import { concat, group, indent, layoutDoc, line, softline, text } from "@coding-adventures/format-doc";
import { VERSION, renderDocToText, renderLayoutToText } from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("renderLayoutToText()", () => {
  it("serializes lines with space indentation", () => {
    const layout = layoutDoc(
      group(
        concat([
          text("foo("),
          indent(concat([softline(), text("bar,"), line(), text("baz")])),
          softline(),
          text(")"),
        ])
      ),
      { printWidth: 10, indentWidth: 2 }
    );

    expect(renderLayoutToText(layout)).toBe("foo(\n  bar,\n  baz\n)");
  });

  it("serializes indentation with tabs when requested", () => {
    const layout = layoutDoc(
      concat([
        text("root"),
        indent(concat([line(), text("child")]), 2),
      ]),
      { printWidth: 4, indentWidth: 2, useTabs: true }
    );

    expect(renderLayoutToText(layout)).toBe("root\n\t\tchild");
  });
});

describe("renderDocToText()", () => {
  it("renders a flat doc directly", () => {
    const doc = group(
      concat([
        text("["),
        indent(concat([softline(), text("a,"), line(), text("b")])),
        softline(),
        text("]"),
      ])
    );

    expect(renderDocToText(doc, { printWidth: 40 })).toBe("[a, b]");
  });
});
