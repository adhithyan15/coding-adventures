defmodule CodingAdventures.DirectedGraph.VisualizationTest do
  @moduledoc """
  Tests for the Visualization module.

  We test all three output formats (DOT, Mermaid, ASCII) with both graph types
  (Graph and LabeledGraph). Tests verify structural correctness and format validity.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.DirectedGraph.Graph
  alias CodingAdventures.DirectedGraph.LabeledGraph
  alias CodingAdventures.DirectedGraph.Visualization

  # ======================================================================
  # Helpers: reusable graph builders
  # ======================================================================

  defp simple_dag do
    g = Graph.new()
    {:ok, g} = Graph.add_edge(g, "A", "B")
    {:ok, g} = Graph.add_edge(g, "B", "C")
    g
  end

  defp diamond_dag do
    g = Graph.new()
    {:ok, g} = Graph.add_edge(g, "A", "B")
    {:ok, g} = Graph.add_edge(g, "A", "C")
    {:ok, g} = Graph.add_edge(g, "B", "D")
    {:ok, g} = Graph.add_edge(g, "C", "D")
    g
  end

  defp turnstile do
    lg = LabeledGraph.new()
    {:ok, lg} = LabeledGraph.add_edge(lg, "locked", "unlocked", "coin")
    {:ok, lg} = LabeledGraph.add_edge(lg, "locked", "locked", "push")
    {:ok, lg} = LabeledGraph.add_edge(lg, "unlocked", "unlocked", "coin")
    {:ok, lg} = LabeledGraph.add_edge(lg, "unlocked", "locked", "push")
    lg
  end

  defp multi_label_graph do
    lg = LabeledGraph.new()
    {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
    {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "y")
    {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "z")
    lg
  end

  defp self_loop_graph do
    lg = LabeledGraph.new()
    {:ok, lg} = LabeledGraph.add_edge(lg, "q0", "q0", "a")
    {:ok, lg} = LabeledGraph.add_edge(lg, "q0", "q1", "b")
    lg
  end

  # ======================================================================
  # Tests: escape_dot
  # ======================================================================

  test "escape_dot plain text" do
    assert Visualization.escape_dot("hello") == "hello"
  end

  test "escape_dot double quotes" do
    result = Visualization.escape_dot(~s(say "hi"))
    assert String.contains?(result, ~s(\\"))
  end

  test "escape_dot angle brackets" do
    result = Visualization.escape_dot("<html>")
    assert String.contains?(result, "\\<")
    assert String.contains?(result, "\\>")
  end

  test "escape_dot braces and pipe" do
    result = Visualization.escape_dot("{a|b}")
    assert String.contains?(result, "\\{")
    assert String.contains?(result, "\\|")
    assert String.contains?(result, "\\}")
  end

  # ======================================================================
  # Tests: escape_mermaid
  # ======================================================================

  test "escape_mermaid plain text" do
    assert Visualization.escape_mermaid("hello") == "hello"
  end

  test "escape_mermaid double quotes replaced" do
    assert Visualization.escape_mermaid(~s(say "hi")) == "say 'hi'"
  end

  # ======================================================================
  # Tests: to_dot with Graph (unlabeled)
  # ======================================================================

  test "dot directed basic structure" do
    result = Visualization.to_dot(simple_dag())
    assert String.starts_with?(result, "digraph G {")
    assert String.ends_with?(result, "}")
    assert String.contains?(result, "rankdir=LR;")
  end

  test "dot directed nodes present" do
    result = Visualization.to_dot(simple_dag())
    assert String.contains?(result, ~s("A";))
    assert String.contains?(result, ~s("B";))
    assert String.contains?(result, ~s("C";))
  end

  test "dot directed edges present" do
    result = Visualization.to_dot(simple_dag())
    assert String.contains?(result, ~s("A" -> "B";))
    assert String.contains?(result, ~s("B" -> "C";))
  end

  test "dot directed no labels on edges" do
    result = Visualization.to_dot(simple_dag())
    refute String.contains?(result, "label=")
  end

  test "dot directed custom name" do
    result = Visualization.to_dot(simple_dag(), name: "MyGraph")
    assert String.contains?(result, "digraph MyGraph {")
  end

  test "dot directed custom rankdir" do
    result = Visualization.to_dot(simple_dag(), rankdir: "TB")
    assert String.contains?(result, "rankdir=TB;")
  end

  test "dot directed diamond graph" do
    result = Visualization.to_dot(diamond_dag())
    assert String.contains?(result, ~s("A" -> "B";))
    assert String.contains?(result, ~s("A" -> "C";))
    assert String.contains?(result, ~s("B" -> "D";))
    assert String.contains?(result, ~s("C" -> "D";))
  end

  test "dot directed node attrs" do
    result =
      Visualization.to_dot(simple_dag(),
        node_attrs: %{"A" => %{"shape" => "box", "color" => "red"}}
      )

    assert String.contains?(result, ~s("A" [color=red, shape=box];))
    assert String.contains?(result, ~s("B";))
  end

  test "dot directed edge attrs" do
    result =
      Visualization.to_dot(simple_dag(),
        edge_attrs: %{{"A", "B"} => %{"color" => "blue", "style" => "dashed"}}
      )

    assert String.contains?(result, "color=blue")
    assert String.contains?(result, "style=dashed")
  end

  test "dot directed initial state" do
    result = Visualization.to_dot(simple_dag(), initial: "A")
    assert String.contains?(result, "__start [shape=point, width=0.2];")
    assert String.contains?(result, ~s(__start -> "A";))
  end

  test "dot directed empty graph" do
    g = Graph.new()
    result = Visualization.to_dot(g)
    assert String.contains?(result, "digraph G {")
    assert String.contains?(result, "}")
  end

  test "dot directed isolated nodes" do
    g = Graph.new()
    {:ok, g} = Graph.add_node(g, "X")
    {:ok, g} = Graph.add_node(g, "Y")
    result = Visualization.to_dot(g)
    assert String.contains?(result, ~s("X";))
    assert String.contains?(result, ~s("Y";))
    refute String.contains?(result, "->")
  end

  # ======================================================================
  # Tests: to_dot with LabeledGraph
  # ======================================================================

  test "dot labeled edge labels" do
    result = Visualization.to_dot(turnstile())
    assert String.contains?(result, ~s(label="coin"))
    assert String.contains?(result, ~s(label="push"))
  end

  test "dot labeled multi labels combined" do
    result = Visualization.to_dot(multi_label_graph())
    assert String.contains?(result, ~s(label="x, y"))
    assert String.contains?(result, ~s(label="z"))
  end

  test "dot labeled self loop" do
    result = Visualization.to_dot(self_loop_graph())
    assert String.contains?(result, ~s("q0" -> "q0"))
    assert String.contains?(result, ~s("q0" -> "q1"))
  end

  test "dot labeled turnstile all edges" do
    result = Visualization.to_dot(turnstile())
    assert String.contains?(result, ~s("locked" -> "locked"))
    assert String.contains?(result, ~s("locked" -> "unlocked"))
    assert String.contains?(result, ~s("unlocked" -> "locked"))
    assert String.contains?(result, ~s("unlocked" -> "unlocked"))
  end

  test "dot labeled initial state" do
    result = Visualization.to_dot(turnstile(), initial: "locked")
    assert String.contains?(result, ~s(__start -> "locked";))
  end

  test "dot labeled node attrs" do
    result =
      Visualization.to_dot(turnstile(),
        node_attrs: %{"unlocked" => %{"shape" => "doublecircle"}}
      )

    assert String.contains?(result, ~s("unlocked" [shape=doublecircle];))
  end

  test "dot labeled empty" do
    lg = LabeledGraph.new()
    result = Visualization.to_dot(lg)
    assert String.contains?(result, "digraph G {")
  end

  # ======================================================================
  # Tests: to_mermaid with Graph (unlabeled)
  # ======================================================================

  test "mermaid directed basic structure" do
    result = Visualization.to_mermaid(simple_dag())
    assert String.starts_with?(result, "graph LR")
  end

  test "mermaid directed edges" do
    result = Visualization.to_mermaid(simple_dag())
    assert String.contains?(result, "A --> B")
    assert String.contains?(result, "B --> C")
  end

  test "mermaid directed no label syntax" do
    result = Visualization.to_mermaid(simple_dag())
    refute String.contains?(result, "-->|")
  end

  test "mermaid directed td direction" do
    result = Visualization.to_mermaid(simple_dag(), direction: "TD")
    assert String.starts_with?(result, "graph TD")
  end

  test "mermaid directed initial state" do
    result = Visualization.to_mermaid(simple_dag(), initial: "A")
    assert String.contains?(result, "__start(( ))")
    assert String.contains?(result, "__start(( )) --> A")
  end

  test "mermaid directed empty" do
    g = Graph.new()
    result = Visualization.to_mermaid(g)
    assert result == "graph LR"
  end

  test "mermaid directed isolated nodes" do
    g = Graph.new()
    {:ok, g} = Graph.add_node(g, "X")
    {:ok, g} = Graph.add_node(g, "Y")
    result = Visualization.to_mermaid(g)
    assert String.contains?(result, "X")
    assert String.contains?(result, "Y")
  end

  test "mermaid directed diamond" do
    result = Visualization.to_mermaid(diamond_dag())
    assert String.contains?(result, "A --> B")
    assert String.contains?(result, "A --> C")
    assert String.contains?(result, "B --> D")
    assert String.contains?(result, "C --> D")
  end

  # ======================================================================
  # Tests: to_mermaid with LabeledGraph
  # ======================================================================

  test "mermaid labeled edge syntax" do
    result = Visualization.to_mermaid(turnstile())
    assert String.contains?(result, ~s(-->|"coin"|))
    assert String.contains?(result, ~s(-->|"push"|))
  end

  test "mermaid labeled multi labels combined" do
    result = Visualization.to_mermaid(multi_label_graph())
    assert String.contains?(result, ~s(-->|"x, y"|))
    assert String.contains?(result, ~s(-->|"z"|))
  end

  test "mermaid labeled self loop" do
    result = Visualization.to_mermaid(self_loop_graph())
    assert String.contains?(result, ~s(q0 -->|"a"| q0))
    assert String.contains?(result, ~s(q0 -->|"b"| q1))
  end

  test "mermaid labeled initial state" do
    result = Visualization.to_mermaid(turnstile(), initial: "locked")
    assert String.contains?(result, "__start(( )) --> locked")
  end

  test "mermaid labeled td direction" do
    result = Visualization.to_mermaid(turnstile(), direction: "TD")
    assert String.starts_with?(result, "graph TD")
  end

  test "mermaid labeled empty" do
    lg = LabeledGraph.new()
    result = Visualization.to_mermaid(lg)
    assert result == "graph LR"
  end

  # ======================================================================
  # Tests: to_ascii_table with Graph (unlabeled)
  # ======================================================================

  test "ascii directed header present" do
    result = Visualization.to_ascii_table(simple_dag())
    assert String.contains?(result, "Node")
    assert String.contains?(result, "Successors")
  end

  test "ascii directed separator present" do
    result = Visualization.to_ascii_table(simple_dag())
    lines = String.split(result, "\n")
    assert Enum.any?(lines, fn line -> String.contains?(line, "+") and String.contains?(line, "-") end)
  end

  test "ascii directed node successors" do
    result = Visualization.to_ascii_table(simple_dag())
    assert String.contains?(result, "A")
    assert String.contains?(result, "B")
    assert String.contains?(result, "(none)")
  end

  test "ascii directed multiple successors" do
    result = Visualization.to_ascii_table(diamond_dag())
    lines = String.split(result, "\n")
    a_line = Enum.find(lines, fn line -> String.starts_with?(line, "A") end)
    assert String.contains?(a_line, "B")
    assert String.contains?(a_line, "C")
  end

  test "ascii directed empty graph" do
    g = Graph.new()
    result = Visualization.to_ascii_table(g)
    assert result == "(empty graph)"
  end

  test "ascii directed isolated nodes" do
    g = Graph.new()
    {:ok, g} = Graph.add_node(g, "X")
    {:ok, g} = Graph.add_node(g, "Y")
    {:ok, g} = Graph.add_node(g, "Z")
    result = Visualization.to_ascii_table(g)

    count =
      result
      |> String.split("(none)")
      |> length()
      |> Kernel.-(1)

    assert count == 3
  end

  # ======================================================================
  # Tests: to_ascii_table with LabeledGraph
  # ======================================================================

  test "ascii labeled header has labels" do
    result = Visualization.to_ascii_table(turnstile())
    first_line = result |> String.split("\n") |> List.first()
    assert String.contains?(first_line, "State")
    assert String.contains?(first_line, "coin")
    assert String.contains?(first_line, "push")
  end

  test "ascii labeled transition values" do
    result = Visualization.to_ascii_table(turnstile())
    lines = String.split(result, "\n")
    locked_line = Enum.find(lines, fn line -> String.starts_with?(line, "locked") end)
    assert String.contains?(locked_line, "unlocked")
  end

  test "ascii labeled missing transitions show dash" do
    lg = LabeledGraph.new()
    {:ok, lg} = LabeledGraph.add_edge(lg, "A", "B", "x")
    {:ok, lg} = LabeledGraph.add_edge(lg, "B", "C", "y")
    result = Visualization.to_ascii_table(lg)
    assert String.contains?(result, "-")
  end

  test "ascii labeled empty" do
    lg = LabeledGraph.new()
    result = Visualization.to_ascii_table(lg)
    assert result == "(empty graph)"
  end

  test "ascii labeled no edges" do
    lg = LabeledGraph.new()
    {:ok, lg} = LabeledGraph.add_node(lg, "A")
    {:ok, lg} = LabeledGraph.add_node(lg, "B")
    result = Visualization.to_ascii_table(lg)
    assert String.contains?(result, "Node")
    assert String.contains?(result, "(none)")
  end

  test "ascii labeled self loop" do
    result = Visualization.to_ascii_table(self_loop_graph())
    lines = String.split(result, "\n")
    q0_line = Enum.find(lines, fn line -> String.starts_with?(line, "q0") end)
    assert String.contains?(q0_line, "q0")
  end

  # ======================================================================
  # Integration / Real-world scenarios
  # ======================================================================

  test "turnstile dfa full dot" do
    lg = turnstile()

    result =
      Visualization.to_dot(lg,
        name: "Turnstile",
        initial: "locked",
        node_attrs: %{"unlocked" => %{"shape" => "doublecircle"}},
        rankdir: "LR"
      )

    assert String.contains?(result, "digraph Turnstile {")
    assert String.contains?(result, "rankdir=LR;")
    assert String.contains?(result, ~s(__start -> "locked";))
    assert String.contains?(result, ~s("unlocked" [shape=doublecircle];))
    assert String.contains?(result, ~s(label="coin"))
    assert String.contains?(result, ~s(label="push"))
  end

  test "build dependency graph" do
    g = Graph.new()
    {:ok, g} = Graph.add_edge(g, "logic-gates", "adder")
    {:ok, g} = Graph.add_edge(g, "adder", "alu")
    {:ok, g} = Graph.add_edge(g, "alu", "cpu")

    dot = Visualization.to_dot(g, name: "BuildDeps")
    assert String.contains?(dot, "digraph BuildDeps {")
    assert String.contains?(dot, ~s("logic-gates" -> "adder";))

    mermaid = Visualization.to_mermaid(g)
    assert String.contains?(mermaid, "logic-gates --> adder")

    table = Visualization.to_ascii_table(g)
    assert String.contains?(table, "logic-gates")
    assert String.contains?(table, "alu")
  end

  test "knowledge graph multi labels" do
    lg = LabeledGraph.new()
    {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Bob", "friend")
    {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Bob", "coworker")
    {:ok, lg} = LabeledGraph.add_edge(lg, "Alice", "Carol", "friend")

    dot = Visualization.to_dot(lg)
    assert String.contains?(dot, ~s(label="coworker, friend"))

    mermaid = Visualization.to_mermaid(lg)
    assert String.contains?(mermaid, ~s(-->|"coworker, friend"|))
  end

  test "dot balanced braces" do
    result = Visualization.to_dot(turnstile())
    open_count = result |> String.graphemes() |> Enum.count(&(&1 == "{"))
    close_count = result |> String.graphemes() |> Enum.count(&(&1 == "}"))
    assert open_count == close_count
    lines = String.split(result, "\n")
    assert String.starts_with?(List.first(lines), "digraph")
    assert List.last(lines) == "}"
  end
end
