import { expect, it } from "vitest";
import { toHtml } from "../src/index.js";
import type { DocumentNode, FencedBlockNode } from "@coding-adventures/document-ast";
const doc = (node: FencedBlockNode): DocumentNode => ({type:"document",children:[node]});
it("renders independent literal fences without interpolating info", () => {
  for (const info of ["typescript linenos", "typescript theme=dark"]) {
    expect(toHtml(doc({type:"fenced_block",name:"typescript",info,value:"<script>&x</script>\n"}))).toBe('<pre><code class="language-typescript">&lt;script&gt;&amp;x&lt;/script&gt;\n</code></pre>\n');
  }
  expect(toHtml(doc({type:"fenced_block",name:null,info:null,value:""}))).toBe("<pre><code></code></pre>\n");
});
it("escapes the language attribute independently", () => {
  expect(toHtml(doc({type:"fenced_block",name:'x"<&',info:'ignored <script>',value:"<&\n"}))).toBe('<pre><code class="language-x&quot;&lt;&amp;">&lt;&amp;\n</code></pre>\n');
});
