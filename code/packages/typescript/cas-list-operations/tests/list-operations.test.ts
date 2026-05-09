import { describe, expect, it } from "vitest";
import { ADD, app, int, sym } from "@coding-adventures/symbolic-ir";
import { append, applyList, first, flatten, last, list, mapList, part, range, rest, reverse, select, sortList } from "../src/index";

describe("cas-list-operations", () => {
  it("handles basic accessors", () => {
    const xs = list([int(1), int(2), int(3)]);
    expect(first(xs)).toEqual(int(1));
    expect(last(xs)).toEqual(int(3));
    expect(rest(xs)).toEqual(list([int(2), int(3)]));
    expect(reverse(xs)).toEqual(list([int(3), int(2), int(1)]));
  });

  it("concatenates, indexes, and ranges", () => {
    expect(append(list([int(1)]), list([int(2), int(3)]))).toEqual(list([int(1), int(2), int(3)]));
    expect(part(list([int(1), int(2), int(3)]), -1)).toEqual(int(3));
    expect(range(1, 5, 2)).toEqual(list([int(1), int(3), int(5)]));
  });

  it("maps and applies heads", () => {
    const xs = list([sym("x"), sym("y")]);
    expect(mapList(sym("f"), xs)).toEqual(list([app(sym("f"), [sym("x")]), app(sym("f"), [sym("y")])]));
    expect(applyList(ADD, xs)).toEqual(app(ADD, [sym("x"), sym("y")]));
  });

  it("selects, sorts, and flattens", () => {
    const xs = list([int(2), list([int(1), list([int(0)])])]);
    expect(flatten(xs, -1)).toEqual(list([int(2), int(1), int(0)]));
    expect(select(list([int(1), int(2)]), (node) => node.kind === "integer" && node.value === 2n)).toEqual(list([int(2)]));
    expect(sortList(list([sym("b"), sym("a")]))).toEqual(list([sym("a"), sym("b")]));
  });
});
