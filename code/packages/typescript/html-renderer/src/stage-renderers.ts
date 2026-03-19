/**
 * Stage Renderers — Turning pipeline data into HTML sections.
 * ============================================================
 *
 * Each stage in the compilation pipeline has its own data format and
 * its own visual representation. This module contains one rendering
 * function per stage type.
 *
 * The rendering functions follow a common pattern:
 *
 * 1. Accept stage-specific data (tokens, AST, bytecode, etc.)
 * 2. Return an HTML string representing that data
 *
 * The main HTMLRenderer calls these functions based on the stage's
 * `name` field, wrapping the output in a `<section>` with a header.
 *
 * Design philosophy: Generate clean, semantic HTML. The styling is
 * handled entirely by CSS (see styles.ts). This separation means
 * you could swap the CSS for a light theme without touching the
 * rendering logic.
 *
 * ```
 * StageReport.data ──→ renderXxx() ──→ HTML string
 *                                          │
 *                                    wrapped in <section>
 *                                    by the main renderer
 * ```
 */

import { escapeHtml } from "./escape.js";
import type {
  LexerData,
  LexerToken,
  ParserData,
  ASTNode,
  CompilerData,
  VMData,
  AssemblerData,
  HardwareExecutionData,
  ALUData,
  GateData,
} from "./types.js";

// ===========================================================================
// Token Classification
// ===========================================================================

/**
 * Classify a token type into a CSS class for color coding.
 *
 * Token types vary across languages (Python's "NAME" vs Ruby's
 * "IDENTIFIER"), so we use pattern matching to group similar
 * types together:
 *
 * | Pattern        | CSS Class      | Color   |
 * |----------------|----------------|---------|
 * | NAME, IDENT    | token-name     | Blue    |
 * | NUMBER, INT    | token-number   | Green   |
 * | +, -, *, /     | token-operator | Red     |
 * | IF, WHILE, DEF | token-keyword  | Yellow  |
 * | STRING, STR    | token-string   | Purple  |
 * | everything else| token-default  | Gray    |
 */
function classifyToken(tokenType: string): string {
  const upper = tokenType.toUpperCase();

  // Names and identifiers
  if (
    upper.includes("NAME") ||
    upper.includes("IDENT") ||
    upper.includes("VARIABLE")
  ) {
    return "token-name";
  }

  // Numbers and numeric literals
  if (
    upper.includes("NUMBER") ||
    upper.includes("INT") ||
    upper.includes("FLOAT") ||
    upper.includes("DIGIT")
  ) {
    return "token-number";
  }

  // Operators and punctuation
  if (
    upper.includes("PLUS") ||
    upper.includes("MINUS") ||
    upper.includes("STAR") ||
    upper.includes("SLASH") ||
    upper.includes("EQUAL") ||
    upper.includes("PAREN") ||
    upper.includes("BRACE") ||
    upper.includes("BRACKET") ||
    upper.includes("COMMA") ||
    upper.includes("SEMICOL") ||
    upper.includes("COLON") ||
    upper.includes("DOT") ||
    upper.includes("OPERATOR") ||
    upper.includes("ASSIGN")
  ) {
    return "token-operator";
  }

  // Keywords
  if (
    upper.includes("KEYWORD") ||
    upper === "IF" ||
    upper === "ELSE" ||
    upper === "WHILE" ||
    upper === "FOR" ||
    upper === "DEF" ||
    upper === "RETURN" ||
    upper === "CLASS" ||
    upper === "IMPORT"
  ) {
    return "token-keyword";
  }

  // Strings
  if (upper.includes("STRING") || upper.includes("STR")) {
    return "token-string";
  }

  return "token-default";
}

// ===========================================================================
// Lexer Stage Renderer
// ===========================================================================

/**
 * Render the lexer stage as colored token badges.
 *
 * Each token becomes a small card showing its type (in a colored
 * label) and its value (in monospace font). The tokens flow left
 * to right, wrapping to the next line as needed.
 *
 * Visual result:
 * ```
 * ┌────────┐ ┌──────────┐ ┌────────┐ ┌──────┐ ┌────────┐ ┌─────┐
 * │ NAME   │ │ EQUALS   │ │ NUMBER │ │ PLUS │ │ NUMBER │ │ EOF │
 * │ "x"    │ │ "="      │ │ "1"    │ │ "+"  │ │ "2"    │ │ ""  │
 * └────────┘ └──────────┘ └────────┘ └──────┘ └────────┘ └─────┘
 * ```
 */
