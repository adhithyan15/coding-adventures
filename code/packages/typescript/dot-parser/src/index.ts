export const VERSION = "0.1.0";

import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { parseParserGrammar, parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";
import { GrammarParser, collectTokens, isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import {
  graphDiagram,
  graphEdge,
  graphNode,
  type DiagramDirection,
  type DiagramShape,
  type GraphDiagram,
  type GraphEdge,
  type GraphNode,
} from "@coding-adventures/diagram-ir";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const DOT_TOKENS_PATH = join(GRAMMARS_DIR, "dot.tokens");
const DOT_GRAMMAR_PATH = join(GRAMMARS_DIR, "dot.grammar");

export interface DotAttribute {
  key: string;
  value?: string;
}

export interface DotNodeStatement {
  kind: "node_stmt";
  id: string;
  attributes: DotAttribute[];
}

export interface DotEdgeStatement {
  kind: "edge_stmt";
  chain: string[];
  attributes: DotAttribute[];
}

export interface DotAttrStatement {
  kind: "attr_stmt";
  target: "graph" | "node" | "edge";
  attributes: DotAttribute[];
}

export interface DotAssignmentStatement {
  kind: "assignment";
  key: string;
  value: string;
}

export type DotStatement =
  | DotNodeStatement
  | DotEdgeStatement
  | DotAttrStatement
  | DotAssignmentStatement;

export interface DotDocument {
  kind: "digraph";
  strict: boolean;
  id?: string;
  statements: DotStatement[];
}

function loadDotTokenGrammar() {
  return parseTokenGrammar(readFileSync(DOT_TOKENS_PATH, "utf-8"));
}

function loadDotParserGrammar() {
  return parseParserGrammar(readFileSync(DOT_GRAMMAR_PATH, "utf-8"));
}

function ruleChildren(node: ASTNode, ruleName?: string): ASTNode[] {
  return node.children.filter(
    (child): child is ASTNode => isASTNode(child) && (ruleName === undefined || child.ruleName === ruleName),
  );
}

function firstRuleChild(node: ASTNode, ruleName: string): ASTNode {
  const child = ruleChildren(node, ruleName)[0];
  if (!child) {
    throw new Error(`Expected DOT rule '${ruleName}' inside '${node.ruleName}'`);
  }
  return child;
}

function tokenValue(node: ASTNode, expectedCount = 1): string {
  const tokens = collectTokens(node).filter((token) => token.type !== "EOF");
  if (tokens.length !== expectedCount) {
    throw new Error(
      `Expected ${expectedCount} token(s) inside '${node.ruleName}', found ${tokens.length}`,
    );
  }
  return tokens[0].value;
}

function parseId(node: ASTNode): string {
  return tokenValue(node);
}

function parseAttribute(node: ASTNode): DotAttribute {
  const ids = ruleChildren(node, "id").map(parseId);
  if (ids.length === 0 || ids.length > 2) {
    throw new Error(`Expected one or two IDs inside DOT attribute, found ${ids.length}`);
  }
  return ids.length === 1
    ? { key: ids[0].toLowerCase() }
    : { key: ids[0].toLowerCase(), value: ids[1] };
}

function parseAttributes(node: ASTNode | undefined): DotAttribute[] {
  if (!node) {
    return [];
  }
  return ruleChildren(node, "bracket_attr_list").flatMap((listNode) => {
    const aList = ruleChildren(listNode, "a_list")[0];
    if (!aList) {
      return [];
    }
    return ruleChildren(aList, "a_pair").map(parseAttribute);
  });
}

function parseStatement(node: ASTNode): DotStatement {
  const inner = ruleChildren(node)[0];
  if (!inner) {
    throw new Error("Expected DOT statement body");
  }

  switch (inner.ruleName) {
    case "attr_stmt": {
      const targetNode = firstRuleChild(inner, "attr_target");
      const attributesNode = firstRuleChild(inner, "attr_list");
      const target = tokenValue(targetNode).toLowerCase() as DotAttrStatement["target"];
      return {
        kind: "attr_stmt",
        target,
        attributes: parseAttributes(attributesNode),
      };
    }

    case "edge_stmt": {
      const firstNode = firstRuleChild(inner, "node_id");
      const edgeRhs = firstRuleChild(inner, "edge_rhs");
      const chain = [
        parseId(firstRuleChild(firstNode, "id")),
        ...ruleChildren(edgeRhs, "node_id").map((nodeId) => parseId(firstRuleChild(nodeId, "id"))),
      ];
      const attributesNode = ruleChildren(inner, "attr_list")[0];
      return {
        kind: "edge_stmt",
        chain,
        attributes: parseAttributes(attributesNode),
      };
    }

    case "assignment": {
      const ids = ruleChildren(inner, "id").map(parseId);
      if (ids.length !== 2) {
        throw new Error(`Expected two IDs inside DOT assignment, found ${ids.length}`);
      }
      return {
        kind: "assignment",
        key: ids[0].toLowerCase(),
        value: ids[1],
      };
    }

    case "node_stmt": {
      const nodeId = firstRuleChild(inner, "node_id");
      const attributesNode = ruleChildren(inner, "attr_list")[0];
      return {
        kind: "node_stmt",
        id: parseId(firstRuleChild(nodeId, "id")),
        attributes: parseAttributes(attributesNode),
      };
    }

    default:
      throw new Error(`Unsupported DOT statement rule '${inner.ruleName}'`);
  }
}

function parseDotSyntaxTree(source: string): ASTNode {
  const tokens = tokenizeDot(source);
  const grammar = loadDotParserGrammar();
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}

function attrValue(
  attributes: DotAttribute[],
  key: string,
): string | undefined {
  return attributes.find((attribute) => attribute.key === key && attribute.value !== undefined)?.value;
}

function shapeFromAttributes(attributes: DotAttribute[]): DiagramShape {
  const shape = attrValue(attributes, "shape");
  const style = attrValue(attributes, "style");

  if (shape === "ellipse" || shape === "circle") {
    return "ellipse";
  }
  if (shape === "diamond") {
    return "diamond";
  }
  if (style?.split(",").map((part) => part.trim()).includes("rounded")) {
    return "rounded_rect";
  }
  return "rect";
}

function directionFromRankdir(rankdir: string | undefined): DiagramDirection {
  switch ((rankdir ?? "TB").toUpperCase()) {
    case "LR":
      return "lr";
    case "RL":
      return "rl";
    case "BT":
      return "bt";
    case "TB":
    default:
      return "tb";
  }
}

function lowerNode(
  id: string,
  explicitAttributes: DotAttribute[],
  inheritedAttributes: DotAttribute[],
): GraphNode {
  const attributes = [...inheritedAttributes, ...explicitAttributes];
  const label = attrValue(attributes, "label") ?? id;
  const color = attrValue(attributes, "color");
  const fillcolor = attrValue(attributes, "fillcolor");
  const fontcolor = attrValue(attributes, "fontcolor");

  return graphNode(id, {
    label: { text: label },
    shape: shapeFromAttributes(attributes),
    style: {
      fill: fillcolor,
      stroke: color,
      textColor: fontcolor,
    },
  });
}

function lowerEdge(
  from: string,
  to: string,
  explicitAttributes: DotAttribute[],
  inheritedAttributes: DotAttribute[],
): GraphEdge {
  const attributes = [...inheritedAttributes, ...explicitAttributes];
  const label = attrValue(attributes, "label");
  const color = attrValue(attributes, "color");
  const fontcolor = attrValue(attributes, "fontcolor");

  return graphEdge(from, to, {
    label: label ? { text: label } : undefined,
    style: {
      stroke: color,
      textColor: fontcolor,
    },
  });
}

export function tokenizeDot(source: string): Token[] {
  return grammarTokenize(source, loadDotTokenGrammar());
}

export function parseDot(source: string): DotDocument {
  const graph = parseDotSyntaxTree(source);
  const children = [...graph.children];
  let index = 0;
  let strict = false;

  const firstToken = children[0];
  if (!isASTNode(firstToken) && firstToken.value === "STRICT") {
    strict = true;
    index += 1;
  }

  const kindToken = children[index];
  if (isASTNode(kindToken) || kindToken.value !== "DIGRAPH") {
    throw new Error("Expected DIGRAPH keyword at DOT document start");
  }
  index += 1;

  let id: string | undefined;
  const maybeId = children[index];
  if (isASTNode(maybeId) && maybeId.ruleName === "id") {
    id = parseId(maybeId);
    index += 1;
  }

  const stmtList = children[index + 1];
  if (!isASTNode(stmtList) || stmtList.ruleName !== "stmt_list") {
    throw new Error("Expected stmt_list in DOT document");
  }

  return {
    kind: "digraph",
    strict,
    id,
    statements: ruleChildren(stmtList, "stmt").map(parseStatement),
  };
}

export function dotAstToGraphDiagram(document: DotDocument): GraphDiagram {
  const graphAssignments = new Map<string, string>();
  const graphAttributes: DotAttribute[] = [];
  const nodeDefaults: DotAttribute[] = [];
  const edgeDefaults: DotAttribute[] = [];
  const nodes = new Map<string, GraphNode>();
  const edges: GraphEdge[] = [];

  const ensureNode = (id: string, attributes: DotAttribute[] = []) => {
    if (nodes.has(id) && attributes.length === 0) return;
    const lowered = lowerNode(id, attributes, nodeDefaults);
    const existing = nodes.get(id);
    nodes.set(
      id,
      existing
        ? {
            ...existing,
            label: lowered.label,
            shape: lowered.shape,
            style: {
              ...existing.style,
              ...lowered.style,
            },
          }
        : lowered,
    );
  };

  for (const statement of document.statements) {
    switch (statement.kind) {
      case "assignment":
        graphAssignments.set(statement.key, statement.value);
        break;
      case "attr_stmt":
        if (statement.target === "graph") {
          graphAttributes.push(...statement.attributes);
        } else if (statement.target === "node") {
          nodeDefaults.push(...statement.attributes);
        } else {
          edgeDefaults.push(...statement.attributes);
        }
        break;
      case "node_stmt":
        ensureNode(statement.id, statement.attributes);
        break;
      case "edge_stmt":
        statement.chain.forEach((id) => ensureNode(id));
        for (let i = 0; i < statement.chain.length - 1; i++) {
          edges.push(
            lowerEdge(
              statement.chain[i],
              statement.chain[i + 1],
              statement.attributes,
              edgeDefaults,
            ),
          );
        }
        break;
    }
  }

  const graphLabel =
    graphAssignments.get("label") ?? attrValue(graphAttributes, "label") ?? document.id;
  const rankdir =
    graphAssignments.get("rankdir") ?? attrValue(graphAttributes, "rankdir");

  return graphDiagram(Array.from(nodes.values()), edges, {
    direction: directionFromRankdir(rankdir),
    title: graphLabel,
  });
}

export function parseDotToGraphDiagram(source: string): GraphDiagram {
  return dotAstToGraphDiagram(parseDot(source));
}
