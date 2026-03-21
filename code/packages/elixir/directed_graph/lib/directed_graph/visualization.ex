defmodule CodingAdventures.DirectedGraph.Visualization do
  @moduledoc """
  Graph Visualization in DOT, Mermaid, and ASCII Formats
  =======================================================

  This module provides three complementary visualization functions for directed
  graphs:

  1. **`to_dot/2`** -- Generates Graphviz DOT format, the gold standard for graph
     visualization. DOT files can be rendered to PNG, SVG, or PDF using the
     `dot` command-line tool (part of the Graphviz suite).

  2. **`to_mermaid/2`** -- Generates Mermaid diagram syntax, which renders directly
     in GitHub Markdown, Notion, and many documentation platforms.

  3. **`to_ascii_table/1`** -- Generates a plain-text representation that works
     everywhere: terminals, log files, IEx sessions.

  ## Why Three Formats?

  Different contexts demand different formats:

  - **Debugging in IEx?** Use `to_ascii_table/1` -- no tools needed.
  - **Writing documentation?** Use `to_mermaid/2` -- renders in Markdown.
  - **Publication-quality diagrams?** Use `to_dot/2` -- maximum control.

  ## How It Works with Both Graph Types

  Both `Graph` and `LabeledGraph` are supported. We use pattern matching on
  the struct type to detect which graph we're working with:

  - `%Graph{}`: Edges have no labels. DOT edges are plain arrows.
  - `%LabeledGraph{}`: Edges carry string labels. DOT edges get `[label="..."]`.

  When a labeled graph has multiple labels between the same pair of nodes,
  we combine them: `"a, b"`.
  """

  alias CodingAdventures.DirectedGraph.Graph
  alias CodingAdventures.DirectedGraph.LabeledGraph

  # ===========================================================================
  # DOT String Escaping
  # ===========================================================================
  #
  # DOT uses double-quoted strings for labels and node names. We escape
  # characters that have special meaning inside those quotes:
  #
  #   "  -> \"    (terminates the string)
  #   \  -> \\   (escape character itself)
  #   <  -> \<   (HTML label start)
  #   >  -> \>   (HTML label end)
  #   {  -> \{   (record label separator)
  #   }  -> \}   (record label separator)
  #   |  -> \|   (record label separator)

  @doc """
  Escape special characters for DOT label strings.

  ## Example

      iex> CodingAdventures.DirectedGraph.Visualization.escape_dot("hello")
      "hello"

      iex> CodingAdventures.DirectedGraph.Visualization.escape_dot(~s(say "hi"))
      ~s(say \\"hi\\")
  """
  @spec escape_dot(String.t()) :: String.t()
  def escape_dot(text) do
    text
    |> String.replace("\\", "\\\\")
    |> String.replace("\"", "\\\"")
    |> String.replace("<", "\\<")
    |> String.replace(">", "\\>")
    |> String.replace("{", "\\{")
    |> String.replace("}", "\\}")
    |> String.replace("|", "\\|")
  end

  # ===========================================================================
  # Mermaid String Escaping
  # ===========================================================================
  #
  # Mermaid has issues with certain characters in labels. We replace double
  # quotes with single quotes.

  @doc """
  Escape special characters for Mermaid labels.

  Replaces double quotes with single quotes since Mermaid labels are
  typically quoted with double quotes.
  """
  @spec escape_mermaid(String.t()) :: String.t()
  def escape_mermaid(text) do
    String.replace(text, "\"", "'")
  end

  # ===========================================================================
  # to_dot -- Graphviz DOT Format
  # ===========================================================================
  #
  # DOT is a plain-text graph description language:
  #
  #     digraph G {
  #         rankdir=LR;
  #         "A" -> "B";
  #         "B" -> "C";
  #     }
  #
  # Render with: dot -Tpng graph.dot -o graph.png

  @doc """
  Generate a Graphviz DOT representation of the graph.

  Works with both `Graph` and `LabeledGraph`. For labeled graphs, edges
  automatically get `[label="..."]` attributes.

  ## Options

  - `:name` - digraph name (default: `"G"`)
  - `:node_attrs` - map of node name (string) to attribute map
    (e.g., `%{"q1" => %{"shape" => "doublecircle"}}`)
  - `:edge_attrs` - map of `{source, target}` to attribute map
  - `:initial` - if set, adds invisible start node with arrow
  - `:rankdir` - layout direction, `"LR"` or `"TB"` (default: `"LR"`)

  ## Example

      alias CodingAdventures.DirectedGraph.{Graph, Visualization}
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      Visualization.to_dot(g)
  """
  @spec to_dot(Graph.t() | LabeledGraph.t(), keyword()) :: String.t()
  def to_dot(graph, opts \\ []) do
    name = Keyword.get(opts, :name, "G")
    node_attrs = Keyword.get(opts, :node_attrs, %{})
    edge_attrs = Keyword.get(opts, :edge_attrs, %{})
    initial = Keyword.get(opts, :initial, nil)
    rankdir = Keyword.get(opts, :rankdir, "LR")

    lines = []

    # --- Header ---
    lines = lines ++ ["digraph #{escape_dot(name)} {"]
    lines = lines ++ ["    rankdir=#{rankdir};"]
    lines = lines ++ [""]

    # --- Initial state arrow ---
    lines =
      if initial != nil do
        lines ++
          [
            "    __start [shape=point, width=0.2];",
            "    __start -> \"#{escape_dot(to_string(initial))}\";",
            ""
          ]
      else
        lines
      end

    # --- Node declarations ---
    sorted_nodes = graph_nodes(graph) |> Enum.sort_by(&to_string/1)

    lines =
      Enum.reduce(sorted_nodes, lines, fn node, acc ->
        node_str = escape_dot(to_string(node))
        attrs = Map.get(node_attrs, to_string(node), %{})

        if map_size(attrs) == 0 do
          acc ++ ["    \"#{node_str}\";"]
        else
          attr_str =
            attrs
            |> Enum.sort()
            |> Enum.map(fn {k, v} -> "#{k}=#{v}" end)
            |> Enum.join(", ")

          acc ++ ["    \"#{node_str}\" [#{attr_str}];"]
        end
      end)

    lines = lines ++ [""]

    # --- Edge declarations ---
    lines =
      case graph do
        %LabeledGraph{} ->
          grouped = collect_labeled_edges(graph)

          grouped
          |> Enum.sort_by(fn {{from, to}, _} -> {to_string(from), to_string(to)} end)
          |> Enum.reduce(lines, fn {{from_node, to_node}, labels}, acc ->
            from_str = escape_dot(to_string(from_node))
            to_str = escape_dot(to_string(to_node))
            combined_label = Enum.join(labels, ", ")

            all_attrs = %{"label" => "\"#{escape_dot(combined_label)}\""}

            user_attrs = Map.get(edge_attrs, {to_string(from_node), to_string(to_node)}, %{})

            all_attrs = Map.merge(all_attrs, user_attrs)

            attr_str =
              all_attrs
              |> Enum.sort()
              |> Enum.map(fn {k, v} -> "#{k}=#{v}" end)
              |> Enum.join(", ")

            acc ++ ["    \"#{from_str}\" -> \"#{to_str}\" [#{attr_str}];"]
          end)

        %Graph{} ->
          collect_unlabeled_edges(graph)
          |> Enum.reduce(lines, fn {from_node, to_node}, acc ->
            from_str = escape_dot(to_string(from_node))
            to_str = escape_dot(to_string(to_node))

            user_attrs =
              Map.get(edge_attrs, {to_string(from_node), to_string(to_node)}, %{})

            if map_size(user_attrs) == 0 do
              acc ++ ["    \"#{from_str}\" -> \"#{to_str}\";"]
            else
              attr_str =
                user_attrs
                |> Enum.sort()
                |> Enum.map(fn {k, v} -> "#{k}=#{v}" end)
                |> Enum.join(", ")

              acc ++ ["    \"#{from_str}\" -> \"#{to_str}\" [#{attr_str}];"]
            end
          end)
      end

    lines = lines ++ ["}"]
    Enum.join(lines, "\n")
  end

  # ===========================================================================
  # to_mermaid -- Mermaid Diagram Syntax
  # ===========================================================================
  #
  # Mermaid renders directly in GitHub Markdown:
  #
  #     graph LR
  #         A --> B
  #         A -->|"coin"| B

  @doc """
  Generate a Mermaid flowchart diagram of the graph.

  Works with both `Graph` and `LabeledGraph`.

  ## Options

  - `:direction` - layout direction, `"LR"` or `"TD"` (default: `"LR"`)
  - `:initial` - if set, adds invisible start node with arrow

  ## Example

      alias CodingAdventures.DirectedGraph.{Graph, Visualization}
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      Visualization.to_mermaid(g)
  """
  @spec to_mermaid(Graph.t() | LabeledGraph.t(), keyword()) :: String.t()
  def to_mermaid(graph, opts \\ []) do
    direction = Keyword.get(opts, :direction, "LR")
    initial = Keyword.get(opts, :initial, nil)

    lines = ["graph #{direction}"]

    # --- Initial state marker ---
    lines =
      if initial != nil do
        lines ++ ["    __start(( )) --> #{escape_mermaid(to_string(initial))}"]
      else
        lines
      end

    # --- Edges ---
    lines =
      case graph do
        %LabeledGraph{} ->
          grouped = collect_labeled_edges(graph)

          grouped
          |> Enum.sort_by(fn {{from, to}, _} -> {to_string(from), to_string(to)} end)
          |> Enum.reduce(lines, fn {{from_node, to_node}, labels}, acc ->
            from_str = escape_mermaid(to_string(from_node))
            to_str = escape_mermaid(to_string(to_node))
            combined_label = Enum.join(labels, ", ")
            escaped_label = escape_mermaid(combined_label)
            acc ++ ["    #{from_str} -->|\"#{escaped_label}\"| #{to_str}"]
          end)

        %Graph{} ->
          collect_unlabeled_edges(graph)
          |> Enum.reduce(lines, fn {from_node, to_node}, acc ->
            from_str = escape_mermaid(to_string(from_node))
            to_str = escape_mermaid(to_string(to_node))
            acc ++ ["    #{from_str} --> #{to_str}"]
          end)
      end

    # --- Isolated nodes ---
    nodes_in_edges = collect_nodes_in_edges(graph)

    lines =
      graph_nodes(graph)
      |> Enum.sort_by(&to_string/1)
      |> Enum.reduce(lines, fn node, acc ->
        if MapSet.member?(nodes_in_edges, to_string(node)) do
          acc
        else
          acc ++ ["    #{escape_mermaid(to_string(node))}"]
        end
      end)

    Enum.join(lines, "\n")
  end

  # ===========================================================================
  # to_ascii_table -- Plain-Text Visualization
  # ===========================================================================
  #
  # For labeled graphs: transition table (rows = nodes, columns = labels)
  # For unlabeled graphs: adjacency list

  @doc """
  Generate a plain-text table representation of the graph.

  For `LabeledGraph`: produces a transition table where rows are nodes,
  columns are unique labels, and cells show target nodes.

  For `Graph`: produces an adjacency list where each row shows a node
  and its successors.

  ## Example

      alias CodingAdventures.DirectedGraph.{Graph, Visualization}
      g = Graph.new()
      {:ok, g} = Graph.add_edge(g, "A", "B")
      {:ok, g} = Graph.add_edge(g, "B", "C")
      Visualization.to_ascii_table(g)
  """
  @spec to_ascii_table(Graph.t() | LabeledGraph.t()) :: String.t()
  def to_ascii_table(%LabeledGraph{} = graph), do: ascii_table_labeled(graph)
  def to_ascii_table(%Graph{} = graph), do: ascii_table_unlabeled(graph)

  # ===========================================================================
  # Private helpers
  # ===========================================================================

  # Get nodes from either graph type
  defp graph_nodes(%Graph{} = g), do: Graph.nodes(g)
  defp graph_nodes(%LabeledGraph{} = g), do: LabeledGraph.nodes(g)

  # Collect all edge labels grouped by {source, target} pair.
  # Returns a map: %{{from, to} => [sorted labels]}
  defp collect_labeled_edges(%LabeledGraph{} = graph) do
    LabeledGraph.edges(graph)
    |> Enum.reduce(%{}, fn {from_node, to_node, label}, acc ->
      key = {from_node, to_node}
      current = Map.get(acc, key, [])
      Map.put(acc, key, [label | current])
    end)
    |> Map.new(fn {key, labels} ->
      {key, labels |> Enum.uniq() |> Enum.sort()}
    end)
  end

  # Collect all edges from an unlabeled graph, sorted.
  defp collect_unlabeled_edges(%Graph{} = graph) do
    Graph.edges(graph)
    |> Enum.sort_by(fn {from, to} -> {to_string(from), to_string(to)} end)
  end

  # Collect all node names that appear in edges.
  defp collect_nodes_in_edges(%LabeledGraph{} = graph) do
    LabeledGraph.edges(graph)
    |> Enum.reduce(MapSet.new(), fn {from, to, _label}, acc ->
      acc |> MapSet.put(to_string(from)) |> MapSet.put(to_string(to))
    end)
  end

  defp collect_nodes_in_edges(%Graph{} = graph) do
    Graph.edges(graph)
    |> Enum.reduce(MapSet.new(), fn {from, to}, acc ->
      acc |> MapSet.put(to_string(from)) |> MapSet.put(to_string(to))
    end)
  end

  # Build a transition table for a labeled graph.
  defp ascii_table_labeled(%LabeledGraph{} = graph) do
    sorted_nodes = LabeledGraph.nodes(graph) |> Enum.sort_by(&to_string/1)

    if sorted_nodes == [] do
      "(empty graph)"
    else
      # Collect all unique labels
      all_labels =
        LabeledGraph.edges(graph)
        |> Enum.map(fn {_, _, label} -> label end)
        |> Enum.uniq()
        |> Enum.sort()

      if all_labels == [] do
        ascii_table_unlabeled_from_nodes(sorted_nodes)
      else
        do_ascii_table_labeled(graph, sorted_nodes, all_labels)
      end
    end
  end

  defp do_ascii_table_labeled(graph, sorted_nodes, sorted_labels) do
    # Build transition map: {node_str, label} => sorted list of target strings
    transition_map =
      LabeledGraph.edges(graph)
      |> Enum.reduce(%{}, fn {from_node, to_node, label}, acc ->
        key = {to_string(from_node), label}
        current = Map.get(acc, key, [])
        Map.put(acc, key, [to_string(to_node) | current])
      end)
      |> Map.new(fn {key, targets} ->
        {key, targets |> Enum.uniq() |> Enum.sort()}
      end)

    # Calculate column widths
    state_col_width =
      max(
        String.length("State"),
        sorted_nodes |> Enum.map(&(to_string(&1) |> String.length())) |> Enum.max()
      )

    label_col_widths =
      Enum.map(sorted_labels, fn label ->
        max_cell = String.length(label)

        Enum.reduce(sorted_nodes, max_cell, fn node, acc ->
          key = {to_string(node), label}

          cell =
            case Map.get(transition_map, key) do
              nil -> "-"
              targets -> Enum.join(targets, ", ")
            end

          max(acc, String.length(cell))
        end)
      end)

    # Build header
    header_parts = [String.pad_trailing("State", state_col_width)]

    label_header_parts =
      sorted_labels
      |> Enum.zip(label_col_widths)
      |> Enum.map(fn {label, width} -> " #{String.pad_trailing(label, width)}" end)

    header_parts = header_parts ++ label_header_parts

    header = Enum.join(header_parts, " |")

    # Build separator
    sep_parts = [String.duplicate("-", state_col_width)]

    label_sep_parts =
      Enum.map(label_col_widths, fn width -> String.duplicate("-", width + 1) end)

    sep_parts = sep_parts ++ label_sep_parts

    separator = Enum.join(sep_parts, "-+")

    # Build data rows
    data_rows =
      Enum.map(sorted_nodes, fn node ->
        row_parts = [String.pad_trailing(to_string(node), state_col_width)]

        label_cells =
          sorted_labels
          |> Enum.zip(label_col_widths)
          |> Enum.map(fn {label, width} ->
            key = {to_string(node), label}

            cell =
              case Map.get(transition_map, key) do
                nil -> "-"
                targets -> Enum.join(targets, ", ")
              end

            " #{String.pad_trailing(cell, width)}"
          end)

        row_parts = row_parts ++ label_cells

        Enum.join(row_parts, " |")
      end)

    Enum.join([header, separator | data_rows], "\n")
  end

  # Build an adjacency list table for an unlabeled graph.
  defp ascii_table_unlabeled(%Graph{} = graph) do
    sorted_nodes = Graph.nodes(graph) |> Enum.sort_by(&to_string/1)

    if sorted_nodes == [] do
      "(empty graph)"
    else
      # Build successor strings
      successor_strs =
        Map.new(sorted_nodes, fn node ->
          {:ok, succs} = Graph.successors(graph, node)
          sorted_succs = Enum.sort_by(succs, &to_string/1)

          str =
            if sorted_succs == [] do
              "(none)"
            else
              Enum.map_join(sorted_succs, ", ", &to_string/1)
            end

          {to_string(node), str}
        end)

      # Calculate column widths
      node_col_width =
        max(
          4,
          sorted_nodes |> Enum.map(&(to_string(&1) |> String.length())) |> Enum.max()
        )

      succ_col_width =
        max(
          10,
          successor_strs |> Map.values() |> Enum.map(&String.length/1) |> Enum.max()
        )

      # Build table
      header =
        "#{String.pad_trailing("Node", node_col_width)} | #{String.pad_trailing("Successors", succ_col_width)}"

      separator =
        "#{String.duplicate("-", node_col_width)}-+-#{String.duplicate("-", succ_col_width)}"

      data_rows =
        Enum.map(sorted_nodes, fn node ->
          "#{String.pad_trailing(to_string(node), node_col_width)} | #{String.pad_trailing(successor_strs[to_string(node)], succ_col_width)}"
        end)

      Enum.join([header, separator | data_rows], "\n")
    end
  end

  # Build a simple node listing when there are no edges.
  defp ascii_table_unlabeled_from_nodes(sorted_nodes) do
    node_col_width =
      max(4, sorted_nodes |> Enum.map(&(to_string(&1) |> String.length())) |> Enum.max())

    header = "#{String.pad_trailing("Node", node_col_width)} | Successors"
    separator = "#{String.duplicate("-", node_col_width)}-+-#{String.duplicate("-", 10)}"

    data_rows =
      Enum.map(sorted_nodes, fn node ->
        "#{String.pad_trailing(to_string(node), node_col_width)} | (none)"
      end)

    Enum.join([header, separator | data_rows], "\n")
  end
end
