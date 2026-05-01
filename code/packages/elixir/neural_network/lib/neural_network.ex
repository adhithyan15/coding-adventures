defmodule NeuralNetwork.Edge do
  defstruct [:id, :from, :to, :weight, properties: %{}]
end

defmodule NeuralNetwork.WeightedInput do
  defstruct [:from, weight: 1.0, edge_id: nil, properties: %{}]
end

defmodule NeuralNetwork.Graph do
  defstruct graph_properties: %{"nn.version" => "0"},
            nodes: [],
            node_properties: %{},
            edges: [],
            next_edge_id: 0

  def new(name \\ nil) do
    graph = %__MODULE__{}

    if name == nil,
      do: graph,
      else: %{graph | graph_properties: Map.put(graph.graph_properties, "nn.name", name)}
  end

  def add_node(graph, node, properties \\ %{}) do
    {nodes, node_properties} =
      if Map.has_key?(graph.node_properties, node) do
        {graph.nodes, graph.node_properties}
      else
        {graph.nodes ++ [node], Map.put(graph.node_properties, node, %{})}
      end

    %{
      graph
      | nodes: nodes,
        node_properties: Map.update!(node_properties, node, &Map.merge(&1, properties))
    }
  end

  def node_properties(graph, node), do: Map.get(graph.node_properties, node, %{})

  def add_edge(graph, from, to, weight \\ 1.0, properties \\ %{}, edge_id \\ nil) do
    graph = graph |> add_node(from) |> add_node(to)

    {id, next_edge_id} =
      if edge_id == nil,
        do: {"e#{graph.next_edge_id}", graph.next_edge_id + 1},
        else: {edge_id, graph.next_edge_id}

    edge = %NeuralNetwork.Edge{
      id: id,
      from: from,
      to: to,
      weight: weight / 1,
      properties: Map.put(properties, "weight", weight / 1)
    }

    {%{graph | edges: graph.edges ++ [edge], next_edge_id: next_edge_id}, id}
  end

  def incoming_edges(graph, node), do: Enum.filter(graph.edges, &(&1.to == node))

  def topological_sort(graph) do
    indegree = Enum.reduce(graph.nodes, %{}, &Map.put(&2, &1, 0))

    indegree =
      Enum.reduce(graph.edges, indegree, fn edge, acc ->
        acc |> Map.put_new(edge.from, 0) |> Map.update(edge.to, 1, &(&1 + 1))
      end)

    ready =
      indegree
      |> Enum.filter(fn {_node, degree} -> degree == 0 end)
      |> Enum.map(&elem(&1, 0))
      |> Enum.sort()

    do_topological_sort(graph.edges, indegree, ready, [])
  end

  defp do_topological_sort(_edges, indegree, [], order) do
    if length(order) == map_size(indegree),
      do: {:ok, order},
      else: {:error, "neural graph contains a cycle"}
  end

  defp do_topological_sort(edges, indegree, [node | rest], order) do
    {indegree, released} =
      edges
      |> Enum.filter(&(&1.from == node))
      |> Enum.reduce({indegree, []}, fn edge, {degrees, released} ->
        next = Map.update!(degrees, edge.to, &(&1 - 1))
        if next[edge.to] == 0, do: {next, [edge.to | released]}, else: {next, released}
      end)

    do_topological_sort(edges, indegree, rest ++ Enum.sort(released), order ++ [node])
  end
end

defmodule NeuralNetwork.Network do
  defstruct [:graph]
  def new(name \\ nil), do: %__MODULE__{graph: NeuralNetwork.create_neural_graph(name)}

  def input(network, node, input_name \\ nil, properties \\ %{}),
    do: %{
      network
      | graph: NeuralNetwork.add_input(network.graph, node, input_name || node, properties)
    }

  def constant(network, node, value, properties \\ %{}),
    do: %{network | graph: NeuralNetwork.add_constant(network.graph, node, value, properties)}

  def weighted_sum(network, node, inputs, properties \\ %{}),
    do: %{
      network
      | graph: NeuralNetwork.add_weighted_sum(network.graph, node, inputs, properties)
    }

  def activation(network, node, input, activation, properties \\ %{}, edge_id \\ nil),
    do: %{
      network
      | graph:
          NeuralNetwork.add_activation(
            network.graph,
            node,
            input,
            activation,
            properties,
            edge_id
          )
          |> elem(0)
    }

  def output(network, node, input, output_name \\ nil, properties \\ %{}, edge_id \\ nil),
    do: %{
      network
      | graph:
          NeuralNetwork.add_output(
            network.graph,
            node,
            input,
            output_name || node,
            properties,
            edge_id
          )
          |> elem(0)
    }
