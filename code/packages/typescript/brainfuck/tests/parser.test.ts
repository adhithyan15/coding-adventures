/**
 * Tests for the Brainfuck Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses Brainfuck
 * source text when loaded with the `brainfuck.grammar` file.
 *
 * The Brainfuck grammar's top-level rule is `program` -- any Brainfuck source
 * file is a single program consisting of zero or more instructions.
 *
 * Test Strategy
 * -------------
 *
 * Each test parses a Brainfuck string and then uses helper functions to walk the
 * resulting AST, looking for specific node types and tokens. This approach is
 * robust against changes in how the grammar wraps nodes.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Empty program** -- empty source produces a "program" root with no children
 *   2. **Simple commands** -- flat sequences of commands (no loops)
 *   3. **Simple loops** -- a single loop with a body
 *   4. **Nested loops** -- loops inside loops
 *   5. **Unmatched brackets** -- should throw a parse error
 *   6. **Canonical "++[>+<-]"** -- verify the exact AST structure
 *   7. **Comments are stripped** -- comments do not appear in the AST
 *   8. **All 6 non-bracket commands** -- each command type appears in the AST
 */

import { describe, it, expect } from "vitest";
import { parseBrainfuck } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This is the core helper for tree inspection. The grammar wraps commands
 * in multiple layers of rules (program -> instruction -> command), so we
 * need to search the whole tree to count nodes of a given type.
 *
 * @param node - The root node to search from.
 * @param ruleName - The grammar rule name to find (e.g., "loop", "command").
 * @returns All nodes in the tree with the given ruleName.
 */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

/**
 * Collect all leaf tokens from an AST subtree.
 *
 * Flattens the tree into a list of tokens. This makes it easy to check
 * which token types appear in a subtree without worrying about nesting.
 */
function findTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child));
    } else {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

describe("empty program", () => {
  it("parses an empty source string", () => {
    /**
     * An empty Brainfuck program is valid. The grammar rule
     *   program = { instruction } ;
     * allows zero instructions, so "" is a legal Brainfuck program.
     *
     * The returned AST root should have ruleName "program" and contain
     * no instruction nodes.
     */
    const ast = parseBrainfuck("");
    expect(ast.ruleName).toBe("program");
  });

  it("produces no instruction nodes for empty source", () => {
    /**
     * An empty program has no instructions, loops, or commands.
     * All three node types should be absent from the tree.
     */
    const ast = parseBrainfuck("");
    expect(findNodes(ast, "instruction")).toHaveLength(0);
    expect(findNodes(ast, "loop")).toHaveLength(0);
    expect(findNodes(ast, "command")).toHaveLength(0);
  });

  it("parses a comment-only source as an empty program", () => {
    /**
     * Source consisting entirely of comment text has no Brainfuck commands.
     * After lexing, the token stream is just EOF. The parser should
     * produce an empty program, identical to parsing "".
     */
    const ast = parseBrainfuck("this is just a comment no commands here");
    expect(ast.ruleName).toBe("program");
    expect(findNodes(ast, "command")).toHaveLength(0);
  });
});

describe("simple commands", () => {
  it("parses a single INC command", () => {
    /**
     * The source "+" contains one command. The parser should produce:
     *   program -> instruction -> command -> Token(INC, "+")
     *
     * We verify by finding one "command" node and one INC token.
     */
    const ast = parseBrainfuck("+");
    expect(ast.ruleName).toBe("program");

    const commandNodes = findNodes(ast, "command");
    expect(commandNodes).toHaveLength(1);

    const tokens = findTokens(ast);
    const incTokens = tokens.filter((t) => t.type === "INC");
    expect(incTokens).toHaveLength(1);
  });

  it("parses a sequence of two INC commands", () => {
    /**
     * "++" produces two instruction nodes, each containing a command node.
     * The "program" node has two children (after wrapping by the { } repetition).
     */
    const ast = parseBrainfuck("++");

    const commandNodes = findNodes(ast, "command");
    expect(commandNodes).toHaveLength(2);
  });

  it("parses all six non-bracket commands", () => {
    /**
     * The source "><+-.,  " contains one of each non-bracket command.
     * The parser should produce 6 command nodes, one per character.
     *
     * Note: LOOP_START ([) and LOOP_END (]) are not commands in the grammar --
     * they are handled by the `loop` rule. So there are exactly 6 command types.
     */
    const ast = parseBrainfuck("><+-.,");

    const commandNodes = findNodes(ast, "command");
    expect(commandNodes).toHaveLength(6);

    const tokens = findTokens(ast);
    const tokenTypes = tokens.map((t) => t.type).filter((t) => t !== "EOF");

    expect(tokenTypes).toContain("RIGHT");
    expect(tokenTypes).toContain("LEFT");
    expect(tokenTypes).toContain("INC");
    expect(tokenTypes).toContain("DEC");
    expect(tokenTypes).toContain("OUTPUT");
    expect(tokenTypes).toContain("INPUT");
  });
});

describe("simple loops", () => {
  it("parses an empty loop []", () => {
    /**
     * `[]` is a legal Brainfuck loop with an empty body. It is either:
     *   - an infinite loop (if the cell is nonzero -- rare, usually a bug)
     *   - a no-op (if the cell is zero -- the `[` skips to `]` immediately)
     *
     * The parser should produce one "loop" node with no instruction children.
     */
    const ast = parseBrainfuck("[]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(1);
  });

  it("parses the canonical clear-cell loop [-]", () => {
    /**
     * `[-]` is the idiomatic "clear cell" idiom in Brainfuck. It decrements
     * the current cell until it reaches zero. Starting from any value N,
     * it takes exactly N iterations.
     *
     * Structure:
     *   program -> instruction -> loop -> [LOOP_START, instruction, LOOP_END]
     *                                       instruction -> command -> DEC
     */
    const ast = parseBrainfuck("[-]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(1);

    const tokens = findTokens(ast);
    expect(tokens.some((t) => t.type === "LOOP_START")).toBe(true);
    expect(tokens.some((t) => t.type === "LOOP_END")).toBe(true);
    expect(tokens.some((t) => t.type === "DEC")).toBe(true);
  });

  it("parses a loop with multiple body commands", () => {
    /**
     * `[>+<-]` is the classic cell-copy loop. The body has 4 commands:
     * RIGHT, INC, LEFT, DEC. The loop node should contain 4 instruction
     * children, each wrapping one command.
     */
    const ast = parseBrainfuck("[>+<-]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(1);

    const commandNodes = findNodes(ast, "command");
    expect(commandNodes).toHaveLength(4); // >, +, <, -
  });
});

describe("nested loops", () => {
  it("parses two sequential loops", () => {
    /**
     * Two consecutive loops: `[-][+]`
     * - First loop: clears the cell
     * - Second loop: would run forever (cell is now 0, so it never executes)
     *
     * The parser should find 2 loop nodes at the top level, not nested.
     */
    const ast = parseBrainfuck("[-][+]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(2);
  });

  it("parses a nested loop [[]]", () => {
    /**
     * `[[]]` is a trivially nested loop. The inner loop body is empty.
     * The parser should find 2 loop nodes: the outer and inner.
     */
    const ast = parseBrainfuck("[[]]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(2);
  });

  it("parses deeply nested loops", () => {
    /**
     * Three levels of nesting: `[[[-]]]`
     * Outer loop -> middle loop -> inner clear-cell loop.
     * The parser should find 3 loop nodes.
     */
    const ast = parseBrainfuck("[[[-]]]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(3);
  });

  it("parses a realistic multiply loop", () => {
    /**
     * `[->+>+<<]` is a common Brainfuck pattern that copies a value.
     * It loops once per unit of the source cell's value, incrementing
     * two destination cells while decrementing the source.
     *
     * Body commands: -, >, +, >, +, <, < (7 commands in the loop body)
     */
    const ast = parseBrainfuck("[->+>+<<]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(1);

    const commandNodes = findNodes(ast, "command");
    expect(commandNodes).toHaveLength(7);
  });
});

describe("unmatched brackets", () => {
  it("throws on unmatched [ (missing ])", () => {
    /**
     * `[+` has an opening bracket with no matching closing bracket.
     * The `loop` grammar rule requires LOOP_START { instruction } LOOP_END,
     * so missing the LOOP_END causes a parse error.
     *
     * This is caught at parse time, before execution, which is one of the
     * benefits of the grammar-driven approach over direct translation.
     */
    expect(() => parseBrainfuck("[+")).toThrow();
  });

  it("throws on unmatched ] (missing [)", () => {
    /**
     * `+]` has a closing bracket with no matching opening bracket.
     * The grammar rules only produce `]` tokens inside loop bodies, so
     * a `]` in the top-level instruction sequence cannot be matched.
     */
    expect(() => parseBrainfuck("+]")).toThrow();
  });
});

describe("canonical ++[>+<-] pattern", () => {
  it("parses ++[>+<-] with program as root", () => {
    /**
     * The canonical copy loop: set cell 0 to 2, then copy to cell 1.
     * The root node should always be "program".
     */
    const ast = parseBrainfuck("++[>+<-]");
    expect(ast.ruleName).toBe("program");
  });

  it("parses ++[>+<-] into the correct structure: 2 commands + 1 loop", () => {
    /**
     * `++[>+<-]` has 3 top-level instructions:
     *   1. command (INC)
     *   2. command (INC)
     *   3. loop ([>+<-])
     *
     * The loop itself contains 4 sub-instructions (commands: >, +, <, -).
     * Total command nodes: 2 (top) + 4 (in loop) = 6.
     * Total loop nodes: 1.
     */
    const ast = parseBrainfuck("++[>+<-]");

    const loopNodes = findNodes(ast, "loop");
    expect(loopNodes).toHaveLength(1);

    const commandNodes = findNodes(ast, "command");
    expect(commandNodes).toHaveLength(6); // ++, then >, +, <, - inside loop
  });

  it("parses ++[>+<-] with comments producing the same AST", () => {
    /**
     * Adding comments should not change the AST structure. The lexer
     * discards all comment text before the parser sees the token stream.
     */
    const cleanAst = parseBrainfuck("++[>+<-]");
    const commentedAst = parseBrainfuck("++ setup  [  >+  <-  ] copy loop");

    const cleanCommands = findNodes(cleanAst, "command").length;
    const commentedCommands = findNodes(commentedAst, "command").length;

    expect(cleanCommands).toBe(commentedCommands);
    expect(findNodes(cleanAst, "loop").length).toBe(
      findNodes(commentedAst, "loop").length
    );
  });

  it("parses ++[>+<-] token types in the loop body", () => {
    /**
     * Inspect the tokens inside the loop body of `++[>+<-]`.
     * The loop body should contain exactly: LOOP_START, RIGHT, INC, LEFT, DEC, LOOP_END.
     */
    const ast = parseBrainfuck("++[>+<-]");

    const loopNodes = findNodes(ast, "loop");
    const loopTokens = findTokens(loopNodes[0]);
    const loopTokenTypes = loopTokens.map((t) => t.type);

    expect(loopTokenTypes).toContain("LOOP_START");
    expect(loopTokenTypes).toContain("RIGHT");
    expect(loopTokenTypes).toContain("INC");
    expect(loopTokenTypes).toContain("LEFT");
    expect(loopTokenTypes).toContain("DEC");
    expect(loopTokenTypes).toContain("LOOP_END");
  });
});