export function renderLexerStage(data: LexerData): string {
  const tokenBadges = data.tokens
    .map((token: LexerToken) => {
      const cssClass = classifyToken(token.type);
      const displayValue =
        token.value === "" ? '""' : escapeHtml(token.value);

      return `<div class="token ${cssClass}">
        <span class="token-type">${escapeHtml(token.type)}</span>
        <span class="token-value">${displayValue}</span>
      </div>`;
    })
    .join("\n      ");

  return `<div class="token-list">
      ${tokenBadges}
    </div>`;
}

// ===========================================================================
// Parser Stage Renderer (AST)
// ===========================================================================

/**
 * Render the parser stage as an SVG tree diagram.
 *
 * Drawing a tree is a classic computer science visualization problem.
 * Our approach:
 *
 * 1. **Measure** — Calculate the width each subtree needs so nodes
 *    don't overlap. Leaf nodes need a fixed width; internal nodes
 *    need the sum of their children's widths.
 *
 * 2. **Position** — Assign (x, y) coordinates to each node using
 *    a top-down traversal. Each level of the tree gets a fixed
 *    y offset. The x position is centered over the subtree.
 *
 * 3. **Draw** — Generate SVG elements: rounded rectangles for nodes,
 *    lines connecting parents to children.
 *
 * ```
 *        Assignment          <- Level 0, y = 40
 *        /        \
 *     Name       BinaryOp    <- Level 1, y = 120
 *      |         /      \
 *      x     Number   Number  <- Level 2, y = 200
 * ```
 */

/** Configuration for tree layout. */
const TREE_NODE_WIDTH = 120;
const TREE_NODE_HEIGHT = 40;
const TREE_LEVEL_HEIGHT = 80;
const TREE_NODE_PADDING = 20;

/**
 * Measure the width a subtree needs.
 *
 * This is a recursive algorithm: a leaf node needs TREE_NODE_WIDTH
 * pixels, and an internal node needs the sum of its children's
 * widths (plus spacing between siblings).
 *
 * Think of it like measuring how wide a family tree would be on
 * paper: a person with no descendants needs one column, but a
 * person with three children needs three columns.
 */
function measureSubtreeWidth(node: ASTNode): number {
  if (node.children.length === 0) {
    return TREE_NODE_WIDTH + TREE_NODE_PADDING;
  }
  return node.children.reduce(
    (sum, child) => sum + measureSubtreeWidth(child),
    0
  );
}

/** A positioned node ready for SVG rendering. */
interface PositionedNode {
  node: ASTNode;
  x: number;
  y: number;
  children: PositionedNode[];
}

/**
 * Assign (x, y) coordinates to every node in the tree.
 *
 * We traverse top-down, distributing each node's children across
 * the horizontal space allocated to that subtree. Each child gets
 * a slice of space proportional to its measured width.
 *
 * The x coordinate is the *center* of each node, and the y coordinate
 * increases by TREE_LEVEL_HEIGHT for each level of depth.
 */
function positionNodes(
  node: ASTNode,
  x: number,
  y: number
): PositionedNode {
  if (node.children.length === 0) {
    return { node, x, y, children: [] };
  }

  let currentX = x - measureSubtreeWidth(node) / 2;
  const positionedChildren: PositionedNode[] = [];

  for (const child of node.children) {
    const childWidth = measureSubtreeWidth(child);
    const childX = currentX + childWidth / 2;
    positionedChildren.push(
      positionNodes(child, childX, y + TREE_LEVEL_HEIGHT)
    );
    currentX += childWidth;
  }

  return { node, x, y, children: positionedChildren };
}

/**
 * Calculate the total depth of the tree (for setting SVG height).
 */
function treeDepth(node: ASTNode): number {
  if (node.children.length === 0) return 1;
  return 1 + Math.max(...node.children.map(treeDepth));
}

/**
 * Generate SVG elements for a positioned tree.
 *
 * This is the final drawing step. For each node, we generate:
 * - A rounded rectangle with the node type
 * - A text label (type + optional value)
 * - Lines connecting it to each of its children
 *
 * The lines are drawn *first* so they appear behind the node
 * rectangles (SVG renders elements in document order).
 */
