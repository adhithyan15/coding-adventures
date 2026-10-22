import { expect, it } from "vitest";
import { sanitize, STRICT } from "../src/index.js";
import type { DocumentNode } from "@coding-adventures/document-ast";
it("preserves inert fence fields and applies the existing drop-code policy", () => {
 const doc:DocumentNode={type:"document",children:[{type:"fenced_block",name:"mermaid",info:"mermaid <script>",value:"<script>&x</script>\n"}]};
 const before=JSON.stringify(doc);
 expect(sanitize(doc,STRICT)).toEqual(doc);
 expect(sanitize(doc,{...STRICT,dropCodeBlocks:true})).toEqual({type:"document",children:[]});
 expect(JSON.stringify(doc)).toBe(before);
});
