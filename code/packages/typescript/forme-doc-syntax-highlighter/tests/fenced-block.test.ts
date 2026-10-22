import { expect, it } from "vitest";
import { highlightCodeBlocks } from "../src/index.js";
it("highlights fences while retaining prior decoration and empty payload", () => {
 for (const value of ["x\n", ""]) {
  const node={type:"fenced_block" as const,name:"ts",info:"ts linenos",value,copyable:true};
  expect(highlightCodeBlocks({type:"document",children:[node]}).children[0]).toEqual({...node,highlighted:value ? [{type:"plain",value}] : []});
  expect(node).not.toHaveProperty("highlighted");
 }
});
