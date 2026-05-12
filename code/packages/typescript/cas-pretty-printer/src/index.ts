import {
  ADD,
  DIV,
  EQUAL,
  INV,
  IRNode,
  LIST,
  MUL,
  NEG,
  NOT_EQUAL,
  POW,
  SQRT,
  SUB,
  headName,
} from "@coding-adventures/symbolic-ir";

export type DialectName = "macsyma" | "mathematica" | "maple";
export type PrettyStyle = "linear" | "2d";

export interface PrettyOptions {
  readonly style?: PrettyStyle;
}

export interface Dialect {
  readonly name: DialectName;
  readonly functionNames?: Readonly<Record<string, string>>;
  readonly binaryOps?: Readonly<Record<string, string>>;
  readonly unaryOps?: Readonly<Record<string, string>>;
  readonly listBrackets?: readonly [string, string];
  readonly callBrackets?: readonly [string, string];
  formatSymbol?(name: string): string;
}

export const MacsymaDialect: Dialect = {
  name: "macsyma",
  listBrackets: ["[", "]"],
  callBrackets: ["(", ")"],
  binaryOps: {
    [ADD.name]: "+",
    [SUB.name]: "-",
    [MUL.name]: "*",
    [DIV.name]: "/",
    [POW.name]: "^",
    [EQUAL.name]: "=",
    [NOT_EQUAL.name]: "#",
  },
  unaryOps: {
    [NEG.name]: "-",
    [INV.name]: "1/",
  },
  functionNames: {
    D: "diff",
    Integrate: "integrate",
    RatSimplify: "ratsimp",
    Apart: "partfrac",
    TrigSimplify: "trigsimp",
    TrigExpand: "trigexpand",
    TrigReduce: "trigreduce",
    Select: "sublist",
    MakeList: "makelist",
    Inverse: "invert",
    Re: "realpart",
    Im: "imagpart",
    Arg: "carg",
    RectForm: "rectform",
    PolarForm: "polarform",
    IsPrime: "primep",
    NextPrime: "next_prime",
    PrevPrime: "prev_prime",
    FactorInteger: "ifactor",
    Divisors: "divisors",
    Totient: "totient",
    MoebiusMu: "moebius",
    JacobiSymbol: "jacobi",
    ChineseRemainder: "chinese",
    IntegerLength: "numdigits",
  },
  formatSymbol: (name) => name === "ImaginaryUnit" ? "%i" : name,
};

export const MathematicaDialect: Dialect = {
  name: "mathematica",
  listBrackets: ["{", "}"],
  callBrackets: ["[", "]"],
  binaryOps: {
    [ADD.name]: "+",
    [SUB.name]: "-",
    [MUL.name]: "*",
    [DIV.name]: "/",
    [POW.name]: "^",
    [EQUAL.name]: "==",
    [NOT_EQUAL.name]: "!=",
  },
  unaryOps: {
    [NEG.name]: "-",
    [INV.name]: "1/",
  },
};

export const MapleDialect: Dialect = {
  name: "maple",
  listBrackets: ["[", "]"],
  callBrackets: ["(", ")"],
  binaryOps: {
    [ADD.name]: "+",
    [SUB.name]: "-",
    [MUL.name]: "*",
    [DIV.name]: "/",
    [POW.name]: "^",
    [EQUAL.name]: "=",
    [NOT_EQUAL.name]: "<>",
  },
  unaryOps: {
    [NEG.name]: "-",
    [INV.name]: "1/",
  },
};

export function formatLisp(node: IRNode): string {
  switch (node.kind) {
    case "symbol":
      return node.name;
    case "integer":
      return node.value.toString();
    case "rational":
      return `${node.numer}/${node.denom}`;
    case "float":
      return String(node.value);
    case "string":
      return JSON.stringify(node.value);
    case "apply": {
      const head = formatLisp(node.head);
      return node.args.length === 0
        ? `(${head})`
        : `(${head} ${node.args.map(formatLisp).join(" ")})`;
    }
  }
}

export class Box {
  readonly lines: readonly string[];
  readonly baseline: number;

  constructor(lines: readonly string[], baseline: number) {
    this.lines = Object.freeze([...lines]);
    this.baseline = baseline;
  }

  get width(): number {
    return this.lines.reduce((max, line) => Math.max(max, line.length), 0);
  }

  get height(): number {
    return this.lines.length;
  }

  render(): string {
    return this.lines.join("\n");
  }

  padWidth(target: number, align: "center" | "left" | "right" = "center"): Box {
    if (target <= this.width) return this;

    return new Box(this.lines.map((line) => {
      const pad = target - line.length;
      if (align === "left") return line + " ".repeat(pad);
      if (align === "right") return " ".repeat(pad) + line;

      const leftPad = Math.floor(pad / 2);
      return " ".repeat(leftPad) + line + " ".repeat(pad - leftPad);
    }), this.baseline);
  }
}

export function atomBox(text: string): Box {
  return new Box([text], 0);
}

