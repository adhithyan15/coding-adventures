namespace CodingAdventures.NeuralNetwork.Tests

open Xunit
open CodingAdventures.NeuralNetwork

module NeuralNetworkTests =
    [<Fact>]
    let ``builds tiny weighted graph`` () =
        let graph =
            NeuralNetwork.createNeuralGraph (Some "tiny")
            |> fun graph -> NeuralNetwork.addInput graph "x0" "x0" Map.empty
            |> fun graph -> NeuralNetwork.addInput graph "x1" "x1" Map.empty
            |> fun graph -> NeuralNetwork.addConstant graph "bias" 1.0 Map.empty
            |> fun graph -> NeuralNetwork.addWeightedSum graph "sum" [ NeuralNetwork.wi "x0" 0.25 "x0_to_sum"; NeuralNetwork.wi "x1" 0.75 "x1_to_sum"; NeuralNetwork.wi "bias" -1.0 "bias_to_sum" ] Map.empty
            |> fun graph -> NeuralNetwork.addActivation graph "relu" "sum" "relu" Map.empty (Some "sum_to_relu")
            |> fun graph -> NeuralNetwork.addOutput graph "out" "relu" "prediction" Map.empty (Some "relu_to_out")
        Assert.Equal(3, NeuralGraph.incomingEdges "sum" graph |> List.length)
        match NeuralGraph.topologicalSort graph with
        | Ok order -> Assert.Equal("out", List.last order)
        | Error err -> failwith err

    [<Fact>]
    let ``xor network has hidden output edge`` () =
        let model = NeuralNetwork.createXorNetwork "xor"
        Assert.Contains(model.Graph.Edges, fun edge -> edge.Id = "h_or_to_out")
