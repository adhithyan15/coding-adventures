/**
 * @coding-adventures/format-doc-std
 *
 * This package is the first reusable template layer on top of `format-doc`.
 * Language-specific printers should be able to build most ordinary constructs
 * by combining a small number of shared syntax-shape helpers instead of
 * hand-writing every `group()` and `line()` pattern from scratch.
 */

import {
  concat,
  group,
  ifBreak,
  indent,
  join,
  line,
  nil,
  softline,
  text,
  type Doc,
} from "@coding-adventures/format-doc";

/** Package version, mirrored in tests as a smoke check. */
export const VERSION = "0.1.0";

/** Whether a delimited list should end with a separator in broken form. */
export type TrailingSeparator = "never" | "always" | "ifBreak";

/** Configuration for `delimitedList()`. */
export interface DelimitedListOptions {
  open: Doc;
  close: Doc;
  items: readonly Doc[];
  separator?: Doc;
  trailingSeparator?: TrailingSeparator;
  emptySpacing?: boolean;
}

/** Configuration for `callLike()`. */
export interface CallLikeOptions {
  open?: Doc;
  close?: Doc;
  separator?: Doc;
  trailingSeparator?: TrailingSeparator;
}

/** Configuration for `blockLike()`. */
export interface BlockLikeOptions {
  open: Doc;
  body: Doc;
  close: Doc;
  emptySpacing?: boolean;
}

/** Configuration for `infixChain()`. */
export interface InfixChainOptions {
  operands: readonly Doc[];
  operators: readonly Doc[];
  breakBeforeOperators?: boolean;
}

/**
 * Format a list surrounded by delimiters such as `[]`, `()`, or `{}`.
 *
 * This is the foundational shape for arrays, tuples, parameter lists, object
 * fields, and many other recurring language constructs.
 */
export function delimitedList(options: DelimitedListOptions): Doc {
  const separator = options.separator ?? text(",");
  const trailing = options.trailingSeparator ?? "never";
  const emptySpacing = options.emptySpacing ?? false;

  if (options.items.length === 0) {
    return concat([
      options.open,
      emptySpacing ? text(" ") : nil(),
      options.close,
    ]);
  }

  const body = join(concat([separator, line()]), options.items);

  return group(
    concat([
      options.open,
      indent(
        concat([
          softline(),
          body,
          trailingDoc(separator, trailing),
        ])
      ),
      softline(),
      options.close,
    ])
  );
}

/**
 * Format a call-like form consisting of a callee followed by a delimited list
 * of arguments.
 */
export function callLike(
  callee: Doc,
  args: readonly Doc[],
  options: CallLikeOptions = {},
): Doc {
  return concat([
    callee,
    delimitedList({
      open: options.open ?? text("("),
      close: options.close ?? text(")"),
      items: args,
      separator: options.separator,
      trailingSeparator: options.trailingSeparator,
    }),
  ]);
}

/**
 * Format a block-like form with an opener, a body, and a closer.
 *
 * Short blocks stay inline if they fit. Longer blocks fall back to one item per
 * line with indentation.
 */
export function blockLike(options: BlockLikeOptions): Doc {
  const emptySpacing = options.emptySpacing ?? true;

  if (options.body.kind === "nil") {
    return concat([
      options.open,
      emptySpacing ? text(" ") : nil(),
      options.close,
    ]);
  }

  return group(
    concat([
      options.open,
      indent(concat([line(), options.body])),
      line(),
      options.close,
    ])
  );
}

/**
 * Format a chain of operands separated by infix operators.
 *
 * The caller decides whether broken form places operators at the end of the
 * previous line or the beginning of the next one.
 */
export function infixChain(options: InfixChainOptions): Doc {
  if (options.operands.length === 0) {
    return nil();
  }

  if (options.operators.length !== options.operands.length - 1) {
    throw new Error("infixChain() requires exactly one fewer operator than operands");
  }

  if (options.operands.length === 1) {
    return options.operands[0]!;
  }

  const breakBeforeOperators = options.breakBeforeOperators ?? false;
  const rest: Doc[] = [];

  for (let i = 0; i < options.operators.length; i += 1) {
    const operator = options.operators[i]!;
    const operand = options.operands[i + 1]!;

    if (breakBeforeOperators) {
      rest.push(line(), operator, text(" "), operand);
    } else {
      rest.push(text(" "), operator, line(), operand);
    }
  }

  return group(
    concat([
      options.operands[0]!,
      indent(concat(rest)),
    ])
  );
}

function trailingDoc(separator: Doc, trailing: TrailingSeparator): Doc {
  switch (trailing) {
    case "never":
      return nil();
    case "always":
      return separator;
    case "ifBreak":
      return ifBreak(separator);
    default: {
      const unreachable: never = trailing;
      throw new Error(`Unknown trailing separator mode: ${unreachable}`);
    }
  }
}
