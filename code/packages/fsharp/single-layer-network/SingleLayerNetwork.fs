namespace CodingAdventures.SingleLayerNetwork

type ActivationName =
    | Linear
    | Sigmoid

type TrainingStep =
    { Predictions: double[][]
      Errors: double[][]
      WeightGradients: double[][]
      BiasGradients: double[]
      NextWeights: double[][]
      NextBiases: double[]
      Loss: double }

module SingleLayerNetwork =
    let version = "0.1.0"

    let private validateMatrix name (matrix: double[][]) =
        if matrix.Length = 0 then invalidArg name $"{name} must contain at least one row"
        let width = matrix[0].Length
        if width = 0 then invalidArg name $"{name} must contain at least one column"
        if matrix |> Array.exists (fun row -> row.Length <> width) then invalidArg name $"{name} must be rectangular"
        matrix.Length, width

    let private activate activation value =
        match activation with
        | Linear -> value
        | Sigmoid when value >= 0.0 -> 1.0 / (1.0 + exp -value)
        | Sigmoid ->
            let z = exp value
            z / (1.0 + z)

    let private derivativeFromOutput activation output =
        match activation with
        | Linear -> 1.0
        | Sigmoid -> output * (1.0 - output)

    let predictWithParameters inputs weights biases activation =
        let sampleCount, inputCount = validateMatrix "inputs" inputs
        let weightRows, outputCount = validateMatrix "weights" weights
        if inputCount <> weightRows then invalidArg "weights" "input column count must match weight row count"
        if Array.length biases <> outputCount then invalidArg "biases" "bias count must match output count"

        Array.init sampleCount (fun row ->
            Array.init outputCount (fun output ->
                let total =
                    [| 0 .. inputCount - 1 |]
                    |> Array.fold (fun sum input -> sum + inputs[row][input] * weights[input][output]) biases[output]
                activate activation total))

    let trainOneEpochWithMatrices inputs targets weights biases learningRate activation =
        let sampleCount, inputCount = validateMatrix "inputs" inputs
        let targetRows, outputCount = validateMatrix "targets" targets
        let weightRows, weightCols = validateMatrix "weights" weights
        if targetRows <> sampleCount then invalidArg "targets" "inputs and targets must have the same row count"
        if weightRows <> inputCount || weightCols <> outputCount then invalidArg "weights" "weights must be shaped input_count x output_count"
        if Array.length biases <> outputCount then invalidArg "biases" "bias count must match output count"

        let predictions = predictWithParameters inputs weights biases activation
        let scale = 2.0 / float (sampleCount * outputCount)
        let errors = Array.init sampleCount (fun row -> Array.init outputCount (fun output -> predictions.[row].[output] - targets.[row].[output]))
        let deltas = Array.init sampleCount (fun row -> Array.init outputCount (fun output -> scale * errors.[row].[output] * derivativeFromOutput activation (predictions.[row].[output])))
        let loss = errors |> Array.sumBy (Array.sumBy (fun error -> error * error)) |> fun total -> total / float (sampleCount * outputCount)
        let weightGradients =
            Array.init inputCount (fun input ->
                Array.init outputCount (fun output ->
                    [| 0 .. sampleCount - 1 |]
                    |> Array.sumBy (fun row -> inputs.[row].[input] * deltas.[row].[output])))
        let biasGradients =
            Array.init outputCount (fun output ->
                [| 0 .. sampleCount - 1 |]
                |> Array.sumBy (fun row -> deltas.[row].[output]))
        let nextWeights = Array.init inputCount (fun input -> Array.init outputCount (fun output -> weights.[input].[output] - learningRate * weightGradients.[input].[output]))
        let nextBiases = Array.init outputCount (fun output -> biases.[output] - learningRate * biasGradients.[output])
        { Predictions = predictions
          Errors = errors
          WeightGradients = weightGradients
          BiasGradients = biasGradients
          NextWeights = nextWeights
          NextBiases = nextBiases
          Loss = loss }

type Model(inputCount: int, outputCount: int, ?activation: ActivationName) =
    let activation = defaultArg activation Linear
    member val Weights = Array.init inputCount (fun _ -> Array.zeroCreate outputCount) with get, set
    member val Biases = Array.zeroCreate outputCount with get, set
    member _.Activation = activation
    member this.Predict(inputs) = SingleLayerNetwork.predictWithParameters inputs this.Weights this.Biases activation
    member this.Fit(inputs, targets, ?learningRate, ?epochs) =
        let learningRate = defaultArg learningRate 0.05
        let epochs = defaultArg epochs 100
        Array.init epochs (fun _ ->
            let step = SingleLayerNetwork.trainOneEpochWithMatrices inputs targets this.Weights this.Biases learningRate activation
            this.Weights <- step.NextWeights
            this.Biases <- step.NextBiases
            step)
