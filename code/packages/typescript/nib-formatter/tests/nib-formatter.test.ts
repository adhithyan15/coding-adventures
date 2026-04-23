import { describe, expect, it } from "vitest";
import { parseNib } from "@coding-adventures/nib-parser";
import { VERSION, formatNib, formatNibAst, printNibDoc, printNibSourceToDoc } from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("printNibDoc()", () => {
  it("rejects non-program AST roots", () => {
    const ast = parseNib("fn main() { return 0; }");
    const fnDecl = ast.children[0];
    expect(() => printNibDoc(fnDecl as never)).toThrow("expects a 'program' AST node");
  });

  it("lowers parsed source to a non-empty Doc tree", () => {
    const doc = printNibSourceToDoc("fn main() { return 0; }");
    expect(doc.kind).toBe("group");
  });
});

describe("formatNib()", () => {
  it("formats an empty program as the empty string", () => {
    expect(formatNib("")).toBe("");
  });

  it("normalizes ugly spacing in a simple function", () => {
    expect(formatNib("fn   main( ){let x:u4=5;return x;}")).toBe(
      "fn main() {\n  let x: u4 = 5;\n  return x;\n}",
    );
  });

  it("separates top-level declarations with a blank line", () => {
    expect(
      formatNib("const MAX:u4=9;static total:u8=0;fn main(){return total;}"),
    ).toBe(
      "const MAX: u4 = 9;\n\nstatic total: u8 = 0;\n\nfn main() {\n  return total;\n}",
    );
  });

  it("wraps long parameter and argument lists when the print width is small", () => {
    expect(
      formatNib(
        "fn totalize(alpha:u8,beta:u8,gamma:u8){return sum(alpha,beta,gamma);}",
        { printWidth: 24 },
      ),
    ).toBe(
      "fn totalize(\n  alpha: u8,\n  beta: u8,\n  gamma: u8\n) {\n  return sum(\n    alpha,\n    beta,\n    gamma\n  );\n}",
    );
  });

  it("formats for-loops and if/else blocks canonically", () => {
    expect(
      formatNib(
        "fn main(){for i:u8 in 0..10{if ready{i=i+%1;}else{reset();}}}",
      ),
    ).toBe(
      "fn main() {\n  for i: u8 in 0..10 {\n    if ready {\n      i = i +% 1;\n    } else {\n      reset();\n    }\n  }\n}",
    );
  });

  it("preserves explicit parentheses while still spacing infix chains canonically", () => {
    expect(formatNib("fn main(){return (a+b)&c;}")).toBe(
      "fn main() {\n  return (a + b) & c;\n}",
    );
  });

  it("formats unary operators tightly against their operand", () => {
    expect(formatNib("fn main(){return !ready;}")).toBe(
      "fn main() {\n  return !ready;\n}",
    );
  });

  it("uses the shared infix-chain printer for longer operator chains", () => {
    expect(formatNib("fn main(){return a+b+c;}")).toBe(
      "fn main() {\n  return a +\n    b +\n    c;\n}",
    );
  });

  it("is idempotent on already-formatted programs", () => {
    const formatted = formatNib(
      "fn main() {\n  let total: u8 = sum(alpha, beta, gamma);\n  return total;\n}",
      { printWidth: 80 },
    );

    expect(formatNib(formatted, { printWidth: 80 })).toBe(formatted);
  });

  it("formats a parsed AST through the same end-to-end path", () => {
    const ast = parseNib("fn main(){return 0;}");
    expect(formatNibAst(ast, { printWidth: 80 })).toBe("fn main() {\n  return 0;\n}");
  });

  it("preserves top-level comments and blank lines through the source-based path", () => {
    expect(
      formatNib("// file header\nconst MAX:u4=9;\n\n// entry point\nfn main(){return MAX;}"),
    ).toBe(
      "// file header\nconst MAX: u4 = 9;\n\n// entry point\nfn main() {\n  return MAX;\n}",
    );
  });

  it("preserves trailing statement comments and comments before closing braces", () => {
    expect(
      formatNib("fn main(){let x:u4=5; // keep me\n// before closing\n}"),
    ).toBe(
      "fn main() {\n  let x: u4 = 5; // keep me\n  // before closing\n}",
    );
  });

  it("preserves comments before else blocks and at end of file", () => {
    expect(
      formatNib("fn main(){if ready{return 1;} // not ready yet\nelse{return 0;}} // eof"),
    ).toBe(
      "fn main() {\n  if ready {\n    return 1;\n  } // not ready yet\n  else {\n    return 0;\n  }\n} // eof",
    );
  });

  it("fails loudly on unsupported AST rule names", () => {
    const ast = {
      ruleName: "program",
      children: [
        {
          ruleName: "top_decl",
          children: [{ ruleName: "mystery_decl", children: [] }],
        },
      ],
    };

    expect(() => printNibDoc(ast as never)).toThrow("Unsupported top-level Nib rule 'mystery_decl'");
  });

  it("fails loudly on malformed unary expressions", () => {
    const ast = {
      ruleName: "program",
      children: [
        {
          ruleName: "top_decl",
          children: [
            {
              ruleName: "fn_decl",
              children: [
                { type: "fn", value: "fn" },
                { type: "NAME", value: "main" },
                { type: "LPAREN", value: "(" },
                { type: "RPAREN", value: ")" },
                {
                  ruleName: "block",
                  children: [
                    { type: "LBRACE", value: "{" },
                    {
                      ruleName: "stmt",
                      children: [
                        {
                          ruleName: "return_stmt",
                          children: [
                            { type: "return", value: "return" },
                            {
                              ruleName: "expr",
                              children: [
                                {
                                  ruleName: "unary_expr",
                                  children: [{ type: "BANG", value: "!" }],
                                },
                              ],
                            },
                            { type: "SEMICOLON", value: ";" },
                          ],
                        },
                      ],
                    },
                    { type: "RBRACE", value: "}" },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };

    expect(() => printNibDoc(ast as never)).toThrow("Malformed unary_expr: expected an AST child");
  });

  it("fails loudly when a unary operator is missing its operand", () => {
    const ast = {
      ruleName: "program",
      children: [
        {
          ruleName: "top_decl",
          children: [
            {
              ruleName: "fn_decl",
              children: [
                { type: "fn", value: "fn" },
                { type: "NAME", value: "main" },
                { type: "LPAREN", value: "(" },
                { type: "RPAREN", value: ")" },
                {
                  ruleName: "block",
                  children: [
                    { type: "LBRACE", value: "{" },
                    {
                      ruleName: "stmt",
                      children: [
                        {
                          ruleName: "return_stmt",
                          children: [
                            { type: "return", value: "return" },
                            {
                              ruleName: "expr",
                              children: [
                                {
                                  ruleName: "unary_expr",
                                  children: [
                                    { type: "BANG", value: "!" },
                                    { type: "SEMICOLON", value: ";" },
                                  ],
                                },
                              ],
                            },
                            { type: "SEMICOLON", value: ";" },
                          ],
                        },
                      ],
                    },
                    { type: "RBRACE", value: "}" },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };

    expect(() => printNibDoc(ast as never)).toThrow(
      "Malformed unary_expr: expected operator and operand",
    );
  });

  it("fails loudly on malformed primary expressions", () => {
    const ast = {
      ruleName: "program",
      children: [
        {
          ruleName: "top_decl",
          children: [
            {
              ruleName: "fn_decl",
              children: [
                { type: "fn", value: "fn" },
                { type: "NAME", value: "main" },
                { type: "LPAREN", value: "(" },
                { type: "RPAREN", value: ")" },
                {
                  ruleName: "block",
                  children: [
                    { type: "LBRACE", value: "{" },
                    {
                      ruleName: "stmt",
                      children: [
                        {
                          ruleName: "return_stmt",
                          children: [
                            { type: "return", value: "return" },
                            {
                              ruleName: "expr",
                              children: [
                                {
                                  ruleName: "primary",
                                  children: [{ type: "LPAREN", value: "(" }],
                                },
                              ],
                            },
                            { type: "SEMICOLON", value: ";" },
                          ],
                        },
                      ],
                    },
                    { type: "RBRACE", value: "}" },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };

    expect(() => printNibDoc(ast as never)).toThrow(
      "Malformed parenthesized primary: expected inner expression",
    );
  });

  it("fails loudly on empty primary nodes", () => {
    const ast = {
      ruleName: "program",
      children: [
        {
          ruleName: "top_decl",
          children: [
            {
              ruleName: "fn_decl",
              children: [
                { type: "fn", value: "fn" },
                { type: "NAME", value: "main" },
                { type: "LPAREN", value: "(" },
                { type: "RPAREN", value: ")" },
                {
                  ruleName: "block",
                  children: [
                    { type: "LBRACE", value: "{" },
                    {
                      ruleName: "stmt",
                      children: [
                        {
                          ruleName: "return_stmt",
                          children: [
                            { type: "return", value: "return" },
                            {
                              ruleName: "expr",
                              children: [{ ruleName: "primary", children: [] }],
                            },
                            { type: "SEMICOLON", value: ";" },
                          ],
                        },
                      ],
                    },
                    { type: "RBRACE", value: "}" },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };

    expect(() => printNibDoc(ast as never)).toThrow("Malformed primary: expected at least one child");
  });
});
