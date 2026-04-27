import { describe, expect, it } from "vitest";
import {
  dotAstToGraphDiagram,
  parseDot,
  parseDotToGraphDiagram,
  tokenizeDot,
} from "../src/index.js";

describe("parseDot", () => {
  it("parses a minimal digraph", () => {
    const document = parseDot(`
      digraph G {
        A -> B;
      }
    `);

    expect(document.kind).toBe("digraph");
    expect(document.id).toBe("G");
    expect(document.statements).toHaveLength(1);
    expect(document.statements[0].kind).toBe("edge_stmt");
  });

  it("tokenizes DOT through the shared grammar files", () => {
    const tokens = tokenizeDot('strict DiGraph G { A -> B; }');

    expect(tokens.map((token) => `${token.type}:${token.value}`)).toEqual([
      "KEYWORD:STRICT",
      "KEYWORD:DIGRAPH",
      "NAME:G",
      "LBRACE:{",
      "NAME:A",
      "EDGEOP:->",
      "NAME:B",
      "SEMICOLON:;",
      "RBRACE:}",
      "EOF:",
    ]);
  });

  it("parses node statements and attributes", () => {
    const document = parseDot(`
      digraph {
        A [label="Start" shape=diamond];
      }
    `);

    const statement = document.statements[0];
    expect(statement.kind).toBe("node_stmt");
    if (statement.kind === "node_stmt") {
      expect(statement.id).toBe("A");
      expect(statement.attributes).toEqual([
        { key: "label", value: "Start" },
        { key: "shape", value: "diamond" },
      ]);
    }
  });

  it("parses keywords case-insensitively through the grammar-driven lexer", () => {
    const document = parseDot(`
      Strict DiGraph Demo {
        Node [shape=ellipse];
        A -> B;
      }
    `);

    expect(document.strict).toBe(true);
    expect(document.id).toBe("Demo");
    expect(document.statements[0]).toEqual({
      kind: "attr_stmt",
      target: "node",
      attributes: [{ key: "shape", value: "ellipse" }],
    });
  });

  it("parses edge chains", () => {
    const document = parseDot(`
      digraph {
        A -> B -> C [label="flow"];
      }
    `);

    const statement = document.statements[0];
    expect(statement.kind).toBe("edge_stmt");
    if (statement.kind === "edge_stmt") {
      expect(statement.chain).toEqual(["A", "B", "C"]);
      expect(statement.attributes).toEqual([{ key: "label", value: "flow" }]);
    }
  });
});

describe("dotAstToGraphDiagram", () => {
  it("lowers DOT statements into graph diagram nodes and edges", () => {
    const diagram = parseDotToGraphDiagram(`
      digraph Demo {
        rankdir=LR;
        A [label="Start"];
        A -> B [label="next"];
      }
    `);

    expect(diagram.direction).toBe("lr");
    expect(diagram.nodes).toHaveLength(2);
    expect(diagram.edges).toHaveLength(1);
    expect(diagram.nodes.find((node) => node.id === "A")?.label.text).toBe("Start");
    expect(diagram.edges[0].label?.text).toBe("next");
  });

  it("applies node defaults and graph title", () => {
    const ast = parseDot(`
      digraph G {
        node [shape=ellipse, fontcolor=blue];
        label="My Graph";
        A;
      }
    `);

    const diagram = dotAstToGraphDiagram(ast);
    expect(diagram.title).toBe("My Graph");
    expect(diagram.nodes[0].shape).toBe("ellipse");
    expect(diagram.nodes[0].style?.textColor).toBe("blue");
  });

  it("supports repeated bracket attribute lists", () => {
    const diagram = parseDotToGraphDiagram(`
      digraph Demo {
        A [label="Start"][shape=diamond];
      }
    `);

    expect(diagram.nodes).toHaveLength(1);
    expect(diagram.nodes[0].label.text).toBe("Start");
    expect(diagram.nodes[0].shape).toBe("diamond");
  });
});
