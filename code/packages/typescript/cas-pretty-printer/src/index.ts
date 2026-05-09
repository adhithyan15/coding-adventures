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
  SUB,
  headName,
} from "@coding-adventures/symbolic-ir";

export type DialectName = "macsyma" | "mathematica" | "maple";

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
    Re: "realpart",
    Im: "imagpart",
    Arg: "carg",
    IsPrime: "primep",
    FactorInteger: "ifactor",
    MoebiusMu: "moebius",
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

export function pretty(node: IRNode, dialect: Dialect = MacsymaDialect): string {
  return format(node, dialect, 0);
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
    if (b.kind === "apply" && headName(b.head) === NEG.name && b.args.length === 1) {
      return { kind: "apply", head: SUB, args: Object.freeze([a, b.args[0]]) };
    }
    if (a.kind === "integer" && a.value < 0n) {
      return { kind: "apply", head: SUB, args: Object.freeze([b, { kind: "integer", value: -a.value }]) };
    }
  }
  if (name === MUL.name && node.args.length === 2) {
    const [a, b] = node.args;
    if (b.kind === "apply" && headName(b.head) === INV.name && b.args.length === 1) {
      return { kind: "apply", head: DIV, args: Object.freeze([a, b.args[0]]) };
    }
    if (a.kind === "integer" && a.value === -1n) {
      return { kind: "apply", head: NEG, args: Object.freeze([b]) };
    }
  }
  return node;
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
