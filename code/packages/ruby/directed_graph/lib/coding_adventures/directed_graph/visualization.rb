# frozen_string_literal: true

# --------------------------------------------------------------------------
# visualization.rb — Graph Visualization in DOT, Mermaid, and ASCII Formats
# --------------------------------------------------------------------------
#
# This module provides three complementary visualization functions for directed
# graphs, available as module methods on CodingAdventures::DirectedGraph:
#
# 1. **to_dot** — Graphviz DOT format.  The gold standard for graph
#    visualization.  Render with: `dot -Tpng graph.dot -o graph.png`
#
# 2. **to_mermaid** — Mermaid diagram syntax.  Renders directly in GitHub
#    Markdown, Notion, and many documentation platforms.
#
# 3. **to_ascii_table** — Plain-text representation.  Works everywhere:
#    terminals, log files, IRB sessions.
#
# == Why Three Formats?
#
# Different contexts demand different formats:
#
# - Debugging in a terminal? → to_ascii_table (no tools needed)
# - Writing documentation?   → to_mermaid (renders in Markdown)
# - Publication-quality?     → to_dot (maximum control)
#
# == How It Works with Both Graph Types
#
# Both Graph and LabeledGraph are supported.  We use `is_a?(LabeledGraph)`
# to detect the graph type and adjust behavior:
#
# - Graph:        Edges have no labels.  DOT arrows are plain.
# - LabeledGraph: Edges carry string labels.  DOT gets [label="..."].
#
# When a labeled graph has multiple labels between the same pair of nodes
# (e.g. A→B with labels "x" and "y"), we combine them: "x, y".
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    # == DOT String Escaping
    #
    # DOT uses double-quoted strings for labels and node names.  We need to
    # escape characters that have special meaning inside those quotes:
    #
    #   "  → \"    (terminates the string)
    #   \  → \\   (escape character itself)
    #   <  → \<   (HTML label start)
    #   >  → \>   (HTML label end)
    #   {  → \{   (record label field separator)
    #   }  → \}   (record label field separator)
    #   |  → \|   (record label field separator)
    def self.escape_dot(text)
      text.gsub("\\", "\\\\\\\\")
        .gsub('"', '\\"')
        .gsub("<", "\\<")
        .gsub(">", "\\>")
        .gsub("{", "\\{")
        .gsub("}", "\\}")
        .gsub("|", "\\|")
    end

    # == Mermaid String Escaping
    #
    # Mermaid has issues with certain characters in labels.  We replace
    # double quotes with single quotes since Mermaid labels are typically
    # quoted with double quotes.
    def self.escape_mermaid(text)
      text.tr('"', "'")
    end

    # ======================================================================
    # to_dot — Graphviz DOT Format
    # ======================================================================
    #
    # DOT is a plain-text graph description language.  A simple graph:
    #
    #     digraph G {
    #         rankdir=LR;
    #         "A" -> "B";
    #         "B" -> "C";
    #     }
    #
    # Render with: dot -Tpng graph.dot -o graph.png
    #
    # == Parameters
    #
    # - graph:      A Graph or LabeledGraph instance.
    # - name:       The name of the digraph (appears in the DOT header).
    # - node_attrs: Hash mapping node names to attribute hashes.
    #               e.g. {"q1" => {"shape" => "doublecircle"}}
    # - edge_attrs: Hash mapping [source, target] pairs to attribute hashes.
    # - initial:    If set, adds an invisible start node with arrow.
    # - rankdir:    Layout direction ("LR" or "TB").
    #
    # Returns a valid DOT format string.
    def self.to_dot(graph, name: "G", node_attrs: {}, edge_attrs: {}, initial: nil, rankdir: "LR")
      lines = []

      # --- Header ---
      lines << "digraph #{escape_dot(name)} {"
      lines << "    rankdir=#{rankdir};"
      lines << ""

      # --- Initial state arrow ---
      # Standard FSM convention: invisible "point" node with arrow to
      # the initial state.
      unless initial.nil?
        lines << "    __start [shape=point, width=0.2];"
        lines << "    __start -> \"#{escape_dot(initial.to_s)}\";"
        lines << ""
      end

      # --- Node declarations ---
      sorted_nodes = graph.nodes.sort_by(&:to_s)
      sorted_nodes.each do |node|
        node_str = escape_dot(node.to_s)
        attrs = node_attrs.fetch(node.to_s, {})
        if attrs.empty?
          lines << "    \"#{node_str}\";"
        else
          attr_str = attrs.sort.map { |k, v| "#{k}=#{v}" }.join(", ")
          lines << "    \"#{node_str}\" [#{attr_str}];"
        end
      end
      lines << ""

      # --- Edge declarations ---
      is_labeled = graph.is_a?(LabeledGraph)

      if is_labeled
        grouped = collect_labeled_edges(graph)
        grouped.sort_by { |(from, to), _| [from.to_s, to.to_s] }.each do |(from_node, to_node), labels|
          from_str = escape_dot(from_node.to_s)
          to_str = escape_dot(to_node.to_s)
          combined_label = labels.join(", ")

          all_attrs = {"label" => "\"#{escape_dot(combined_label)}\""}
          user_attrs = edge_attrs.fetch([from_node.to_s, to_node.to_s], {})
          user_attrs.each { |k, v| all_attrs[k] = v }

          attr_str = all_attrs.sort.map { |k, v| "#{k}=#{v}" }.join(", ")
          lines << "    \"#{from_str}\" -> \"#{to_str}\" [#{attr_str}];"
        end
      else
        collect_unlabeled_edges(graph).each do |from_node, to_node|
          from_str = escape_dot(from_node.to_s)
          to_str = escape_dot(to_node.to_s)

          user_attrs = edge_attrs.fetch([from_node.to_s, to_node.to_s], {})
          if user_attrs.empty?
            lines << "    \"#{from_str}\" -> \"#{to_str}\";"
          else
            attr_str = user_attrs.sort.map { |k, v| "#{k}=#{v}" }.join(", ")
            lines << "    \"#{from_str}\" -> \"#{to_str}\" [#{attr_str}];"
          end
        end
      end

      lines << "}"
      lines.join("\n")
    end

    # ======================================================================
    # to_mermaid — Mermaid Diagram Syntax
    # ======================================================================
    #
    # Mermaid is a JavaScript-based diagramming tool that renders directly
    # in Markdown.  A simple flowchart:
    #
    #     graph LR
    #         A --> B
    #         B --> C
    #
    # For labeled edges:
    #
    #     graph LR
    #         A -->|"coin"| B
    #
    # == Parameters
    #
    # - graph:     A Graph or LabeledGraph instance.
    # - direction: Layout direction ("LR" or "TD").
    # - initial:   If set, adds invisible start node with arrow.
    #
    # Returns a Mermaid diagram string.
    def self.to_mermaid(graph, direction: "LR", initial: nil)
      lines = []

      # --- Header ---
      lines << "graph #{direction}"

      # --- Initial state marker ---
      unless initial.nil?
        lines << "    __start(( )) --> #{escape_mermaid(initial.to_s)}"
      end

      # --- Edges ---
      is_labeled = graph.is_a?(LabeledGraph)

      if is_labeled
        grouped = collect_labeled_edges(graph)
        grouped.sort_by { |(from, to), _| [from.to_s, to.to_s] }.each do |(from_node, to_node), labels|
          from_str = escape_mermaid(from_node.to_s)
          to_str = escape_mermaid(to_node.to_s)
          combined_label = labels.join(", ")
          escaped_label = escape_mermaid(combined_label)
          lines << "    #{from_str} -->|\"#{escaped_label}\"| #{to_str}"
        end
      else
        collect_unlabeled_edges(graph).each do |from_node, to_node|
          from_str = escape_mermaid(from_node.to_s)
          to_str = escape_mermaid(to_node.to_s)
          lines << "    #{from_str} --> #{to_str}"
        end
      end

      # --- Isolated nodes ---
      nodes_in_edges = Set.new
      if is_labeled
        graph.edges.each do |from_node, to_node, _label|
          nodes_in_edges.add(from_node.to_s)
          nodes_in_edges.add(to_node.to_s)
        end
      else
        graph.edges.each do |from_node, to_node|
          nodes_in_edges.add(from_node.to_s)
          nodes_in_edges.add(to_node.to_s)
        end
      end

      graph.nodes.sort_by(&:to_s).each do |node|
        unless nodes_in_edges.include?(node.to_s)
          lines << "    #{escape_mermaid(node.to_s)}"
        end
      end

      lines.join("\n")
    end

    # ======================================================================
    # to_ascii_table — Plain-Text Visualization
    # ======================================================================
    #
    # For labeled graphs, produces a transition table:
    #
    #     State    | coin     | push
    #     ---------+----------+---------
    #     locked   | unlocked | locked
    #     unlocked | unlocked | locked
    #
    # For unlabeled graphs, produces an adjacency list:
    #
    #     Node | Successors
    #     -----+-----------
    #     A    | B, C
    #     B    | C
    #     C    | (none)
    #
    # Returns a formatted ASCII table string.
    def self.to_ascii_table(graph)
      if graph.is_a?(LabeledGraph)
        ascii_table_labeled(graph)
      else
        ascii_table_unlabeled(graph)
      end
    end

    # === Private Helpers ===

    # Collect all edge labels grouped by [source, target] pair.
    # Returns a Hash: { [from, to] => [sorted labels] }
    def self.collect_labeled_edges(graph)
      grouped = {}
      graph.edges.each do |from_node, to_node, label|
        key = [from_node, to_node]
        grouped[key] ||= []
        grouped[key] << label
      end
      grouped.each_value(&:sort!)
      # Deduplicate labels (edges() may return sorted, but be safe)
      grouped.each { |key, labels| grouped[key] = labels.uniq }
      grouped
    end

    # Collect all edges from an unlabeled graph, sorted.
    # Returns an array of [from, to] pairs.
    def self.collect_unlabeled_edges(graph)
      graph.edges.sort_by { |from_node, to_node| [from_node.to_s, to_node.to_s] }
    end

    # Build a transition table for a labeled graph.
    def self.ascii_table_labeled(graph)
      sorted_nodes = graph.nodes.sort_by(&:to_s)

      return "(empty graph)" if sorted_nodes.empty?

      # Collect all unique labels
      all_labels = Set.new
      graph.edges.each { |_, _, label| all_labels.add(label) }
      sorted_labels = all_labels.to_a.sort

      return ascii_table_unlabeled_from_nodes(sorted_nodes) if sorted_labels.empty?

      # Build transition map: [node_str, label] => sorted list of target strings
      transition_map = {}
      graph.edges.each do |from_node, to_node, label|
        key = [from_node.to_s, label]
        transition_map[key] ||= []
        transition_map[key] << to_node.to_s
      end
      transition_map.each_value(&:sort!)
      transition_map.each { |key, targets| transition_map[key] = targets.uniq }

      # Calculate column widths
      state_col_width = [5, sorted_nodes.map { |n| n.to_s.length }.max].max # "State" = 5

      label_col_widths = sorted_labels.map do |label|
        max_cell = label.length
        sorted_nodes.each do |node|
          key = [node.to_s, label]
          cell = transition_map.key?(key) ? transition_map[key].join(", ") : "-"
          max_cell = [max_cell, cell.length].max
        end
        max_cell
      end

      # Build header
      header_parts = ["State".ljust(state_col_width)]
      sorted_labels.each_with_index do |label, i|
        header_parts << " #{label.ljust(label_col_widths[i])}"
      end
      header = header_parts.join(" |")

      # Build separator
      sep_parts = ["-" * state_col_width]
      label_col_widths.each do |width|
        sep_parts << "-" * (width + 1)
      end
      separator = sep_parts.join("-+")

      # Build rows
      rows = [header, separator]
      sorted_nodes.each do |node|
        row_parts = [node.to_s.ljust(state_col_width)]
        sorted_labels.each_with_index do |label, i|
          key = [node.to_s, label]
          cell = transition_map.key?(key) ? transition_map[key].join(", ") : "-"
          row_parts << " #{cell.ljust(label_col_widths[i])}"
        end
        rows << row_parts.join(" |")
      end

      rows.join("\n")
    end

    # Build an adjacency list table for an unlabeled graph.
    def self.ascii_table_unlabeled(graph)
      sorted_nodes = graph.nodes.sort_by(&:to_s)

      return "(empty graph)" if sorted_nodes.empty?

      # Build successor strings
      successor_strs = {}
      sorted_nodes.each do |node|
        succs = graph.successors(node).sort_by(&:to_s)
        successor_strs[node.to_s] = succs.empty? ? "(none)" : succs.map(&:to_s).join(", ")
      end

      # Calculate column widths
      node_col_width = [4, sorted_nodes.map { |n| n.to_s.length }.max].max # "Node" = 4
      succ_col_width = [10, successor_strs.values.map(&:length).max].max # "Successors" = 10

      # Build table
      header = "#{" Node".rstrip.ljust(node_col_width)} | #{"Successors".ljust(succ_col_width)}"
      separator = "#{"-" * node_col_width}-+-#{"-" * succ_col_width}"

      rows = [header, separator]
      sorted_nodes.each do |node|
        rows << "#{node.to_s.ljust(node_col_width)} | #{successor_strs[node.to_s].ljust(succ_col_width)}"
      end

      rows.join("\n")
    end

    # Build a simple node listing when there are no edges.
    def self.ascii_table_unlabeled_from_nodes(sorted_nodes)
      node_col_width = [4, sorted_nodes.map { |n| n.to_s.length }.max].max

      header = "#{"Node".ljust(node_col_width)} | Successors"
      separator = "#{"-" * node_col_width}-+-#{"-" * 10}"

      rows = [header, separator]
      sorted_nodes.each do |node|
        rows << "#{node.to_s.ljust(node_col_width)} | (none)"
      end

      rows.join("\n")
    end

    private_class_method :collect_labeled_edges, :collect_unlabeled_edges,
      :ascii_table_labeled, :ascii_table_unlabeled,
      :ascii_table_unlabeled_from_nodes
  end
end
