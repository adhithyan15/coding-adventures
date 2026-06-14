namespace CodingAdventures.TwoLayerNetwork

type ActivationName =
    | Linear
    | Sigmoid

type Parameters =
    { InputToHiddenWeights: double[][]
      HiddenBiases: double[]
      HiddenToOutputWeights: double[][]
      OutputBiases: double[] }

type ForwardPass =
    { HiddenRaw: double[][]
      HiddenActivations: double[][]
      OutputRaw: double[][]
      Predictions: double[][] }

type TrainingStep =
    { Predictions: double[][]
      Errors: double[][]
      OutputDeltas: double[][]
      HiddenDeltas: double[][]
      HiddenToOutputWeightGradients: double[][]
      OutputBiasGradients: double[]
      InputToHiddenWeightGradients: double[][]
      HiddenBiasGradients: double[]
      NextParameters: Parameters
      Loss: double }

module TwoLayerNetwork =
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

    let private derivative activation _raw activated =
        match activation with
        | Linear -> 1.0
        | Sigmoid -> activated * (1.0 - activated)

    let private dot (left: double[][]) (right: double[][]) =
        let rows, width = validateMatrix "left" left
        let rightRows, cols = validateMatrix "right" right
        if width <> rightRows then invalidArg "right" "matrix shapes do not align"
        Array.init rows (fun row ->
            Array.init cols (fun col ->
                [| 0 .. width - 1 |] |> Array.sumBy (fun k -> left[row][k] * right[k][col])))

    let private transpose (matrix: double[][]) =
        let rows, cols = validateMatrix "matrix" matrix
        Array.init cols (fun col -> Array.init rows (fun row -> matrix[row][col]))

    let private addBiases (matrix: double[][]) (biases: double[]) =
        matrix |> Array.map (fun row -> row |> Array.mapi (fun col value -> value + biases[col]))

    let private applyActivation activation (matrix: double[][]) =
        matrix |> Array.map (Array.map (activate activation))

    let private columnSums (matrix: double[][]) =
        let _, cols = validateMatrix "matrix" matrix
        Array.init cols (fun col -> matrix |> Array.sumBy (fun row -> row[col]))

    let private subtractScaled (matrix: double[][]) (gradients: double[][]) learningRate =
        matrix |> Array.mapi (fun rowIndex row -> row |> Array.mapi (fun col value -> value - learningRate * gradients[rowIndex][col]))

    let private subtractScaledVector (values: double[]) (gradients: double[]) learningRate =
        values |> Array.mapi (fun index value -> value - learningRate * gradients[index])

    let private mse errors =
        errors |> Array.sumBy (Array.sumBy (fun value -> value * value)) |> fun total -> total / float (errors |> Array.sumBy Array.length)

    let xorWarmStartParameters () =
        { InputToHiddenWeights = [| [| 4.0; -4.0 |]; [| 4.0; -4.0 |] |]
          HiddenBiases = [| -2.0; 6.0 |]
          HiddenToOutputWeights = [| [| 4.0 |]; [| 4.0 |] |]
          OutputBiases = [| -6.0 |] }

    let forward (inputs: double[][]) (parameters: Parameters) hiddenActivation outputActivation =
        let hiddenRaw = addBiases (dot inputs (parameters.InputToHiddenWeights)) (parameters.HiddenBiases)
        let hiddenActivations = applyActivation hiddenActivation hiddenRaw
        let outputRaw = addBiases (dot hiddenActivations (parameters.HiddenToOutputWeights)) (parameters.OutputBiases)
        let predictions = applyActivation outputActivation outputRaw
        { HiddenRaw = hiddenRaw; HiddenActivations = hiddenActivations; OutputRaw = outputRaw; Predictions = predictions }

    let trainOneEpoch (inputs: double[][]) (targets: double[][]) (parameters: Parameters) learningRate hiddenActivation outputActivation =
        let sampleCount, _ = validateMatrix "inputs" inputs
        let _, outputCount = validateMatrix "targets" targets
        let passed = forward inputs parameters hiddenActivation outputActivation
        let scale = 2.0 / float (sampleCount * outputCount)
        let errors = Array.init sampleCount (fun row -> Array.init outputCount (fun output -> passed.Predictions.[row].[output] - targets.[row].[output]))
        let outputDeltas = Array.init sampleCount (fun row -> Array.init outputCount (fun output -> scale * errors.[row].[output] * derivative outputActivation passed.OutputRaw.[row].[output] passed.Predictions.[row].[output]))
        let h2oGradients = dot (transpose passed.HiddenActivations) outputDeltas
        let outputBiasGradients = columnSums outputDeltas
        let hiddenErrors = dot outputDeltas (transpose parameters.HiddenToOutputWeights)
        let hiddenWidth = parameters.HiddenBiases.Length
        let hiddenDeltas = Array.init sampleCount (fun row -> Array.init hiddenWidth (fun hidden -> hiddenErrors.[row].[hidden] * derivative hiddenActivation passed.HiddenRaw.[row].[hidden] passed.HiddenActivations.[row].[hidden]))
        let i2hGradients = dot (transpose inputs) hiddenDeltas
        let hiddenBiasGradients = columnSums hiddenDeltas
        { Predictions = passed.Predictions
          Errors = errors
          OutputDeltas = outputDeltas
          HiddenDeltas = hiddenDeltas
          HiddenToOutputWeightGradients = h2oGradients
          OutputBiasGradients = outputBiasGradients
          InputToHiddenWeightGradients = i2hGradients
          HiddenBiasGradients = hiddenBiasGradients
          NextParameters =
            { InputToHiddenWeights = subtractScaled (parameters.InputToHiddenWeights) i2hGradients learningRate
              HiddenBiases = subtractScaledVector (parameters.HiddenBiases) hiddenBiasGradients learningRate
              HiddenToOutputWeights = subtractScaled (parameters.HiddenToOutputWeights) h2oGradients learningRate
              OutputBiases = subtractScaledVector (parameters.OutputBiases) outputBiasGradients learningRate }
          Loss = mse errors }
