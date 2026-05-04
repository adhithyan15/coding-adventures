module NeuralNetwork
  VERSION = "0.1.0"

  Edge = Struct.new(:id, :from, :to, :weight, :properties, keyword_init: true)
  WeightedInput = Struct.new(:from, :weight, :edge_id, :properties, keyword_init: true)

  class Graph
    attr_reader :graph_properties

    def initialize(name = nil)
      @graph_properties = { "nn.version" => "0" }
      @graph_properties["nn.name"] = name if name
      @nodes = []
      @node_properties = {}
      @edges = []
      @next_edge_id = 0
    end

    def add_node(node, properties = {})
      unless @node_properties.key?(node)
        @nodes << node
        @node_properties[node] = {}
      end
      @node_properties[node].merge!(properties || {})
    end

    def nodes
      @nodes.dup
    end

    def edges
      @edges.dup
    end

    def node_properties(node)
      (@node_properties[node] || {}).dup
    end

    def add_edge(from, to, weight = 1.0, properties = {}, edge_id = nil)
      add_node(from)
      add_node(to)
      edge_id ||= begin
        id = "e#{@next_edge_id}"
        @next_edge_id += 1
        id
      end
      props = (properties || {}).merge("weight" => weight)
      @edges << Edge.new(id: edge_id, from: from, to: to, weight: weight, properties: props)
      edge_id
    end

    def incoming_edges(node)
      @edges.select { |edge| edge.to == node }
    end

    def topological_sort
      indegree = {}
      @nodes.each { |node| indegree[node] = 0 }
      @edges.each do |edge|
        indegree[edge.from] ||= 0
        indegree[edge.to] ||= 0
        indegree[edge.to] += 1
      end
      ready = indegree.select { |_node, degree| degree.zero? }.keys.sort
      order = []
      until ready.empty?
        node = ready.shift
        order << node
        released = []
        @edges.select { |edge| edge.from == node }.each do |edge|
          indegree[edge.to] -= 1
          released << edge.to if indegree[edge.to].zero?
        end
        ready.concat(released.sort)
      end
      raise ArgumentError, "neural graph contains a cycle" unless order.length == indegree.length
      order
    end
  end

  class Network
    attr_reader :graph

    def initialize(name = nil)
      @graph = NeuralNetwork.create_neural_graph(name)
    end

    def input(node, input_name = node, properties = {})
      NeuralNetwork.add_input(@graph, node, input_name, properties)
      self
    end

    def constant(node, value, properties = {})
      NeuralNetwork.add_constant(@graph, node, value, properties)
      self
    end

    def weighted_sum(node, inputs, properties = {})
      NeuralNetwork.add_weighted_sum(@graph, node, inputs, properties)
      self
    end

    def activation(node, input, activation, properties = {}, edge_id = nil)
      NeuralNetwork.add_activation(@graph, node, input, activation, properties, edge_id)
      self
    end

    def output(node, input, output_name = node, properties = {}, edge_id = nil)
      NeuralNetwork.add_output(@graph, node, input, output_name, properties, edge_id)
      self
    end
  end

  def self.create_neural_graph(name = nil)
    Graph.new(name)
  end

  def self.create_neural_network(name = nil)
    Network.new(name)
  end

  def self.add_input(graph, node, input_name = node, properties = {})
    graph.add_node(node, properties.merge("nn.op" => "input", "nn.input" => input_name))
  end

  def self.add_constant(graph, node, value, properties = {})
    raise ArgumentError, "constant value must be finite" unless value.finite?
    graph.add_node(node, properties.merge("nn.op" => "constant", "nn.value" => value.to_f))
  end

  def self.add_weighted_sum(graph, node, inputs, properties = {})
    graph.add_node(node, properties.merge("nn.op" => "weighted_sum"))
    inputs.each do |input|
      graph.add_edge(input.from, node, input.weight || 1.0, input.properties || {}, input.edge_id)
    end
  end

  def self.add_activation(graph, node, input, activation, properties = {}, edge_id = nil)
    graph.add_node(node, properties.merge("nn.op" => "activation", "nn.activation" => activation.to_s))
    graph.add_edge(input, node, 1.0, {}, edge_id)
  end

  def self.add_output(graph, node, input, output_name = node, properties = {}, edge_id = nil)
    graph.add_node(node, properties.merge("nn.op" => "output", "nn.output" => output_name))
    graph.add_edge(input, node, 1.0, {}, edge_id)
  end

  def self.create_xor_network(name = "xor")
    create_neural_network(name)
      .input("x0")
      .input("x1")
      .constant("bias", 1.0, "nn.role" => "bias")
      .weighted_sum("h_or_sum", [wi("x0", 20, "x0_to_h_or"), wi("x1", 20, "x1_to_h_or"), wi("bias", -10, "bias_to_h_or")], "nn.layer" => "hidden")
      .activation("h_or", "h_or_sum", "sigmoid", { "nn.layer" => "hidden" }, "h_or_sum_to_h_or")
      .weighted_sum("h_nand_sum", [wi("x0", -20, "x0_to_h_nand"), wi("x1", -20, "x1_to_h_nand"), wi("bias", 30, "bias_to_h_nand")], "nn.layer" => "hidden")
      .activation("h_nand", "h_nand_sum", "sigmoid", { "nn.layer" => "hidden" }, "h_nand_sum_to_h_nand")
      .weighted_sum("out_sum", [wi("h_or", 20, "h_or_to_out"), wi("h_nand", 20, "h_nand_to_out"), wi("bias", -30, "bias_to_out")], "nn.layer" => "output")
      .activation("out_activation", "out_sum", "sigmoid", { "nn.layer" => "output" }, "out_sum_to_activation")
      .output("out", "out_activation", "prediction", { "nn.layer" => "output" }, "activation_to_out")
  end

  def self.wi(from, weight, edge_id)
    WeightedInput.new(from: from, weight: weight.to_f, edge_id: edge_id, properties: {})
  end
end