export function hbox(boxes: readonly Box[], sep = ""): Box {
  if (boxes.length === 0) return atomBox("");

  const commonBaseline = Math.max(...boxes.map((box) => box.baseline));
  const maxBelow = Math.max(...boxes.map((box) => box.height - box.baseline - 1));
  const totalHeight = commonBaseline + 1 + maxBelow;

  const padded = boxes.map((box) => {
    const aboveRows = commonBaseline - box.baseline;
    const belowRows = totalHeight - box.height - aboveRows;
    const empty = " ".repeat(box.width);
    return [
      ...Array.from({ length: aboveRows }, () => empty),
      ...box.lines,
      ...Array.from({ length: belowRows }, () => empty),
    ];
  });

  const lines = Array.from({ length: totalHeight }, (_, row) => padded.map((rows) => rows[row]).join(sep));
  return new Box(lines, commonBaseline);
}

export function vbox(boxes: readonly Box[]): Box {
  if (boxes.length === 0) return atomBox("");

  const width = Math.max(...boxes.map((box) => box.width));
  const lines = boxes.flatMap((box) => box.padWidth(width).lines);
  return new Box(lines, Math.floor(lines.length / 2));
}

export function pretty(node: IRNode, dialect: Dialect = MacsymaDialect, options: PrettyOptions | PrettyStyle = {}): string {
  const style = typeof options === "string" ? options : (options.style ?? "linear");
  if (style === "2d") return pretty2D(node, dialect);
  if (style !== "linear") {
    throw new Error(`unsupported style: ${style}`);
  }
  return format(node, dialect, 0);
}

export function pretty2D(node: IRNode, dialect: Dialect = MacsymaDialect): string {
  return box(node, dialect).render();
}

function format(node: IRNode, dialect: Dialect, parentPrecedence: number): string {
  switch (node.kind) {
    case "symbol":
      return dialect.formatSymbol?.(node.name) ?? node.name;
    case "integer":
      return node.value.toString();
    case "rational":
      return `${node.numer}/${node.denom}`;
    case "float":
      return String(node.value);
    case "string":
      return JSON.stringify(node.value);
    case "apply":
      return formatApply(node, dialect, parentPrecedence);
  }
}

function formatApply(node: Extract<IRNode, { kind: "apply" }>, dialect: Dialect, parentPrecedence: number): string {
  const sugar = sugarApply(node);
  if (sugar !== node) return format(sugar, dialect, parentPrecedence);

  const name = headName(node.head);
  if (name === LIST.name) {
    const [open, close] = dialect.listBrackets ?? ["[", "]"];
    return `${open}${node.args.map((arg) => format(arg, dialect, 0)).join(", ")}${close}`;
  }

  const binary = dialect.binaryOps?.[name];
  if (binary !== undefined && node.args.length === 2) {
    const precedence = opPrecedence(name);
    const left = format(node.args[0], dialect, precedence);
    const right = format(node.args[1], dialect, precedence + (name === POW.name ? -1 : 1));
    return parenthesize(`${left} ${binary} ${right}`, precedence, parentPrecedence);
  }

  const unary = dialect.unaryOps?.[name];
  if (unary !== undefined && node.args.length === 1) {
    const precedence = 40;
    const rendered = `${unary}${format(node.args[0], dialect, precedence)}`;
    return parenthesize(rendered, precedence, parentPrecedence);
  }

  const [open, close] = dialect.callBrackets ?? ["(", ")"];
  const renderedHead = node.head.kind === "symbol"
    ? functionName(node.head.name, dialect)
    : format(node.head, dialect, 0);
  return `${renderedHead}${open}${node.args.map((arg) => format(arg, dialect, 0)).join(", ")}${close}`;
}

function sugarApply(node: Extract<IRNode, { kind: "apply" }>): IRNode {
  const name = headName(node.head);
  if (name === ADD.name && node.args.length === 2) {
    const [a, b] = node.args;
    const sugaredB = b.kind === "apply" ? sugarApply(b) : b;
    if (sugaredB.kind === "apply" && headName(sugaredB.head) === NEG.name && sugaredB.args.length === 1) {
      return { kind: "apply", head: SUB, args: Object.freeze([a, sugaredB.args[0]]) };
    }
    if (a.kind === "integer" && a.value < 0n) {
      return { kind: "apply", head: SUB, args: Object.freeze([b, { kind: "integer", value: -a.value }]) };
    }
  }
  if (name === MUL.name && node.args.length >= 2) {
    const [a, b] = node.args;
    if (a.kind === "integer" && a.value === -1n) {
      const rest = node.args.slice(1);
      const inner = rest.length === 1
        ? rest[0]
        : { kind: "apply" as const, head: MUL, args: Object.freeze(rest) };
      return { kind: "apply", head: NEG, args: Object.freeze([inner]) };
    }
  }
  if (name === MUL.name && node.args.length === 2) {
    const [a, b] = node.args;
    if (b.kind === "apply" && headName(b.head) === INV.name && b.args.length === 1) {
      return { kind: "apply", head: DIV, args: Object.freeze([a, b.args[0]]) };
    }
    if (b.kind === "apply" && headName(b.head) === NEG.name && b.args.length === 1) {
      return {
        kind: "apply",
        head: NEG,
        args: Object.freeze([{ kind: "apply", head: MUL, args: Object.freeze([a, b.args[0]]) }]),
      };
    }
  }
  return node;
}

