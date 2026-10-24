import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "../src/index.js";
const corpus = JSON.parse(readFileSync(new URL("../../../../specs/gfm-fenced-block-v1/cases.json", import.meta.url), "utf8"));
describe("TE04 independent fenced metadata literals", () => {
  for (const c of corpus.cases) it(c.id, () => expect(parse(c.markdown).children).toEqual([c.expected]));
  it("retains indented code and nested fences", () => {
    expect(parse("    code\n").children).toEqual([{type:"code_block",language:null,value:"code\n"}]);
    expect(parse("> ```ts extra\n> x\n> ```\n").children).toEqual([{type:"blockquote",children:[{type:"fenced_block",name:"ts",info:"ts extra",value:"x\n"}]}]);
  });
});
