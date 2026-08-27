import { describe, expect, it } from "vitest";
import type { BlockNode } from "@coding-adventures/document-ast";
import { toHtml } from "@coding-adventures/document-ast-to-html";
import {
  applyGfmBlockExtensions,
  convertToAst,
  parseBlocks,
} from "../src/block-parser.js";
import { resolveInlineContent } from "../src/inline-parser.js";

function freezeBlockTree(block: BlockNode): void {
  switch (block.type) {
    case "document":
    case "blockquote":
    case "list_item":
    case "task_item":
      block.children.forEach(freezeBlockTree);
      Object.freeze(block.children);
      break;
    case "list":
      block.children.forEach(freezeBlockTree);
      Object.freeze(block.children);
      break;
    case "table":
      block.children.forEach(freezeBlockTree);
      Object.freeze(block.align);
      Object.freeze(block.children);
      break;
    case "table_row":
      block.children.forEach(freezeBlockTree);
      Object.freeze(block.children);
      break;
    case "heading":
    case "paragraph":
    case "table_cell":
      Object.freeze(block.children);
      break;
  }
  Object.freeze(block);
}

describe("immutable Document AST assembly", () => {
  it("returns new trees for GFM block and inline transforms", () => {
    const markdown = [
      "- [x] **done**",
      "",
      "| Name | Role |",
      "| :--- | ---: |",
      "| Ada | Math |",
      "",
    ].join("\n");
    const { document: mutableDocument, linkRefs } = parseBlocks(markdown);
    const { document, rawInlineContent } = convertToAst(mutableDocument, linkRefs);
    freezeBlockTree(document);

    const extended = applyGfmBlockExtensions(document, rawInlineContent);

    expect(extended).not.toBe(document);
    expect(document.children[1]?.type).toBe("paragraph");
    expect(extended.children[1]?.type).toBe("table");
    freezeBlockTree(extended);

    const resolved = resolveInlineContent(extended, rawInlineContent, linkRefs);

    expect(resolved).not.toBe(extended);
    expect(toHtml(resolved)).toBe(
      '<ul>\n<li><input type="checkbox" disabled="" checked="" /> <strong>done</strong></li>\n</ul>\n' +
      '<table>\n<thead>\n<tr>\n<th align="left">Name</th>\n<th align="right">Role</th>\n</tr>\n</thead>\n' +
      '<tbody>\n<tr>\n<td align="left">Ada</td>\n<td align="right">Math</td>\n</tr>\n</tbody>\n</table>\n',
    );
  });
});