function renderTreeSvg(positioned: PositionedNode): string {
  const lines: string[] = [];
  const nodes: string[] = [];

  function traverse(p: PositionedNode): void {
    // Draw lines to children first (behind nodes)
    for (const child of p.children) {
      lines.push(
        `<line x1="${p.x}" y1="${p.y + TREE_NODE_HEIGHT / 2}" ` +
          `x2="${child.x}" y2="${child.y - TREE_NODE_HEIGHT / 2}" ` +
          `stroke="#585b70" stroke-width="2"/>`
      );
    }

    // Draw the node rectangle and label
    const label = p.node.value
      ? `${p.node.type}(${p.node.value})`
      : p.node.type;
    const rectX = p.x - TREE_NODE_WIDTH / 2;
    const rectY = p.y - TREE_NODE_HEIGHT / 2;

    nodes.push(
      `<rect x="${rectX}" y="${rectY}" ` +
        `width="${TREE_NODE_WIDTH}" height="${TREE_NODE_HEIGHT}" ` +
        `rx="8" fill="#313244" stroke="#89b4fa" stroke-width="1.5"/>` +
        `<text x="${p.x}" y="${p.y + 5}" ` +
        `text-anchor="middle" fill="#cdd6f4" ` +
        `font-family="monospace" font-size="12">${escapeHtml(label)}</text>`
    );

    // Recurse into children
    for (const child of p.children) {
      traverse(child);
    }
  }

  traverse(positioned);

  // Lines first (behind), then nodes (in front)
  return lines.join("\n") + "\n" + nodes.join("\n");
}

/**
 * Main entry point for rendering the parser stage.
 *
 * Measures the tree, positions all nodes, then generates an SVG
 * element with the correct dimensions.
 */
export function renderParserStage(data: ParserData): string {
  const ast = data.ast;
  const width = Math.max(measureSubtreeWidth(ast), TREE_NODE_WIDTH * 2);
  const depth = treeDepth(ast);
  const height = depth * TREE_LEVEL_HEIGHT + TREE_NODE_HEIGHT;

  const positioned = positionNodes(ast, width / 2, TREE_NODE_HEIGHT);
  const svgContent = renderTreeSvg(positioned);

  return `<div class="ast-container">
      <svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">
        ${svgContent}
      </svg>
    </div>`;
}

// ===========================================================================
// Compiler Stage Renderer (Bytecode)
// ===========================================================================

/**
 * Render the compiler stage as a bytecode instruction table.
 *
 * Bytecode is a flat list of instructions that a virtual machine
 * can execute. Each instruction has:
 * - An index (its position in the instruction list)
 * - An opcode (what operation to perform)
 * - An optional argument
 * - A stack effect showing how the VM's stack changes
 *
 * We also display the constants pool and names table, which are
 * lookup tables the bytecode instructions reference.
 *
 * Visual result:
 * ```
 * | #  | Opcode     | Arg | Stack Effect |
 * |----|------------|-----|--------------|
 * | 0  | LOAD_CONST | 1   | -> 1         |
 * | 1  | LOAD_CONST | 2   | -> 2         |
 * | 2  | ADD        |     | 1, 2 -> 3    |
 * ```
 */
export function renderCompilerStage(data: CompilerData): string {
  const rows = data.instructions
    .map(
      (instr) =>
        `<tr>
        <td>${instr.index}</td>
        <td>${escapeHtml(instr.opcode)}</td>
        <td>${instr.arg !== null ? escapeHtml(String(instr.arg)) : ""}</td>
        <td>${escapeHtml(instr.stack_effect)}</td>
      </tr>`
    )
    .join("\n      ");

  let html = `<table>
      <thead>
        <tr><th>#</th><th>Opcode</th><th>Arg</th><th>Stack Effect</th></tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>`;

  // Show constants pool if non-empty
  if (data.constants.length > 0) {
    html += `\n    <h3>Constants Pool</h3>
    <p><code>[${data.constants.map((c) => escapeHtml(String(c))).join(", ")}]</code></p>`;
  }

  // Show names table if non-empty
  if (data.names.length > 0) {
    html += `\n    <h3>Names Table</h3>
    <p><code>[${data.names.map((n) => escapeHtml(String(n))).join(", ")}]</code></p>`;
  }

  return html;
}

// ===========================================================================
// VM Stage Renderer
// ===========================================================================

/**
 * Render the VM execution stage as a step-by-step trace table.
 *
 * This is like watching a debugger step through the code. Each row
 * shows:
 * - The instruction being executed
 * - The stack state *before* execution
 * - The stack state *after* execution
 * - The current variable bindings
 *
 * Stacks are rendered as visual columns of boxes (bottom to top,
 * like a physical stack of plates). Variables are shown as
 * name=value pairs.
 */

/**
 * Render a stack (array of values) as a visual column.
 *
 * The stack is displayed vertically with the bottom of the stack
 * at the bottom of the column. An empty stack shows as an empty
 * bordered box.
 *
 * ```
 *   ┌───┐        ┌───┐
 *   │ 2 │  top    │   │  empty
 *   │ 1 │  bot    └───┘
 *   └───┘
 * ```
 */
