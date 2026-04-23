namespace CodingAdventures.JsonSerializer.FSharp

open System
open System.Globalization
open System.Text
open CodingAdventures.JsonValue.FSharp

type JsonSerializerError(message: string) =
    inherit Exception(message)

type SerializerConfig(?indentSize: int, ?indentChar: char, ?sortKeys: bool, ?trailingNewline: bool) =
    member _.IndentSize = defaultArg indentSize 2
    member _.IndentChar = defaultArg indentChar ' '
    member _.SortKeys = defaultArg sortKeys false
    member _.TrailingNewline = defaultArg trailingNewline false

[<AbstractClass; Sealed>]
type JsonSerializer private () =
    static member Serialize(value: JsonValue) =
        match value with
        | :? JsonNullValue -> "null"
        | :? JsonBooleanValue as booleanValue -> if booleanValue.Value then "true" else "false"
        | :? JsonNumberValue as numberValue -> JsonSerializer.SerializeNumber(numberValue)
        | :? JsonStringValue as stringValue -> JsonSerializer.SerializeString(stringValue.Value)
        | :? JsonArrayValue as arrayValue ->
            arrayValue.Elements |> Seq.map JsonSerializer.Serialize |> String.concat "," |> sprintf "[%s]"
        | :? JsonObjectValue as objectValue ->
            objectValue.Pairs
            |> Seq.map (fun pair -> sprintf "%s:%s" (JsonSerializer.SerializeString(pair.Key)) (JsonSerializer.Serialize(pair.Value)))
            |> String.concat ","
            |> sprintf "{%s}"
        | _ ->
            raise (JsonSerializerError(sprintf "Unsupported JsonValue node '%s'." (value.GetType().Name)))

    static member SerializePretty(value: JsonValue, ?config: SerializerConfig) =
        let resolved = defaultArg config (SerializerConfig())
        let text = JsonSerializer.SerializePrettyRecursive(value, resolved, 0)
        if resolved.TrailingNewline then text + "\n" else text

    static member Stringify(value: obj) =
        JsonValue.FromNative(value) |> JsonSerializer.Serialize

    static member StringifyPretty(value: obj, ?config: SerializerConfig) =
        JsonValue.FromNative(value) |> fun jsonValue -> JsonSerializer.SerializePretty(jsonValue, ?config = config)

    static member private SerializePrettyRecursive(value: JsonValue, config: SerializerConfig, depth: int) =
        let indentUnit = String.replicate config.IndentSize (string config.IndentChar)
        let currentIndent = String.Concat(Seq.replicate depth indentUnit)
        let nextIndent = String.Concat(Seq.replicate (depth + 1) indentUnit)

        match value with
        | :? JsonNullValue -> "null"
        | :? JsonBooleanValue as booleanValue -> if booleanValue.Value then "true" else "false"
        | :? JsonNumberValue as numberValue -> JsonSerializer.SerializeNumber(numberValue)
        | :? JsonStringValue as stringValue -> JsonSerializer.SerializeString(stringValue.Value)
        | :? JsonArrayValue as arrayValue ->
            if arrayValue.Elements.Count = 0 then
                "[]"
            else
                let lines =
                    arrayValue.Elements
                    |> Seq.map (fun element -> nextIndent + JsonSerializer.SerializePrettyRecursive(element, config, depth + 1))
                    |> String.concat ",\n"

                sprintf "[\n%s\n%s]" lines currentIndent
        | :? JsonObjectValue as objectValue ->
            if objectValue.Pairs.Count = 0 then
                "{}"
            else
                let pairs =
                    let basePairs = objectValue.Pairs |> Seq.toList
                    if config.SortKeys then
                        basePairs |> List.sortBy (fun pair -> pair.Key)
                    else
                        basePairs

                let lines =
                    pairs
                    |> Seq.map (fun pair ->
                        sprintf "%s%s: %s"
                            nextIndent
                            (JsonSerializer.SerializeString(pair.Key))
                            (JsonSerializer.SerializePrettyRecursive(pair.Value, config, depth + 1)))
                    |> String.concat ",\n"

                sprintf "{\n%s\n%s}" lines currentIndent
        | _ ->
            raise (JsonSerializerError(sprintf "Unsupported JsonValue node '%s'." (value.GetType().Name)))

    static member private SerializeNumber(numberValue: JsonNumberValue) =
        if Double.IsNaN(numberValue.Value) || Double.IsInfinity(numberValue.Value) then
            raise (JsonSerializerError("JSON does not support NaN or infinity."))

        if numberValue.IsInteger
           && numberValue.Value <= double Int64.MaxValue
           && numberValue.Value >= double Int64.MinValue
           && abs (numberValue.Value % 1.0) < Double.Epsilon then
            Convert.ToInt64(numberValue.Value, CultureInfo.InvariantCulture).ToString(CultureInfo.InvariantCulture)
        else
            numberValue.Value.ToString("R", CultureInfo.InvariantCulture)

    static member private SerializeString(value: string) =
        let builder = StringBuilder()
        builder.Append('"') |> ignore

        for ch in value do
            match ch with
            | '"' -> builder.Append("\\\"") |> ignore
            | '\\' -> builder.Append("\\\\") |> ignore
            | '\b' -> builder.Append("\\b") |> ignore
            | '\f' -> builder.Append("\\f") |> ignore
            | '\n' -> builder.Append("\\n") |> ignore
            | '\r' -> builder.Append("\\r") |> ignore
            | '\t' -> builder.Append("\\t") |> ignore
            | _ when Char.IsControl(ch) ->
                builder.Append("\\u") |> ignore
                builder.Append((int ch).ToString("x4", CultureInfo.InvariantCulture)) |> ignore
            | _ ->
                builder.Append(ch) |> ignore

        builder.Append('"') |> ignore
        builder.ToString()
