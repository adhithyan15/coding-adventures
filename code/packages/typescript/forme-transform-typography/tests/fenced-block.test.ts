import { expect, it } from "vitest";
import { typography } from "../src/index.js";
it("preserves opaque fence metadata and payload", () => {
 const node={type:"fenced_block" as const,name:"ts",info:"ts -- extra",value:'"quotes" -- [link](slug)\n'};
 const doc={type:"document" as const,children:[node]};
 const result=typography(doc);
 expect(result).toEqual(doc);
 expect(result.children[0]).not.toBe(node);
});
