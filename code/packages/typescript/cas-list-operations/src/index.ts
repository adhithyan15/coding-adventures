import { IRApply, IRNode, LIST, app, headName, int, structuralKey, sym } from "@coding-adventures/symbolic-ir";

export class ListOperationError extends Error {}

export const LENGTH = sym("Length");
export const FIRST = sym("First");
export const REST = sym("Rest");
export const LAST = sym("Last");
export const APPEND = sym("Append");
export const REVERSE = sym("Reverse");
export const RANGE = sym("Range");
export const MAP = sym("Map");
export const APPLY = sym("Apply");
export const SELECT = sym("Select");
export const SORT = sym("Sort");
export const PART = sym("Part");
export const FLATTEN = sym("Flatten");
export const JOIN = sym("Join");

export function list(args: readonly IRNode[]): IRApply {
  return app(LIST, args);
}

export function length(value: IRNode): IRNode {
  return int(asList(value).length);
}

export function first(value: IRNode): IRNode {
  const args = asList(value);
  if (args.length === 0) throw new ListOperationError("first() of empty list");
  return args[0];
}

export function rest(value: IRNode): IRApply {
  const args = asList(value);
  if (args.length === 0) throw new ListOperationError("rest() of empty list");
  return list(args.slice(1));
}

export function last(value: IRNode): IRNode {
  const args = asList(value);
  if (args.length === 0) throw new ListOperationError("last() of empty list");
  return args[args.length - 1];
}

export function reverse(value: IRNode): IRApply {
  return list([...asList(value)].reverse());
}

export function append(...values: readonly IRNode[]): IRApply {
  return list(values.flatMap((value) => [...asList(value)]));
}

export const join = append;

export function part(value: IRNode, index: number): IRNode {
  const args = asList(value);
  if (!Number.isSafeInteger(index) || index === 0) {
    throw new ListOperationError("Part index must be a non-zero integer");
  }
  const resolved = index > 0 ? index - 1 : args.length + index;
  if (resolved < 0 || resolved >= args.length) {
    throw new ListOperationError(`Part index ${index} out of range`);
  }
  return args[resolved];
}

export function range(start: number, stop?: number, step = 1): IRApply {
  if (!Number.isSafeInteger(start) || (stop !== undefined && !Number.isSafeInteger(stop)) || !Number.isSafeInteger(step)) {
    throw new ListOperationError("Range arguments must be integers");
  }
  if (step === 0) throw new ListOperationError("Range step cannot be 0");
  const from = stop === undefined ? 1 : start;
  const to = stop === undefined ? start : stop;
  const out: IRNode[] = [];
  if (step > 0) {
    for (let value = from; value <= to; value += step) out.push(int(value));
  } else {
    for (let value = from; value >= to; value += step) out.push(int(value));
  }
  return list(out);
}

export function mapList(head: IRNode, value: IRNode): IRApply {
  return list(asList(value).map((arg) => app(head, [arg])));
}

export function applyList(head: IRNode, value: IRNode): IRApply {
  return app(head, asList(value));
}

export function select(value: IRNode, predicate: (node: IRNode) => boolean): IRApply {
  return list(asList(value).filter(predicate));
}

export function sortList(value: IRNode): IRApply {
  return list([...asList(value)].sort((a, b) => structuralKey(a).localeCompare(structuralKey(b))));
}

export function flatten(value: IRNode, depth = 1): IRApply {
  return list(flattenArgs(asList(value), depth < 0 ? Number.MAX_SAFE_INTEGER : depth));
}

export function asList(value: IRNode): readonly IRNode[] {
  if (value.kind === "apply" && headName(value.head) === LIST.name) {
    return value.args;
  }
  throw new ListOperationError(`expected List, got ${value.kind}`);
}

function flattenArgs(args: readonly IRNode[], depth: number): readonly IRNode[] {
  if (depth === 0) return args;
  const out: IRNode[] = [];
  for (const arg of args) {
    if (arg.kind === "apply" && headName(arg.head) === LIST.name) {
      out.push(...flattenArgs(arg.args, depth - 1));
    } else {
      out.push(arg);
    }
  }
  return out;
}
