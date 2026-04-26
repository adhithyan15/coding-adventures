namespace CodingAdventures.Perceptron.FSharp

open System
open CodingAdventures.ActivationFunctions
open CodingAdventures.LossFunctions

[<RequireQualifiedAccess>]
module private Validation =
    let ensureFinite name value =
        if Double.IsNaN(value) || Double.IsInfinity(value) then
            invalidArg name "Values must be finite."

    let validateFeatures (features: float array array) =
        if isNull features then nullArg "features"
        if features.Length = 0 then invalidArg "features" "Training data must contain at least one sample."
        if isNull features.[0] || features.[0].Length = 0 then
            invalidArg "features" "Samples must contain at least one feature."

        let expectedColumns = features.[0].Length

        features
        |> Array.mapi (fun row values ->
            if isNull values then nullArg $"features[{row}]"
            if values.Length <> expectedColumns then
                invalidArg "features" $"Sample {row} has {values.Length} features, expected {expectedColumns}."

            values
            |> Array.map (fun value ->
                ensureFinite "features" value
                value))

    let validateLabels (labels: float array) expectedRows =
        if isNull labels then nullArg "labels"
        if labels.Length <> expectedRows || labels.Length = 0 then
            invalidArg "labels" "Labels must match the non-zero sample count."

        labels
        |> Array.map (fun value ->
            ensureFinite "labels" value
            value)

    let flattenLabels (labels: float array array) =
        if isNull labels then nullArg "labels"

        labels
        |> Array.mapi (fun row values ->
            if isNull values then nullArg $"labels[{row}]"
            if values.Length <> 1 then
                invalidArg "labels" "Column labels must have exactly one value per row."
            values.[0])

type Perceptron(learningRate: float, epochs: int) =
    let mutable weights: float array option = None
    let mutable bias = 0.0

    do
        Validation.ensureFinite "learningRate" learningRate
        if epochs < 0 then invalidArg "epochs" "Epochs must be non-negative."

    new() = Perceptron(0.1, 2000)

    new(learningRate: float) = Perceptron(learningRate, 2000)

    member _.LearningRate = learningRate

    member _.Epochs = epochs

    member _.Bias = bias

    member _.Weights = weights |> Option.map Array.copy

    member this.Fit(features: float array array, labels: float array) =
        this.Fit(features, labels, 0)

    member this.Fit(features: float array array, labels: float array array) =
        this.Fit(features, Validation.flattenLabels labels, 0)

    member _.Fit(features: float array array, labels: float array, logSteps: int) =
        let x = Validation.validateFeatures features
        let y = Validation.validateLabels labels x.Length
        let featureCount = x.[0].Length
        let currentWeights = Array.zeroCreate<float> featureCount
        weights <- Some currentWeights
        bias <- 0.0

        for epoch in 0 .. epochs do
            let rawScores =
                x
                |> Array.map (fun row ->
                    let weighted =
                        row
                        |> Array.mapi (fun index value -> value * currentWeights.[index])
                        |> Array.sum

                    weighted + bias)

            let predictions = rawScores |> Array.map ActivationFunctions.sigmoid
            let lossGradient = LossFunctions.bceDerivative y predictions
            let weightGradient = Array.zeroCreate<float> featureCount
            let mutable biasGradient = 0.0

            for row in 0 .. x.Length - 1 do
                let combined = lossGradient.[row] * ActivationFunctions.sigmoidDerivative rawScores.[row]
                for col in 0 .. featureCount - 1 do
                    weightGradient.[col] <- weightGradient.[col] + (x.[row].[col] * combined)

                biasGradient <- biasGradient + combined

            for col in 0 .. featureCount - 1 do
                currentWeights.[col] <- currentWeights.[col] - (learningRate * weightGradient.[col])

            bias <- bias - (learningRate * biasGradient)

            if logSteps > 0 && epoch % logSteps = 0 then
                let loss = LossFunctions.bce y predictions
                printfn "Epoch %4d | BCE Loss: %.4f | Bias: %.2f" epoch loss bias

        weights <- Some(Array.copy currentWeights)

    member this.Fit(features: float array array, labels: float array array, logSteps: int) =
        this.Fit(features, Validation.flattenLabels labels, logSteps)

    member _.Predict(features: float array array) =
        match weights with
        | None -> invalidOp "Perceptron has not been trained yet. Call Fit first."
        | Some currentWeights ->
            let x = Validation.validateFeatures features
            if x.[0].Length <> currentWeights.Length then
                invalidArg "features" $"Feature width {x.[0].Length} does not match trained width {currentWeights.Length}."

            x
            |> Array.map (fun row ->
                let raw =
                    row
                    |> Array.mapi (fun index value -> value * currentWeights.[index])
                    |> Array.sum
                    |> fun value -> value + bias

                ActivationFunctions.sigmoid raw)
