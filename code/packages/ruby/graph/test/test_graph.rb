# frozen_string_literal: true

require "minitest/autorun"
require "set"
require_relative "../lib/coding_adventures_graph"

include CodingAdventures::Graph

class TestGraphConstruction < Minitest::Test
  def test_empty_graph_adjacency_list
    g = Graph.new(GraphRepr::ADJACENCY_LIST)
    assert_equal 0, g.length
    assert_equal [], g.nodes
    assert_equal [], g.edges
  end

  def test_empty_graph_adjacency_matrix
    g = Graph.new(GraphRepr::ADJACENCY_MATRIX)
    assert_equal 0, g.length
    assert_equal [], g.nodes
    assert_equal [], g.edges
  end

  def test_default_representation_is_adjacency_list
    g = Graph.new
    g.add_node("A")
    assert_equal 1, g.length
  end

  def test_repr_string
    g = Graph.new
    g.add_node("A")
    repr_str = g.to_s
    assert_includes repr_str, "Graph"
    assert_includes repr_str, "adjacency_list"
  end
end

class TestNodeOperations < Minitest::Test
  [GraphRepr::ADJACENCY_LIST, GraphRepr::ADJACENCY_MATRIX].each do |repr|
    define_method "test_add_node_#{repr}" do
      g = Graph.new(repr)
      g.add_node("A")
      assert g.has_node?("A")
      assert_equal 1, g.length
      assert g.nodes.include?("A")
    end

    define_method "test_add_multiple_nodes_#{repr}" do
      g = Graph.new(repr)
      g.add_node("A")
      g.add_node("B")
      g.add_node("C")
      assert_equal 3, g.length
      assert_equal Set.new(["A", "B", "C"]), Set.new(g.nodes)
    end

    define_method "test_add_duplicate_node_is_noop_#{repr}" do
      g = Graph.new(repr)
      g.add_node("A")
      g.add_node("A")
      assert_equal 1, g.length
    end

    define_method "test_remove_node_#{repr}" do
      g = Graph.new(repr)
      g.add_node("A")
      g.remove_node("A")
      assert_equal 0, g.length
      assert !g.has_node?("A")
    end

    define_method "test_remove_node_with_edges_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      g.add_edge("A", "C")
      g.remove_node("A")
      assert !g.has_node?("A")
      assert g.has_node?("B")
      assert g.has_node?("C")
      assert !g.has_edge?("A", "B")
      assert !g.has_edge?("A", "C")
      assert_equal 0, g.edges.length
    end

    define_method "test_remove_nonexistent_node_raises_error_#{repr}" do
      g = Graph.new(repr)
      assert_raises NodeNotFoundError do
        g.remove_node("X")
      end
    end

    define_method "test_has_node_#{repr}" do
      g = Graph.new(repr)
      assert !g.has_node?("A")
      g.add_node("A")
      assert g.has_node?("A")
    end
  end
end

class TestEdgeOperations < Minitest::Test
  [GraphRepr::ADJACENCY_LIST, GraphRepr::ADJACENCY_MATRIX].each do |repr|
    define_method "test_add_edge_creates_nodes_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      assert g.has_node?("A")
      assert g.has_node?("B")
    end

    define_method "test_add_edge_default_weight_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      assert_equal 1.0, g.edge_weight("A", "B")
      assert_equal 1.0, g.edge_weight("B", "A")
    end

    define_method "test_add_edge_with_weight_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B", 5.0)
      assert_equal 5.0, g.edge_weight("A", "B")
    end

    define_method "test_edge_is_undirected_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B", 5.0)
      assert g.has_edge?("A", "B")
      assert g.has_edge?("B", "A")
      assert_equal g.edge_weight("A", "B"), g.edge_weight("B", "A")
    end

    define_method "test_update_edge_weight_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B", 1.0)
      g.add_edge("A", "B", 5.0)
      assert_equal 5.0, g.edge_weight("A", "B")
    end

    define_method "test_remove_edge_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      g.remove_edge("A", "B")
      assert !g.has_edge?("A", "B")
      assert !g.has_edge?("B", "A")
    end

    define_method "test_remove_nonexistent_edge_raises_error_#{repr}" do
      g = Graph.new(repr)
      g.add_node("A")
      g.add_node("B")
      assert_raises EdgeNotFoundError do
        g.remove_edge("A", "B")
      end
    end

    define_method "test_remove_edge_missing_node_raises_error_#{repr}" do
      g = Graph.new(repr)
      g.add_node("A")
      assert_raises NodeNotFoundError do
        g.remove_edge("A", "B")
      end
    end

    define_method "test_has_edge_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      assert g.has_edge?("A", "B")
      assert g.has_edge?("B", "A")
      assert !g.has_edge?("A", "C")
    end

    define_method "test_has_edge_missing_node_#{repr}" do
      g = Graph.new(repr)
      assert !g.has_edge?("A", "B")
    end

    define_method "test_all_edges_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      g.add_edge("B", "C")
      g.add_edge("A", "C")
      assert_equal 3, g.edges.length

      # Check that edges are canonical (ordered)
      edge_set = Set.new(g.edges.map { |a, b, _| "#{a}-#{b}" })
      assert_equal 3, edge_set.length # No duplicates
    end
  end