function box(node: IRNode, dialect: Dialect): Box {
  switch (node.kind) {
    case "symbol":
      return atomBox(dialect.formatSymbol?.(node.name) ?? node.name);
    case "integer":
      return atomBox(node.value.toString());
    case "rational":
      return atomBox(`${node.numer}/${node.denom}`);
    case "float":
      return atomBox(String(node.value));
    case "string":
      return atomBox(JSON.stringify(node.value));
    case "apply":
      return boxApply(node, dialect);
  }
}

function boxApply(node: Extract<IRNode, { kind: "apply" }>, dialect: Dialect): Box {
  const sugar = sugarApply(node);
  if (sugar !== node) return box(sugar, dialect);

  const name = headName(node.head);
  if (name === NEG.name && node.args.length === 1) {
    const inner = box(node.args[0], dialect);
    return new Box(inner.lines.map((line, i) => i === inner.baseline ? `-${line}` : ` ${line}`), inner.baseline);
  }

  if (name === DIV.name && node.args.length === 2) {
    return divBox(box(node.args[0], dialect), box(node.args[1], dialect));
  }

  if (name === POW.name && node.args.length === 2) {
    return powBox(box(node.args[0], dialect), box(node.args[1], dialect));
  }

  if (name === SQRT.name && node.args.length === 1) {
    return sqrtBox(box(node.args[0], dialect));
  }

  if (name === ADD.name && node.args.length >= 2) {
    return hbox(intersperseBoxes(node.args.map((arg) => box(arg, dialect)), atomBox(" + ")));
  }

  if (name === SUB.name && node.args.length === 2) {
    return hbox([box(node.args[0], dialect), atomBox(" - "), box(node.args[1], dialect)]);
  }

  if (name === MUL.name && node.args.length >= 2) {
    return hbox(intersperseBoxes(node.args.map((arg) => box(arg, dialect)), atomBox("*")));
  }

  if (name === LIST.name) {
    const [open, close] = dialect.listBrackets ?? ["[", "]"];
    if (node.args.length === 0) return atomBox(`${open}${close}`);
    return hbox([
      atomBox(open),
      hbox(intersperseBoxes(node.args.map((arg) => box(arg, dialect)), atomBox(", "))),
      atomBox(close),
    ]);
  }

  return atomBox(format(node, dialect, 0));
}

function divBox(numBox: Box, denBox: Box): Box {
  const barWidth = Math.max(numBox.width, denBox.width) + 2;
  const num = numBox.padWidth(barWidth);
  const den = denBox.padWidth(barWidth);
  return new Box([...num.lines, "─".repeat(barWidth), ...den.lines], num.height);
}

function powBox(baseBox: Box, expBox: Box): Box {
  const baseBlank = " ".repeat(baseBox.width);
  const expBlank = " ".repeat(expBox.width);
  const expRows = expBox.lines.map((line) => baseBlank + line.padEnd(expBox.width, " "));
  const baseRows = baseBox.lines.map((line) => line.padEnd(baseBox.width, " ") + expBlank);
  return new Box([...expRows, ...baseRows], expBox.height + baseBox.baseline);
}

function sqrtBox(argBox: Box): Box {
  const argWidth = argBox.width;
  const innerWidth = argWidth + 2;
  const lines = [
    `  ┌${"─".repeat(innerWidth)}┐`,
    ...argBox.lines.map((line, i) => {
      const content = ` ${line.padEnd(argWidth, " ")} `;
      return i === argBox.baseline ? `√ │${content}│` : `  │${content}│`;
    }),
  ];
  return new Box(lines, argBox.baseline + 1);
}

function intersperseBoxes(boxes: readonly Box[], separator: Box): Box[] {
  const parts: Box[] = [];
  for (const [index, box] of boxes.entries()) {
    if (index > 0) parts.push(separator);
    parts.push(box);
  }
  return parts;
}

function functionName(name: string, dialect: Dialect): string {
  return dialect.functionNames?.[name] ?? (dialect.name === "mathematica" ? name : lowerFirst(name));
}

function lowerFirst(value: string): string {
  return value.length === 0 ? value : value[0].toLowerCase() + value.slice(1);
}

function opPrecedence(name: string): number {
  if (name === EQUAL.name || name === NOT_EQUAL.name) return 5;
  if (name === ADD.name || name === SUB.name) return 10;
  if (name === MUL.name || name === DIV.name) return 20;
  if (name === POW.name) return 30;
  return 50;
}

function parenthesize(value: string, precedence: number, parentPrecedence: number): string {
  return precedence < parentPrecedence ? `(${value})` : value;
}
