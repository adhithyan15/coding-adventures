import type { SvgNode } from "../../src/ductusview";

export function collect(
  node: SvgNode,
  pick: (candidate: SvgNode) => boolean,
  out: SvgNode[] = [],
): SvgNode[] {
  if (pick(node)) out.push(node);
  for (const child of node.children ?? []) collect(child, pick, out);
  return out;
}

export const byTag = (node: SvgNode, tag: string): SvgNode[] =>
  collect(node, (candidate) => candidate.tag === tag);