end

class TestNeighbourhoodQueries < Minitest::Test
  [GraphRepr::ADJACENCY_LIST, GraphRepr::ADJACENCY_MATRIX].each do |repr|
    define_method "test_neighbors_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      g.add_edge("A", "C")
      g.add_edge("B", "D")
      assert_equal Set.new(["B", "C"]), Set.new(g.neighbors("A"))
      assert_equal Set.new(["A", "D"]), Set.new(g.neighbors("B"))
    end

    define_method "test_neighbors_nonexistent_raises_error_#{repr}" do
      g = Graph.new(repr)
      assert_raises NodeNotFoundError do
        g.neighbors("X")
      end
    end

    define_method "test_neighbors_weighted_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B", 2.0)
      g.add_edge("A", "C", 3.0)
      weighted = g.neighbors_weighted("A")
      assert_equal 2.0, weighted["B"]
      assert_equal 3.0, weighted["C"]
    end

    define_method "test_degree_#{repr}" do
      g = Graph.new(repr)
      g.add_edge("A", "B")
      g.add_edge("A", "C")
      g.add_edge("A", "D")
      assert_equal 3, g.degree("A")
      assert_equal 1, g.degree("B")
    end

    define_method "test_degree_nonexistent_raises_error_#{repr}" do
      g = Graph.new(repr)
      assert_raises NodeNotFoundError do
        g.degree("X")
      end
    end
  end
end

class TestBFS < Minitest::Test
  def test_bfs_breadth_first_order
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("A", "C")
    g.add_edge("B", "D")
    g.add_edge("C", "E")

    result = CodingAdventures::Graph.bfs(g, "A")
    assert_equal "A", result[0]
    # B and C should be before D and E
    index_b = result.index("B")
    index_c = result.index("C")
    index_d = result.index("D")
    index_e = result.index("E")
    assert [index_b, index_c].max < [index_d, index_e].min
  end

  def test_bfs_disconnected_graph
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("C", "D")

    result = CodingAdventures::Graph.bfs(g, "A")
    assert result.include?("A")
    assert result.include?("B")
    assert !result.include?("C")
    assert !result.include?("D")
  end
end

class TestDFS < Minitest::Test
  def test_dfs_depth_first_order
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("A", "C")
    g.add_edge("B", "D")

    result = CodingAdventures::Graph.dfs(g, "A")
    assert_equal 4, result.length
    assert_equal "A", result[0]
  end

  def test_dfs_disconnected_graph
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("C", "D")

    result = CodingAdventures::Graph.dfs(g, "A")
    assert result.include?("A")
    assert result.include?("B")
    assert !result.include?("C")
  end
end

class TestConnectivity < Minitest::Test
  def test_is_connected_true
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "A")
    assert CodingAdventures::Graph.is_connected?(g)
  end

  def test_is_connected_false
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("C", "D")
    assert !CodingAdventures::Graph.is_connected?(g)
  end

  def test_is_connected_empty
    g = Graph.new
    assert CodingAdventures::Graph.is_connected?(g)
  end

  def test_connected_components
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("D", "E")
    g.add_node("F")

    components = CodingAdventures::Graph.connected_components(g)
    assert_equal 3, components.length

    # Check component sizes
    sizes = components.map(&:size).sort
    assert_equal [1, 2, 3], sizes
  end
end

