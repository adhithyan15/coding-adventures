import { expect, it } from "vitest";
import { decorateCodeBlocks } from "../src/index.js";
import type { DocumentNode } from "@coding-adventures/document-ast";
it("decorates named and anonymous fences without losing source metadata", () => {
 for (const name of ["ts", null]) {
  const node={type:"fenced_block" as const,name,info:name ? "ts linenos" : null,value:"// file: src/a.ts\nx\n"};
  const doc:DocumentNode={type:"document",children:[node]};
  expect(decorateCodeBlocks(doc,{lineNumbers:true}).children[0]).toEqual({...node,value:"x\n",copyable:true,languageLabel:name ? "TypeScript" : null,filename:"src/a.ts",lineNumbers:true});
  expect(node.value).toBe("// file: src/a.ts\nx\n");
 }
});
