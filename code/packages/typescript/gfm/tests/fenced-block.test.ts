import { expect, it } from "vitest";
import { parse, toHtml } from "../src/index.js";
it("exposes source-preserving fences through the facade", () => {
 const doc=parse("```ts extra\nx\n```\n");
 expect(doc.children).toEqual([{type:"fenced_block",name:"ts",info:"ts extra",value:"x\n"}]);
 expect(toHtml(doc)).toBe('<pre><code class="language-ts">x\n</code></pre>\n');
});
