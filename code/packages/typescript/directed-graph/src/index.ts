/**
 * Directed Graph Library
 * ======================
 *
 * A directed graph implementation with topological sort, cycle detection, transitive
 * closure, and parallel execution level computation. Built for use in build systems,
 * dependency resolution, and task scheduling.
 *
 * The library provides a single `Graph` class that stores nodes and directed edges
 * using a pair of adjacency maps (forward and reverse). All graph algorithms --
 * topological sort, cycle detection, transitive closure, independent grouping -- are
 * methods on the graph object itself, so you never need to import a separate module.
 *
 * Quick start:
 *
 *     import { Graph } from "@coding-adventures/directed-graph";
 *
 *     const g = new Graph();
 *     g.addEdge("A", "B");
 *     g.addEdge("B", "C");
 *
 *     console.log(g.topologicalSort());   // ['A', 'B', 'C']
 *     console.log(g.independentGroups()); // [['A'], ['B'], ['C']]
 *
 * Error classes are available at the top level too:
 *
 *     import { CycleError, NodeNotFoundError, EdgeNotFoundError } from "@coding-adventures/directed-graph";
 */

export {
  Graph,
  CycleError,
  NodeNotFoundError,
  EdgeNotFoundError,
  type GraphPropertyBag,
  type GraphPropertyValue,
  type WeightedEdge,
} from "./graph.js";
export { LabeledDirectedGraph } from "./labeled-graph.js";
export { toDot, toMermaid, toAsciiTable } from "./visualization.js";
export type { DotOptions, MermaidOptions } from "./visualization.js";
