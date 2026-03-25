/**
 * Verilog Preprocessor — handles compiler directives before tokenization.
 *
 * Verilog (IEEE 1364-2005) has a C-like preprocessor that runs before the
 * main compilation phase. The preprocessor handles:
 *
 *   `define    — Define a text macro (simple or parameterized)
 *   `undef     — Remove a previously defined macro
 *   `ifdef     — Conditional compilation: include block if macro is defined
 *   `ifndef    — Conditional compilation: include block if macro is NOT defined
 *   `else      — Alternative branch of conditional compilation
 *   `endif     — End conditional compilation block
 *   `include   — File inclusion (stubbed — we can't read files here)
 *   `timescale — Time unit specification (stripped — irrelevant for synthesis)
 *
 * Why Preprocess Before Tokenizing?
 * ---------------------------------
 *
 * Unlike C/C++ where the preprocessor operates on raw text, Verilog's
 * preprocessor directives are tightly integrated with the language. Consider:
 *
 *     `define WIDTH 8
 *     reg [`WIDTH-1:0] data;
 *
 * After preprocessing, this becomes:
 *
 *     reg [8-1:0] data;
 *
 * If we tokenized first, `WIDTH would be a DIRECTIVE token, and the lexer
 * wouldn't know it should be replaced with "8". By preprocessing first,
 * the tokenizer sees clean Verilog source with no directives.
 *
 * Parameterized Macros
 * --------------------
 *
 * Verilog macros can take parameters, similar to C macros:
 *
 *     `define MAX(a, b) ((a) > (b) ? (a) : (b))
 *     assign result = `MAX(x, y);
 *
 * After preprocessing:
 *
 *     assign result = ((x) > (y) ? (x) : (y));
 *
 * Parameters are positional and separated by commas. The expansion replaces
 * each parameter name in the macro body with the corresponding argument.
 *
 * Conditional Compilation
 * -----------------------
 *
 * Conditional directives let you include or exclude code based on whether
 * a macro is defined. This is commonly used for:
 *
 *   - Debug/release builds: `ifdef DEBUG ... `endif
 *   - Platform-specific code: `ifdef FPGA_XILINX ... `else ... `endif
 *   - Feature flags: `ifndef FEATURE_X ... `endif
 *
 * Conditionals can nest:
 *
 *     `ifdef A
 *       `ifdef B
 *         // A and B both defined
 *       `endif
 *     `endif
 */

/**
 * A macro definition. Simple macros have no parameters and just a body.
 * Parameterized macros have a list of parameter names.
 *
 * Example simple macro:
 *   `define WIDTH 8
 *   => { params: null, body: "8" }
 *
 * Example parameterized macro:
 *   `define MAX(a, b) ((a) > (b) ? (a) : (b))
 *   => { params: ["a", "b"], body: "((a) > (b) ? (a) : (b))" }
 */
interface MacroDefinition {
  params: string[] | null;
  body: string;
}

/**
 * Preprocess Verilog source code by expanding macros and evaluating
 * conditional compilation directives.
 *
 * The preprocessor operates line-by-line, which matches how real Verilog
 * preprocessors work. Each line is checked for directive prefixes (backtick
 * followed by a keyword), and non-directive lines are scanned for macro
 * invocations to expand.
 *
 * @param source - Raw Verilog source that may contain preprocessor directives.
 * @returns Preprocessed source with all directives resolved and macros expanded.
 *
 * @example
 *     // Simple macro expansion:
 *     verilogPreprocess("`define WIDTH 8\nreg [`WIDTH-1:0] data;")
 *     // => "\nreg [8-1:0] data;"
 *
 * @example
 *     // Conditional compilation:
 *     verilogPreprocess("`define DEBUG\n`ifdef DEBUG\n$display(\"debug\");\n`endif")
 *     // => "\n\n$display(\"debug\");\n"
 */