end

defmodule NeuralNetwork do
  alias NeuralNetwork.{Graph, Network, WeightedInput}
  def create_neural_graph(name \\ nil), do: Graph.new(name)
  def create_neural_network(name \\ nil), do: Network.new(name)

  def wi(from, weight, edge_id),
    do: %WeightedInput{from: from, weight: weight / 1, edge_id: edge_id, properties: %{}}

  def add_input(graph, node, input_name \\ nil, properties \\ %{}),
    do:
      Graph.add_node(
        graph,
        node,
        Map.merge(properties, %{"nn.op" => "input", "nn.input" => input_name || node})
      )

  def add_constant(graph, node, value, properties \\ %{}),
    do:
      Graph.add_node(
        graph,
        node,
        Map.merge(properties, %{"nn.op" => "constant", "nn.value" => value / 1})
      )

  def add_weighted_sum(graph, node, inputs, properties \\ %{}) do
    graph = Graph.add_node(graph, node, Map.merge(properties, %{"nn.op" => "weighted_sum"}))

    Enum.reduce(inputs, graph, fn input, acc ->
      Graph.add_edge(acc, input.from, node, input.weight, input.properties, input.edge_id)
      |> elem(0)
    end)
  end

  def add_activation(graph, node, input, activation, properties \\ %{}, edge_id \\ nil) do
    graph =
      Graph.add_node(
        graph,
        node,
        Map.merge(properties, %{"nn.op" => "activation", "nn.activation" => activation})
      )

    Graph.add_edge(graph, input, node, 1.0, %{}, edge_id)
  end

  def add_output(graph, node, input, output_name \\ nil, properties \\ %{}, edge_id \\ nil) do
    graph =
      Graph.add_node(
        graph,
        node,
        Map.merge(properties, %{"nn.op" => "output", "nn.output" => output_name || node})
      )

    Graph.add_edge(graph, input, node, 1.0, %{}, edge_id)
  end

  def create_xor_network(name \\ "xor") do
    create_neural_network(name)
    |> Network.input("x0")
    |> Network.input("x1")
    |> Network.constant("bias", 1.0, %{"nn.role" => "bias"})
    |> Network.weighted_sum(
      "h_or_sum",
      [wi("x0", 20, "x0_to_h_or"), wi("x1", 20, "x1_to_h_or"), wi("bias", -10, "bias_to_h_or")],
      %{"nn.layer" => "hidden"}
    )
    |> Network.activation(
      "h_or",
      "h_or_sum",
      "sigmoid",
      %{"nn.layer" => "hidden"},
      "h_or_sum_to_h_or"
    )
    |> Network.weighted_sum(
      "h_nand_sum",
      [
        wi("x0", -20, "x0_to_h_nand"),
        wi("x1", -20, "x1_to_h_nand"),
        wi("bias", 30, "bias_to_h_nand")
      ],
      %{"nn.layer" => "hidden"}
    )
    |> Network.activation(
      "h_nand",
      "h_nand_sum",
      "sigmoid",
      %{"nn.layer" => "hidden"},
      "h_nand_sum_to_h_nand"
    )
    |> Network.weighted_sum(
      "out_sum",
      [
        wi("h_or", 20, "h_or_to_out"),
        wi("h_nand", 20, "h_nand_to_out"),
        wi("bias", -30, "bias_to_out")
      ],
      %{"nn.layer" => "output"}
    )
    |> Network.activation(
      "out_activation",
      "out_sum",
      "sigmoid",
      %{"nn.layer" => "output"},
      "out_sum_to_activation"
    )
    |> Network.output(
      "out",
      "out_activation",
      "prediction",
      %{"nn.layer" => "output"},
      "activation_to_out"
    )
  end
end
