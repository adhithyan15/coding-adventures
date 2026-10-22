import { expect, it } from "vitest";
import { toHtml } from "../src/index.js";
import type { BlockNode, ParagraphNode, ListChildNode, TableRowNode } from "@coding-adventures/document-ast";
const para=(s:string):ParagraphNode=>({type:"paragraph",children:[{type:"text",value:s}]});
const html=(...children:BlockNode[])=>toHtml({type:"document",children});
const list=(...children:ListChildNode[]):BlockNode=>({type:"list",ordered:false,start:null,tight:true,children});
it("renders standalone blocks and list/task boundaries", () => {
 expect(html({type:"document",children:[para("nested")]})).toBe("<p>nested</p>\n");
 expect(html({type:"list_item",children:[]})).toBe("<li></li>\n");
 expect(html({type:"task_item",checked:true,children:[]})).toBe('<li><input type="checkbox" disabled="" checked="" /> </li>\n');
 expect(html({type:"task_item",checked:false,children:[para("x")]})).toBe('<li><input type="checkbox" disabled="" /> \n<p>x</p>\n</li>\n');
 expect(html(list({type:"list_item",children:[para("x"),{type:"thematic_break"}]}))).toBe("<ul>\n<li>x\n<hr />\n</li>\n</ul>\n");
 expect(html(list({type:"list_item",children:[{type:"thematic_break"},para("last")]}))).toBe("<ul>\n<li>\n<hr />\nlast</li>\n</ul>\n");
 expect(html(list({type:"task_item",checked:false,children:[para("x"),{type:"thematic_break"}]}))).toBe('<ul>\n<li><input type="checkbox" disabled="" /> x\n<hr />\n</li>\n</ul>\n');
});
it("renders table shapes with independent literal outputs", () => {
 const cell={type:"table_cell" as const,children:[{type:"text" as const,value:"A"}]};
 const row:TableRowNode={type:"table_row",isHeader:true,children:[cell]};
 expect(html(cell)).toBe("<td>A</td>\n"); expect(html(row)).toBe("<tr>\n<th>A</th>\n</tr>\n");
 expect(html({type:"table",align:[],children:[]})).toBe("<table>\n</table>\n");
 expect(html({type:"table",align:[],children:[row]})).toBe("<table>\n<thead>\n<tr>\n<th>A</th>\n</tr>\n</thead>\n</table>\n");
 expect(html({type:"table",align:[],children:[{...row,isHeader:false}]})).toBe("<table>\n<tbody>\n<tr>\n<td>A</td>\n</tr>\n</tbody>\n</table>\n");
});
it("suppresses raw nodes under sanitize and percent-encodes autolinks", () => {
 expect(toHtml({type:"document",children:[{type:"raw_block",format:"html",value:"<script>"},{type:"paragraph",children:[{type:"raw_inline",format:"html",value:"<img>"},{type:"text",value:"safe"}]}]},{sanitize:true})).toBe("<p>safe</p>\n");
 expect(html({type:"paragraph",children:[{type:"autolink",destination:"https://example.test/a b?x=λ",isEmail:false}]})).toBe('<p><a href="https://example.test/a%20b?x=%CE%BB">https://example.test/a b?x=λ</a></p>\n');
});