class TestCycleDetection < Minitest::Test
  def test_has_cycle_true
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "A")
    assert CodingAdventures::Graph.has_cycle?(g)
  end

  def test_has_cycle_false
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("B", "D")
    assert !CodingAdventures::Graph.has_cycle?(g)
  end

  def test_has_cycle_single_node
    g = Graph.new
    g.add_node("A")
    assert !CodingAdventures::Graph.has_cycle?(g)
  end

  def test_has_cycle_disconnected
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "A")
    g.add_edge("D", "E")
    assert CodingAdventures::Graph.has_cycle?(g)
  end
end

class TestShortestPath < Minitest::Test
  def test_shortest_path_unweighted
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "D")
    g.add_edge("A", "D")

    path = CodingAdventures::Graph.shortest_path(g, "A", "D")
    assert_equal ["A", "D"], path
  end

  def test_shortest_path_no_path
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("C", "D")

    path = CodingAdventures::Graph.shortest_path(g, "A", "D")
    assert_equal [], path
  end

  def test_shortest_path_same_node
    g = Graph.new
    g.add_node("A")

    assert_equal ["A"], CodingAdventures::Graph.shortest_path(g, "A", "A")
  end

  def test_shortest_path_weighted
    g = Graph.new
    g.add_edge("A", "B", 1.0)
    g.add_edge("B", "D", 10.0)
    g.add_edge("A", "C", 3.0)
    g.add_edge("C", "D", 3.0)

    path = CodingAdventures::Graph.shortest_path(g, "A", "D")
    assert_equal ["A", "C", "D"], path
  end
end

class TestMinimumSpanningTree < Minitest::Test
  def test_minimum_spanning_tree
    g = Graph.new
    g.add_edge("A", "B", 1.0)
    g.add_edge("B", "C", 2.0)
    g.add_edge("A", "C", 3.0)

    mst = CodingAdventures::Graph.minimum_spanning_tree(g)
    assert_equal 2, mst.length # V - 1 edges

    # Check total weight
    total_weight = mst.sum { |_, _, w| w }
    assert_equal 3.0, total_weight
  end

  def test_minimum_spanning_tree_disconnected
    g = Graph.new
    g.add_edge("A", "B")
    g.add_edge("C", "D")

    assert_raises GraphError do
      CodingAdventures::Graph.minimum_spanning_tree(g)
    end
  end

  def test_minimum_spanning_tree_single_node
    g = Graph.new
    g.add_node("A")

    mst = CodingAdventures::Graph.minimum_spanning_tree(g)
    assert_equal 0, mst.length
  end

  def test_minimum_spanning_tree_empty
    g = Graph.new
    mst = CodingAdventures::Graph.minimum_spanning_tree(g)
    assert_equal 0, mst.length
  end

  def test_minimum_spanning_tree_complex
    g = Graph.new
    g.add_edge("A", "B", 3.0)
    g.add_edge("A", "C", 1.0)
    g.add_edge("B", "D", 4.0)
    g.add_edge("C", "D", 2.0)
    g.add_edge("C", "E", 5.0)
    g.add_edge("D", "E", 1.0)

    mst = CodingAdventures::Graph.minimum_spanning_tree(g)
    assert_equal 4, mst.length # 5 nodes -> 4 edges

    total_weight = mst.sum { |_, _, w| w }
    assert total_weight <= 10
  end
end

class TestBothRepresentations < Minitest::Test
  def test_identical_results
    gl = Graph.new(GraphRepr::ADJACENCY_LIST)
    gm = Graph.new(GraphRepr::ADJACENCY_MATRIX)

    edges = [
      ["A", "B", 1],
      ["B", "C", 2],
      ["C", "A", 3],
      ["D", "E", 4]
    ]

    edges.each do |u, v, w|
      gl.add_edge(u, v, w)
      gm.add_edge(u, v, w)
    end

    # Compare basic properties
    assert_equal gl.length, gm.length
    assert_equal gl.edges.length, gm.edges.length

    # Compare algorithms
    assert_equal CodingAdventures::Graph.bfs(gl, "A").length, CodingAdventures::Graph.bfs(gm, "A").length
    assert_equal CodingAdventures::Graph.dfs(gl, "A").length, CodingAdventures::Graph.dfs(gm, "A").length
    assert_equal CodingAdventures::Graph.is_connected?(gl), CodingAdventures::Graph.is_connected?(gm)
    assert_equal CodingAdventures::Graph.has_cycle?(gl), CodingAdventures::Graph.has_cycle?(gm)
    assert_equal CodingAdventures::Graph.connected_components(gl).length, CodingAdventures::Graph.connected_components(gm).length
  end
end
