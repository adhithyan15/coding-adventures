require "neural_network"

module NeuralGraphVM
  Instruction = Struct.new(:op, :dst, :input_name, :output_name, :edge_id, :value, :left, :right, :inputs, :input, :activation, :source_node, :source_edge, keyword_init: true)
  Function = Struct.new(:id, :kind, :instructions, keyword_init: true)
  GraphEdge = Struct.new(:id, :from, :to, :weight, keyword_init: true)
  BytecodeModule = Struct.new(:magic, :version, :nodes, :edges, :functions, keyword_init: true)

  def self.compile_neural_network_to_bytecode(network)
    compile_neural_graph_to_bytecode(network.graph)
  end

  def self.compile_neural_graph_to_bytecode(graph)
    values = {}
    next_value_id = 0
    alloc = -> { id = "v#{next_value_id}"; next_value_id += 1; id }
    instructions = []

    graph.topological_sort.each do |node|
      props = graph.node_properties(node)
      case props.fetch("nn.op", "weighted_sum")
      when "input"
        dst = alloc.call
        values[node] = dst
        instructions << Instruction.new(op: "LOAD_INPUT", dst: dst, input_name: props.fetch("nn.input", node), source_node: node)
      when "constant"
        dst = alloc.call
        values[node] = dst
        instructions << Instruction.new(op: "LOAD_CONST", dst: dst, value: props.fetch("nn.value"), source_node: node)
      when "weighted_sum"
        terms = []
        graph.incoming_edges(node).sort_by(&:id).each do |edge|
          source = values.fetch(edge.from)
          weight_value = alloc.call
          term_value = alloc.call
          instructions << Instruction.new(op: "LOAD_EDGE_WEIGHT", dst: weight_value, edge_id: edge.id, source_edge: edge.id)
          instructions << Instruction.new(op: "MUL", dst: term_value, left: source, right: weight_value, source_edge: edge.id)
          terms << term_value
        end
        dst = alloc.call
        values[node] = dst
        instructions << Instruction.new(op: terms.empty? ? "LOAD_CONST" : "ADD", dst: dst, value: terms.empty? ? 0.0 : nil, inputs: terms, source_node: node)
      when "activation"
        dst = alloc.call
        values[node] = dst
        instructions << Instruction.new(op: "ACTIVATE", dst: dst, input: single_input_value(graph, values, node), activation: props.fetch("nn.activation", "relu"), source_node: node)
      when "output"
        input = single_input_value(graph, values, node)
        values[node] = input
        instructions << Instruction.new(op: "STORE_OUTPUT", output_name: props.fetch("nn.output", node), input: input, source_node: node)
      else
        raise ArgumentError, "unsupported neural graph op: #{props['nn.op']}"
      end
    end

    BytecodeModule.new(
      magic: "CANN",
      version: 0,
      nodes: graph.nodes,
      edges: graph.edges.map { |edge| GraphEdge.new(id: edge.id, from: edge.from, to: edge.to, weight: edge.weight) },
      functions: [Function.new(id: "forward", kind: "forward", instructions: instructions)]
    )
  end

  def self.run_neural_bytecode_forward(bytecode, inputs)
    values = {}
    edge_weights = {}
    bytecode.edges.each { |edge| edge_weights[edge.id] = edge.weight }
    outputs = {}
    forward = bytecode.functions.find { |fn| fn.kind == "forward" } or raise ArgumentError, "neural bytecode module has no forward function"
    forward.instructions.each do |instruction|
      case instruction.op
      when "LOAD_INPUT" then values[instruction.dst] = inputs.fetch(instruction.input_name)
      when "LOAD_CONST" then values[instruction.dst] = instruction.value || 0.0
      when "LOAD_EDGE_WEIGHT" then values[instruction.dst] = edge_weights.fetch(instruction.edge_id, 1.0)
      when "MUL" then values[instruction.dst] = values.fetch(instruction.left) * values.fetch(instruction.right)
      when "ADD" then values[instruction.dst] = (instruction.inputs || []).sum { |id| values.fetch(id) }
      when "ACTIVATE" then values[instruction.dst] = apply_neural_activation(values.fetch(instruction.input), instruction.activation || "relu")
      when "STORE_OUTPUT" then outputs[instruction.output_name || "output"] = values.fetch(instruction.input)
      else raise ArgumentError, "unsupported opcode: #{instruction.op}"
      end
    end
    outputs
  end

  def self.apply_neural_activation(value, activation)
    case activation
    when "relu" then value.positive? ? value : 0.0
    when "sigmoid" then 1.0 / (1.0 + Math.exp(-value))
    when "tanh" then Math.tanh(value)
    when "none" then value
    else value
    end
  end

  def self.single_input_value(graph, values, node)
    incoming = graph.incoming_edges(node)
    raise ArgumentError, "node #{node} expects exactly one input" unless incoming.length == 1
    values.fetch(incoming.first.from)
  end
end
