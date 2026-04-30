defmodule NeuralGraphVM do
  def compile_neural_network_to_bytecode(network), do: compile_neural_graph_to_bytecode(network.graph)

  def compile_neural_graph_to_bytecode(graph) do
    {:ok, order} = NeuralNetwork.Graph.topological_sort(graph)
    {instructions, _values, _next} = Enum.reduce(order, {[], %{}, 0}, fn node, {instructions, values, next_value_id} ->
      props = NeuralNetwork.Graph.node_properties(graph, node)
      case Map.get(props, "nn.op", "weighted_sum") do
        "input" ->
          {dst, next_value_id} = alloc(next_value_id)
          {instructions ++ [%{op: "LOAD_INPUT", dst: dst, input_name: Map.get(props, "nn.input", node), source_node: node}], Map.put(values, node, dst), next_value_id}
        "constant" ->
          {dst, next_value_id} = alloc(next_value_id)
          {instructions ++ [%{op: "LOAD_CONST", dst: dst, value: Map.fetch!(props, "nn.value"), source_node: node}], Map.put(values, node, dst), next_value_id}
        "weighted_sum" ->
          {term_instructions, terms, next_value_id} =
            graph
            |> NeuralNetwork.Graph.incoming_edges(node)
            |> Enum.sort_by(& &1.id)
            |> Enum.reduce({[], [], next_value_id}, fn edge, {insts, terms, next_id} ->
              {weight_value, next_id} = alloc(next_id)
              {term_value, next_id} = alloc(next_id)
              {insts ++ [%{op: "LOAD_EDGE_WEIGHT", dst: weight_value, edge_id: edge.id, source_edge: edge.id}, %{op: "MUL", dst: term_value, left: Map.fetch!(values, edge.from), right: weight_value, source_edge: edge.id}], terms ++ [term_value], next_id}
            end)
          {dst, next_value_id} = alloc(next_value_id)
          add = if terms == [], do: %{op: "LOAD_CONST", dst: dst, value: 0.0, source_node: node}, else: %{op: "ADD", dst: dst, inputs: terms, source_node: node}
          {instructions ++ term_instructions ++ [add], Map.put(values, node, dst), next_value_id}
        "activation" ->
          {dst, next_value_id} = alloc(next_value_id)
          input = single_input_value(graph, values, node)
          {instructions ++ [%{op: "ACTIVATE", dst: dst, input: input, activation: Map.get(props, "nn.activation", "relu"), source_node: node}], Map.put(values, node, dst), next_value_id}
        "output" ->
          input = single_input_value(graph, values, node)
          {instructions ++ [%{op: "STORE_OUTPUT", output_name: Map.get(props, "nn.output", node), input: input, source_node: node}], Map.put(values, node, input), next_value_id}
      end
    end)

    %{magic: "CANN", version: 0, nodes: graph.nodes, edges: Enum.map(graph.edges, &%{id: &1.id, from: &1.from, to: &1.to, weight: &1.weight}), functions: [%{id: "forward", kind: "forward", instructions: instructions}]}
  end

  def run_neural_bytecode_forward(module, inputs) do
    edge_weights = Map.new(module.edges, &{&1.id, &1.weight})
    forward = Enum.find(module.functions, &(&1.kind == "forward"))
    {outputs, _values} = Enum.reduce(forward.instructions, {%{}, %{}}, fn inst, {outputs, values} ->
      case inst.op do
        "LOAD_INPUT" -> {outputs, Map.put(values, inst.dst, Map.fetch!(inputs, inst.input_name))}
        "LOAD_CONST" -> {outputs, Map.put(values, inst.dst, Map.get(inst, :value, 0.0))}
        "LOAD_EDGE_WEIGHT" -> {outputs, Map.put(values, inst.dst, Map.get(edge_weights, inst.edge_id, 1.0))}
        "MUL" -> {outputs, Map.put(values, inst.dst, Map.fetch!(values, inst.left) * Map.fetch!(values, inst.right))}
        "ADD" -> {outputs, Map.put(values, inst.dst, Enum.reduce(Map.get(inst, :inputs, []), 0.0, &(&2 + Map.fetch!(values, &1))))}
        "ACTIVATE" -> {outputs, Map.put(values, inst.dst, apply_neural_activation(Map.fetch!(values, inst.input), Map.get(inst, :activation, "relu")))}
        "STORE_OUTPUT" -> {Map.put(outputs, Map.get(inst, :output_name, "output"), Map.fetch!(values, inst.input)), values}
      end
    end)
    outputs
  end

  def apply_neural_activation(value, "relu"), do: if(value > 0, do: value, else: 0.0)
  def apply_neural_activation(value, "sigmoid"), do: 1.0 / (1.0 + :math.exp(-value))
  def apply_neural_activation(value, "tanh"), do: :math.tanh(value)
  def apply_neural_activation(value, _), do: value

  defp alloc(next_value_id), do: {"v#{next_value_id}", next_value_id + 1}
  defp single_input_value(graph, values, node) do
    [edge] = NeuralNetwork.Graph.incoming_edges(graph, node)
    Map.fetch!(values, edge.from)
  end
end
