namespace CodingAdventures.NeuralGraphVM.Tests

open Xunit
open CodingAdventures.NeuralNetwork
open CodingAdventures.NeuralGraphVM

module NeuralGraphVMTests =
    let tinyGraph () =
        NeuralNetwork.createNeuralGraph (Some "tiny")
        |> fun graph -> NeuralNetwork.addInput graph "x0" "x0" Map.empty
        |> fun graph -> NeuralNetwork.addInput graph "x1" "x1" Map.empty
        |> fun graph -> NeuralNetwork.addConstant graph "bias" 1.0 Map.empty
        |> fun graph -> NeuralNetwork.addWeightedSum graph "sum" [ NeuralNetwork.wi "x0" 0.25 "x0_to_sum"; NeuralNetwork.wi "x1" 0.75 "x1_to_sum"; NeuralNetwork.wi "bias" -1.0 "bias_to_sum" ] Map.empty
        |> fun graph -> NeuralNetwork.addActivation graph "relu" "sum" "relu" Map.empty (Some "sum_to_relu")
        |> fun graph -> NeuralNetwork.addOutput graph "out" "relu" "prediction" Map.empty (Some "relu_to_out")

    [<Fact>]
    let ``runs tiny weighted sum`` () =
        match tinyGraph () |> NeuralGraphVM.compileNeuralGraphToBytecode with
        | Error err -> failwith err
        | Ok bytecode ->
            let outputs = NeuralGraphVM.runNeuralBytecodeForward bytecode (Map.ofList [ "x0", 4.0; "x1", 8.0 ])
            Assert.Equal(6.0, outputs["prediction"], 9)

    [<Fact>]
    let ``runs xor`` () =
        let bytecode = NeuralNetwork.createXorNetwork "xor" |> NeuralGraphVM.compileNeuralNetworkToBytecode |> Result.defaultWith failwith
        for x0, x1, expected in [ 0.0, 0.0, 0.0; 0.0, 1.0, 1.0; 1.0, 0.0, 1.0; 1.0, 1.0, 0.0 ] do
            let prediction = (NeuralGraphVM.runNeuralBytecodeForward bytecode (Map.ofList [ "x0", x0; "x1", x1 ])).["prediction"]
            Assert.True(if expected = 1.0 then prediction > 0.99 else prediction < 0.01)
