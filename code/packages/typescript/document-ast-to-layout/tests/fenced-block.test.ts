import { expect, it } from "vitest";
import { document_ast_to_layout, document_default_theme } from "../src/index.js";
import type { DocumentNode } from "@coding-adventures/document-ast";
it("uses the exact preformatted layout fallback including empty fences", () => {
 for (const value of ["<script>&x</script>\n", ""]) {
  const fenced:DocumentNode={type:"document",children:[{type:"fenced_block",name:"unknown",info:"unknown options",value}]};
  const legacy:DocumentNode={type:"document",children:[{type:"code_block",language:"unknown",value}]};
  expect(document_ast_to_layout(fenced, document_default_theme())).toEqual(document_ast_to_layout(legacy, document_default_theme()));
 }
});
