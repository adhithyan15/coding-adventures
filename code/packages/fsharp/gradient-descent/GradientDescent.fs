namespace CodingAdventures.GradientDescent

open System

type GradientDescent private () =
    static member Sgd(weights: float list, gradients: float list, learningRate: float) =
        if isNull (box weights) then
            nullArg (nameof weights)

        if isNull (box gradients) then
            nullArg (nameof gradients)

        if List.isEmpty weights || weights.Length <> gradients.Length then
            invalidArg (nameof gradients) "Weights and gradients must have the same non-zero length."

        List.map2 (fun weight gradient -> weight - learningRate * gradient) weights gradients
