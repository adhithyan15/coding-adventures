namespace CodingAdventures.FeatureNormalization

open System

type StandardScaler =
    { Means: float list
      StandardDeviations: float list }

type MinMaxScaler =
    { Minimums: float list
      Maximums: float list }

[<RequireQualifiedAccess>]
module FeatureNormalization =
    let private validateMatrix (rows: float list list) =
        if List.isEmpty rows || List.isEmpty rows.Head then
            invalidArg (nameof rows) "Matrix must have at least one row and one column."

        let width = rows.Head.Length
        if rows |> List.exists (fun row -> row.Length <> width) then
            invalidArg (nameof rows) "All rows must have the same number of columns."

        width

    let fitStandardScaler (rows: float list list) =
        let width = validateMatrix rows
        let count = float rows.Length

        let means =
            [ for col in 0 .. width - 1 ->
                rows |> List.sumBy (fun row -> row.[col]) |> fun sum -> sum / count ]

        let standardDeviations =
            [ for col in 0 .. width - 1 ->
                rows
                |> List.sumBy (fun row ->
                    let diff = row.[col] - means.[col]
                    diff * diff)
                |> fun variance -> Math.Sqrt(variance / count) ]

        { Means = means
          StandardDeviations = standardDeviations }

    let transformStandard (rows: float list list) (scaler: StandardScaler) =
        let width = validateMatrix rows
        if width <> scaler.Means.Length || width <> scaler.StandardDeviations.Length then
            invalidArg (nameof scaler) "Matrix width must match scaler width."

        rows
        |> List.map (fun row ->
            [ for col in 0 .. width - 1 ->
                if scaler.StandardDeviations.[col] = 0.0 then
                    0.0
                else
                    (row.[col] - scaler.Means.[col]) / scaler.StandardDeviations.[col] ])

    let fitMinMaxScaler (rows: float list list) =
        let width = validateMatrix rows
        { Minimums = [ for col in 0 .. width - 1 -> rows |> List.map (fun row -> row.[col]) |> List.min ]
          Maximums = [ for col in 0 .. width - 1 -> rows |> List.map (fun row -> row.[col]) |> List.max ] }

    let transformMinMax (rows: float list list) (scaler: MinMaxScaler) =
        let width = validateMatrix rows
        if width <> scaler.Minimums.Length || width <> scaler.Maximums.Length then
            invalidArg (nameof scaler) "Matrix width must match scaler width."

        rows
        |> List.map (fun row ->
            [ for col in 0 .. width - 1 ->
                let span = scaler.Maximums.[col] - scaler.Minimums.[col]
                if span = 0.0 then 0.0 else (row.[col] - scaler.Minimums.[col]) / span ])