function renderStack(values: (number | string)[]): string {
  if (values.length === 0) {
    return `<div class="stack"><div class="stack-item" style="color: #585b70">-</div></div>`;
  }
  const items = values
    .map((v) => `<div class="stack-item">${escapeHtml(String(v))}</div>`)
    .join("");
  return `<div class="stack">${items}</div>`;
}

/**
 * Render variable bindings as colored name=value pairs.
 */
function renderVariables(vars: Record<string, number | string>): string {
  const entries = Object.entries(vars);
  if (entries.length === 0) {
    return `<span style="color: #585b70">-</span>`;
  }
  return `<div class="variables">${entries
    .map(
      ([name, value]) =>
        `<span class="var-pair"><span class="var-name">${escapeHtml(name)}</span>=<span class="var-value">${escapeHtml(String(value))}</span></span>`
    )
    .join("")}</div>`;
}

export function renderVMStage(data: VMData): string {
  const rows = data.steps
    .map(
      (step) =>
        `<tr>
        <td>${step.index}</td>
        <td>${escapeHtml(step.instruction)}</td>
        <td>${renderStack(step.stack_before)}</td>
        <td>${renderStack(step.stack_after)}</td>
        <td>${renderVariables(step.variables)}</td>
      </tr>`
    )
    .join("\n      ");

  return `<table>
      <thead>
        <tr><th>#</th><th>Instruction</th><th>Stack Before</th><th>Stack After</th><th>Variables</th></tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>`;
}

// ===========================================================================
// Assembler Stage Renderer
// ===========================================================================

/**
 * Render the assembler stage as an assembly listing with binary encoding.
 *
 * Each line shows the memory address, the human-readable assembly,
 * the hex representation of the machine code, and a color-coded
 * breakdown of the binary encoding fields.
 *
 * The encoding breakdown is the most interesting part — it shows
 * how a single instruction like `add x3, x1, x2` is packed into
 * 32 bits with specific fields for the opcode, registers, and
 * function codes.
 */

/**
 * Render the bit-field encoding of a single instruction.
 *
 * Each field (opcode, rs1, rs2, funct3, etc.) gets a different
 * color, with a label above and the binary value below:
 *
 * ```
 *   funct7    rs2     rs1    funct3    rd     opcode
 *  0000000   00010   00001    000    00011   0110011
 * ```
 */
function renderEncoding(encoding: Record<string, string>): string {
  const entries = Object.entries(encoding);
  if (entries.length === 0) return "";

  const fields = entries
    .map(
      ([label, value]) =>
        `<span class="bit-field">
          <span class="bit-field-label">${escapeHtml(label)}</span>
          <span class="bit-field-value">${escapeHtml(value)}</span>
        </span>`
    )
    .join("");

  return `<div class="encoding">${fields}</div>`;
}

export function renderAssemblerStage(data: AssemblerData): string {
  const rows = data.lines
    .map(
      (line) =>
        `<tr>
        <td style="color: #94e2d5">0x${line.address.toString(16).padStart(2, "0")}</td>
        <td>${escapeHtml(line.assembly)}</td>
        <td>${escapeHtml(line.binary)}</td>
        <td>${renderEncoding(line.encoding)}</td>
      </tr>`
    )
    .join("\n      ");

  return `<table>
      <thead>
        <tr><th>Address</th><th>Assembly</th><th>Binary</th><th>Encoding</th></tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>`;
}

// ===========================================================================
// Hardware Execution Stage Renderer (RISC-V / ARM)
// ===========================================================================

/**
 * Render hardware execution as a register-state trace.
 *
 * Unlike the VM (which has a stack), hardware processors use
 * *registers* — a small number of named storage locations. This
 * renderer shows which registers changed at each step, highlighted
 * in yellow to draw attention.
 *
 * The full register state is shown as a compact table. Registers
 * that changed in this step are highlighted.
 */
export function renderHardwareExecutionStage(
  data: HardwareExecutionData
): string {
  const rows = data.steps
    .map((step) => {
      // Build register display with highlights for changed ones
      const regEntries = Object.entries(step.registers);
      const regs = regEntries
        .map(([name, value]) => {
          const changed = name in step.registers_changed;
          const cls = changed ? "reg-changed" : "";
          return `<span class="${cls}">${escapeHtml(name)}=${value}</span>`;
        })
        .join(" ");

      return `<tr>
        <td style="color: #94e2d5">0x${step.address.toString(16).padStart(2, "0")}</td>
        <td>${escapeHtml(step.instruction)}</td>
        <td>${regs}</td>
      </tr>`;
    })
    .join("\n      ");

  return `<table>
      <thead>
        <tr><th>Address</th><th>Instruction</th><th>Registers</th></tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>`;
}

