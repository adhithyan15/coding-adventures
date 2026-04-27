import { describe, expect, it } from "vitest";
import { layoutDoc, text } from "@coding-adventures/format-doc";
import {
  VERSION,
  blockLike,
  callLike,
  delimitedList,
  infixChain,
} from "../src/index.js";

function render(doc: ReturnType<typeof text> | Parameters<typeof layoutDoc>[0], printWidth = 80): string {
  const layout = layoutDoc(doc, { printWidth, indentWidth: 2 });
  return layout.lines
    .map((line) => {
      let out = "";
      let column = 0;

      for (const span of line.spans) {
        if (span.column > column) {
          out += " ".repeat(span.column - column);
          column = span.column;
        }
        out += span.text;
        column += span.text.length;
      }

      return out;
    })
    .join("\n");
}

describe("format-doc-std", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("delimitedList()", () => {
  it("formats an empty list without inner spacing by default", () => {
    expect(
      render(
        delimitedList({
          open: text("["),
          close: text("]"),
          items: [],
        })
      )
    ).toBe("[]");
  });

  it("formats an empty list with configurable inner spacing", () => {
    expect(
      render(
        delimitedList({
          open: text("{"),
          close: text("}"),
          items: [],
          emptySpacing: true,
        })
      )
    ).toBe("{ }");
  });

  it("keeps short lists flat", () => {
    expect(
      render(
        delimitedList({
          open: text("["),
          close: text("]"),
          items: [text("a"), text("b"), text("c")],
        })
      )
    ).toBe("[a, b, c]");
  });

  it("breaks long lists and emits a conditional trailing separator", () => {
    expect(
      render(
        delimitedList({
          open: text("["),
          close: text("]"),
          items: [text("alphabet"), text("beta"), text("gamma")],
          trailingSeparator: "ifBreak",
        }),
        12
      )
    ).toBe("[\n  alphabet,\n  beta,\n  gamma,\n]");
  });
});

describe("callLike()", () => {
  it("formats short calls flat", () => {
    expect(
      render(callLike(text("foo"), [text("a"), text("b")]))
    ).toBe("foo(a, b)");
  });

  it("breaks long calls like a delimited list", () => {
    expect(
      render(
        callLike(
          text("veryLongFunctionName"),
          [text("alpha"), text("beta")],
          { trailingSeparator: "ifBreak" }
        ),
        16
      )
    ).toBe("veryLongFunctionName(\n  alpha,\n  beta,\n)");
  });
});

describe("blockLike()", () => {
  it("formats empty blocks inline", () => {
    expect(
      render(
        blockLike({
          open: text("{"),
          body: text(""),
          close: text("}"),
        })
      )
    ).toBe("{ }");
  });

  it("keeps short blocks inline when they fit", () => {
    expect(
      render(
        blockLike({
          open: text("{"),
          body: text("x"),
          close: text("}"),
        })
      )
    ).toBe("{ x }");
  });

  it("breaks long blocks onto multiple lines", () => {
    expect(
      render(
        blockLike({
          open: text("{"),
          body: text("somethingLong"),
          close: text("}"),
        }),
        8
      )
    ).toBe("{\n  somethingLong\n}");
  });
});

describe("infixChain()", () => {
  it("formats short chains flat", () => {
    expect(
      render(
        infixChain({
          operands: [text("a"), text("b"), text("c")],
          operators: [text("+"), text("+")],
        })
      )
    ).toBe("a + b + c");
  });

  it("breaks after operators by default", () => {
    expect(
      render(
        infixChain({
          operands: [text("alpha"), text("beta"), text("gamma")],
          operators: [text("+"), text("+")],
        }),
        9
      )
    ).toBe("alpha +\n  beta +\n  gamma");
  });

  it("can break before operators", () => {
    expect(
      render(
        infixChain({
          operands: [text("alpha"), text("beta"), text("gamma")],
          operators: [text("+"), text("+")],
          breakBeforeOperators: true,
        }),
        9
      )
    ).toBe("alpha\n  + beta\n  + gamma");
  });

  it("rejects mismatched operator counts", () => {
    expect(() =>
      infixChain({
        operands: [text("a"), text("b"), text("c")],
        operators: [text("+")],
      })
    ).toThrow(/one fewer operator/);
  });
});