export function verilogPreprocess(source: string): string {
  /**
   * The macro table maps macro names to their definitions.
   * This is populated by `define directives and pruned by `undef.
   */
  const macros: Map<string, MacroDefinition> = new Map();

  /**
   * The condition stack tracks nested `ifdef/`ifndef/`else/`endif blocks.
   * Each entry is a boolean: true means "we are in an active branch" (include
   * the code), false means "we are in an inactive branch" (skip the code).
   *
   * When the stack is empty, all code is active. When any entry is false,
   * all code is skipped until the matching `else or `endif.
   *
   * Why a stack? Because conditionals can nest:
   *     `ifdef A         <- push true/false
   *       `ifdef B       <- push true/false
   *       `endif         <- pop
   *     `endif           <- pop
   */
  const conditionStack: boolean[] = [];

  /**
   * Track whether we've seen an `else for the current level.
   * This prevents `else `else sequences (which would be ambiguous).
   */
  const seenElse: boolean[] = [];

  const lines = source.split("\n");
  const outputLines: string[] = [];

  for (const line of lines) {
    /**
     * Trim leading whitespace for directive detection. Verilog allows
     * indented directives (unlike C where # must be in column 1).
     */
    const trimmed = line.trimStart();

    /**
     * Check if we are currently in an active code region.
     * If any level of the condition stack is false, we are inactive.
     * Directive lines (`ifdef, `else, `endif) are always processed
     * regardless of active state, because they control the stack.
     */
    const isActive = conditionStack.every((v) => v);

    /**
     * --- `define: Define a macro ---
     *
     * Format:  `define NAME body
     * Format:  `define NAME(param1, param2) body
     *
     * The regex captures:
     *   1. The macro name
     *   2. Optional parameter list (with parentheses)
     *   3. The macro body (everything after the name/params)
     */
    if (trimmed.startsWith("`define")) {
      if (isActive) {
        const defineMatch = trimmed.match(
          /^`define\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:\(([^)]*)\))?\s*(.*?)\s*$/,
        );
        if (defineMatch) {
          const name = defineMatch[1];
          const paramsStr = defineMatch[2];
          const body = defineMatch[3] ?? "";

          const params =
            paramsStr !== undefined
              ? paramsStr.split(",").map((p) => p.trim())
              : null;

          macros.set(name, { params, body });
        }
      }
      outputLines.push("");
      continue;
    }

    /**
     * --- `undef: Remove a macro definition ---
     *
     * Format: `undef NAME
     *
     * After `undef, the macro is no longer defined. Subsequent `ifdef
     * checks for this macro will evaluate to false.
     */
    if (trimmed.startsWith("`undef")) {
      if (isActive) {
        const undefMatch = trimmed.match(
          /^`undef\s+([a-zA-Z_][a-zA-Z0-9_]*)/,
        );
        if (undefMatch) {
          macros.delete(undefMatch[1]);
        }
      }
      outputLines.push("");
      continue;
    }

    /**
     * --- `ifdef: Conditional compilation (if defined) ---
     *
     * Format: `ifdef MACRO_NAME
     *
     * Pushes true onto the condition stack if the macro is defined,
     * false otherwise. Code between `ifdef and the matching `else/`endif
     * is included only when the condition is true.
     */
    if (trimmed.startsWith("`ifdef")) {
      const ifdefMatch = trimmed.match(
        /^`ifdef\s+([a-zA-Z_][a-zA-Z0-9_]*)/,
      );
      if (ifdefMatch) {
        const defined = macros.has(ifdefMatch[1]);
        conditionStack.push(isActive && defined);
        seenElse.push(false);
      }
      outputLines.push("");
      continue;
    }

    /**
     * --- `ifndef: Conditional compilation (if NOT defined) ---
     *
     * Format: `ifndef MACRO_NAME
     *
     * The inverse of `ifdef. Pushes true if the macro is NOT defined.
     * Commonly used for include guards:
     *     `ifndef MY_HEADER_INCLUDED
     *     `define MY_HEADER_INCLUDED
     *     ... declarations ...
     *     `endif
     */
    if (trimmed.startsWith("`ifndef")) {
      const ifndefMatch = trimmed.match(
        /^`ifndef\s+([a-zA-Z_][a-zA-Z0-9_]*)/,
      );
      if (ifndefMatch) {
        const defined = macros.has(ifndefMatch[1]);
        conditionStack.push(isActive && !defined);
        seenElse.push(false);
      }
      outputLines.push("");
      continue;
    }

    /**
     * --- `else: Alternative branch ---
     *
     * Flips the current condition. If we were including code, we now
     * skip it, and vice versa. Only valid inside an `ifdef/`ifndef block.
     *
     * The logic is subtle: we need to check whether the PARENT levels
     * are all active. If a parent is inactive, `else should not activate
     * this level (you can't escape an inactive parent by using `else).
     */
    if (trimmed.startsWith("`else")) {
      if (conditionStack.length > 0) {
        const currentIdx = conditionStack.length - 1;
        if (!seenElse[currentIdx]) {
          /**
           * Check if ALL parent levels are active. If any parent is
           * inactive, this `else branch stays inactive too.
           */
          const parentActive =
            conditionStack.length <= 1 ||
            conditionStack.slice(0, -1).every((v) => v);

          if (parentActive) {
            conditionStack[currentIdx] = !conditionStack[currentIdx];
          }
          seenElse[currentIdx] = true;
        }
      }
      outputLines.push("");
      continue;
    }

    /**
     * --- `endif: End conditional block ---
     *
     * Pops the condition stack. Must match a preceding `ifdef/`ifndef.
     */
    if (trimmed.startsWith("`endif")) {
      if (conditionStack.length > 0) {
        conditionStack.pop();
        seenElse.pop();
      }
      outputLines.push("");
      continue;
    }

    /**
     * --- `include: File inclusion (stubbed) ---
     *
     * Format: `include "filename.v"
     *
     * In a real preprocessor, this would read the file and insert its
     * contents. We can't do file I/O here, so we emit a comment noting
     * the inclusion was stubbed. This preserves line count for error
     * reporting while making it clear the include was not resolved.
     */
    if (trimmed.startsWith("`include")) {
      outputLines.push(
        isActive ? `/* stubbed: ${trimmed} */` : "",
      );
      continue;
    }

    /**
     * --- `timescale: Time unit specification (stripped) ---
     *
     * Format: `timescale 1ns/1ps
     *
     * The `timescale directive specifies the time unit and precision
     * for simulation. It has no effect on synthesis and is stripped
     * entirely from the output.
     */
    if (trimmed.startsWith("`timescale")) {
      outputLines.push("");
      continue;
    }

    /**
     * --- Non-directive lines: expand macros if active ---
     *
     * For active code regions, scan the line for macro invocations
     * (backtick followed by a defined macro name) and replace them
     * with the macro body.
     */
    if (!isActive) {
      outputLines.push("");
      continue;
    }

    outputLines.push(expandMacros(line, macros));
  }

  return outputLines.join("\n");
}

/**
 * Expand all macro invocations in a single line.
 *
 * A macro invocation is a backtick followed by a macro name:
 *     `WIDTH      — expands to the body of the WIDTH macro
 *     `MAX(a, b)  — expands to the body of MAX with a and b substituted
 *
 * For parameterized macros, the arguments are parsed by matching parentheses,
 * which allows nested parentheses in arguments:
 *     `MAX((a+b), c)  — argument 1 is "(a+b)", argument 2 is "c"
 *
 * The function iterates through the line character by character, building
 * the output string. When it encounters a backtick, it checks if the
 * following text is a defined macro name. If so, it expands it.
 *
 * Expansion is not recursive to avoid infinite loops from macros that
 * reference themselves. Each macro is expanded at most once per line.
 *
 * @param line - A single line of Verilog source code.
 * @param macros - The current macro definition table.
 * @returns The line with all macro invocations expanded.
 */
function expandMacros(
  line: string,
  macros: Map<string, MacroDefinition>,
): string {
  /**
   * Pattern to find macro invocations: backtick followed by an identifier.
   * We use a global regex and replace iteratively.
   */
  const macroPattern = /`([a-zA-Z_][a-zA-Z0-9_]*)/g;
  let result = "";
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = macroPattern.exec(line)) !== null) {
    const macroName = match[1];
    const macro = macros.get(macroName);

    if (!macro) {
      /**
       * Not a defined macro — leave the backtick-identifier as-is.
       * It might be a compiler directive like `timescale or `default_nettype
       * that wasn't caught by the directive handlers above.
       */
      result += line.slice(lastIndex, match.index + match[0].length);
      lastIndex = match.index + match[0].length;
      continue;
    }

    // Append text before this macro invocation
    result += line.slice(lastIndex, match.index);

    if (macro.params === null) {
      /**
       * Simple macro — replace `NAME with the body text.
       */
      result += macro.body;
      lastIndex = match.index + match[0].length;
    } else {
      /**
       * Parameterized macro — parse the argument list after the macro name.
       *
       * We need to find the opening '(' and then match arguments by
       * tracking parenthesis depth. This handles nested parens in args:
       *     `MAX((a+b), c)  =>  args = ["(a+b)", "c"]
       */
      const afterName = match.index + match[0].length;

      if (afterName < line.length && line[afterName] === "(") {
        const args = parseArguments(line, afterName);
        if (args !== null) {
          const expanded = expandParameterizedMacro(
            macro.body,
            macro.params,
            args.args,
          );
          result += expanded;
          lastIndex = args.endIndex;
          macroPattern.lastIndex = args.endIndex;
        } else {
          // Malformed argument list — leave as-is
          result += match[0];
          lastIndex = match.index + match[0].length;
        }
      } else {
        // Parameterized macro invoked without arguments — leave as-is
        result += match[0];
        lastIndex = match.index + match[0].length;
      }
    }
  }

  // Append any remaining text after the last macro
  result += line.slice(lastIndex);
  return result;
}

/**
 * Parse a parenthesized argument list from a line, starting at the '(' position.
 *
 * Arguments are separated by commas at the top level (depth 0). Commas inside
 * nested parentheses are part of the argument, not separators.
 *
 * @param line - The source line.
 * @param startIndex - Index of the opening '('.
 * @returns An object with the parsed args and the index after the closing ')',
 *          or null if the closing ')' is not found.
 */
function parseArguments(
  line: string,
  startIndex: number,
): { args: string[]; endIndex: number } | null {
  let depth = 0;
  let current = "";
  const args: string[] = [];

  for (let i = startIndex; i < line.length; i++) {
    const ch = line[i];

    if (ch === "(") {
      depth++;
      if (depth === 1) {
        // Skip the opening paren itself
        continue;
      }
    } else if (ch === ")") {
      depth--;
      if (depth === 0) {
        // End of argument list
        args.push(current.trim());
        return { args, endIndex: i + 1 };
      }
    } else if (ch === "," && depth === 1) {
      // Top-level comma separates arguments
      args.push(current.trim());
      current = "";
      continue;
    }

    if (depth >= 1) {
      current += ch;
    }
  }

  // If we get here, the closing ')' was never found
  return null;
}

/**
 * Expand a parameterized macro body by substituting parameter names
 * with the corresponding argument values.
 *
 * This uses word-boundary matching to avoid replacing substrings.
 * For example, if param is "a", it replaces standalone "a" but not
 * the "a" inside "array".
 *
 * @param body - The macro body template.
 * @param params - The parameter names from the macro definition.
 * @param args - The argument values from the macro invocation.
 * @returns The expanded body with all parameters replaced.
 *
 * @example
 *     expandParameterizedMacro("((a) > (b) ? (a) : (b))", ["a", "b"], ["x", "y"])
 *     // => "((x) > (y) ? (x) : (y))"
 */
function expandParameterizedMacro(
  body: string,
  params: string[],
  args: string[],
): string {
  let result = body;
  for (let i = 0; i < params.length; i++) {
    const paramName = params[i];
    const argValue = args[i] ?? "";
    /**
     * Use word-boundary regex to replace only whole-word occurrences
     * of the parameter name. This prevents "a" from matching inside
     * "array" or "data".
     */
    const paramRegex = new RegExp(`\\b${escapeRegex(paramName)}\\b`, "g");
    result = result.replace(paramRegex, argValue);
  }
  return result;
}

/**
 * Escape special regex characters in a string.
 *
 * Parameter names in Verilog are simple identifiers (letters, digits,
 * underscores), so this is defensive coding — none of these characters
 * should appear in a parameter name. But if they do, this prevents
 * regex injection.
 */
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