// ===========================================================================
// ALU Stage Renderer
// ===========================================================================

/**
 * Render ALU operations showing bit-level arithmetic.
 *
 * The ALU (Arithmetic Logic Unit) is the part of the CPU that
 * performs actual math. This renderer shows the binary representation
 * of each operand and result, plus the CPU flags (zero, carry,
 * negative, overflow) that indicate special conditions.
 *
 * Flags are important for conditional branching — when the CPU
 * needs to make a decision (like "is x > 0?"), it checks the
 * flags set by the most recent ALU operation.
 *
 * ```
 * ADD: 00000001 + 00000010 = 00000011
 * Flags: Z=0 C=0 N=0 V=0
 * ```
 */
export function renderALUStage(data: ALUData): string {
  const rows = data.operations
    .map((op) => {
      const flags = [
        `<span class="${op.flags.zero ? "flag-set" : "flag-clear"}">Z=${op.flags.zero ? "1" : "0"}</span>`,
        `<span class="${op.flags.carry ? "flag-set" : "flag-clear"}">C=${op.flags.carry ? "1" : "0"}</span>`,
        `<span class="${op.flags.negative ? "flag-set" : "flag-clear"}">N=${op.flags.negative ? "1" : "0"}</span>`,
        `<span class="${op.flags.overflow ? "flag-set" : "flag-clear"}">V=${op.flags.overflow ? "1" : "0"}</span>`,
      ].join(" ");

      return `<tr>
        <td>${escapeHtml(op.op)}</td>
        <td>${op.a}</td>
        <td>${op.b}</td>
        <td>${op.result}</td>
        <td><span class="alu-bits">${escapeHtml(op.bits_a)}</span></td>
        <td><span class="alu-bits">${escapeHtml(op.bits_b)}</span></td>
        <td><span class="alu-bits">${escapeHtml(op.bits_result)}</span></td>
        <td>${flags}</td>
      </tr>`;
    })
    .join("\n      ");

  return `<table>
      <thead>
        <tr><th>Op</th><th>A</th><th>B</th><th>Result</th><th>Bits A</th><th>Bits B</th><th>Bits Result</th><th>Flags</th></tr>
      </thead>
      <tbody>
        ${rows}
      </tbody>
    </table>`;
}

// ===========================================================================
// Gate Stage Renderer
// ===========================================================================

/**
 * Render gate-level operations as a circuit trace.
 *
 * This is the absolute lowest level of the computing stack. Every
 * computation ultimately reduces to AND, OR, XOR, and NOT gates
 * operating on individual bits.
 *
 * Operations are grouped (e.g., "Full adder bit 0") with individual
 * gate evaluations shown within each group:
 *
 * ```
 * Full adder bit 0:
 *   XOR  [1, 0] -> 1  (A0 XOR B0)
 *   AND  [1, 0] -> 0  (A0 AND B0)
 *   XOR  [1, 0] -> 1  (Sum0 = partial XOR carry_in)
 *   AND  [1, 0] -> 0  (partial AND carry_in)
 *   OR   [0, 0] -> 0  (Carry0)
 * ```
 */
export function renderGateStage(data: GateData): string {
  const groups = data.operations
    .map((operation) => {
      const gateRows = operation.gates
        .map(
          (gate) =>
            `<div class="gate-row">
            <span class="gate-name">${escapeHtml(gate.gate)}</span>
            <span class="gate-inputs">[${gate.inputs.join(", ")}]</span>
            <span class="gate-arrow">&rarr;</span>
            <span class="gate-output">${gate.output}</span>
            <span class="gate-label">${escapeHtml(gate.label)}</span>
          </div>`
        )
        .join("\n        ");

      return `<div class="gate-group">
        <div class="gate-group-title">${escapeHtml(operation.description)}</div>
        ${gateRows}
      </div>`;
    })
    .join("\n      ");

  return groups;
}

// ===========================================================================
// Fallback Renderer
// ===========================================================================

/**
 * Render an unknown stage type as a JSON dump.
 *
 * If a new stage type is added to the pipeline but the renderer
 * doesn't know about it yet, we fall back to displaying the raw
 * JSON data. This is better than crashing or silently omitting
 * the stage — the user can at least see what data is available.
 */
export function renderFallbackStage(data: Record<string, unknown>): string {
  return `<pre><code>${escapeHtml(JSON.stringify(data, null, 2))}</code></pre>`;
}
