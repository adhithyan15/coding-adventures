namespace CodingAdventures.NibWasmCompiler.FSharp

open System
open System.Collections.Generic
open System.IO
open System.Text.RegularExpressions
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmModuleEncoder.FSharp
open CodingAdventures.WasmTypes.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type PackageError(stage: string, message: string) =
    inherit Exception(message)
    member _.Stage = stage
    override _.ToString() = $"[{stage}] {message}"

type NibFunction(name: string, parameters: string list, expression: string) =
    member _.Name = name
    member _.Parameters = parameters
    member _.Expression = expression.Trim()

type PackageResult(source: string, functions: NibFunction list, wasmBytes: byte array, wasmPath: string option) =
    let copiedBytes = Array.copy wasmBytes

    member _.Source = source
    member _.Functions = functions
    member _.WasmBytes = Array.copy copiedBytes
    member _.WasmPath = wasmPath

[<RequireQualifiedAccess>]
module NibWasmCompiler =
    let private maxSourceLength = 1_000_000
    let private maxExpressionNesting = 256
    let private functionPattern =
        Regex(
            """fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*->\s*u4\s*\{\s*return\s+([^;]+);\s*\}""",
            RegexOptions.Singleline ||| RegexOptions.CultureInvariant
        )
    let private identifierPattern = Regex("""\A[A-Za-z_][A-Za-z0-9_]*\z""", RegexOptions.CultureInvariant)
    let private callPattern =
        Regex("""\A([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\z""", RegexOptions.Singleline ||| RegexOptions.CultureInvariant)

    let private parseParameters text =
        if String.IsNullOrWhiteSpace text then
            []
        else
            let parameters =
                text.Split(',')
                |> Array.map (fun piece ->
                    let parts = Regex.Split(piece.Trim(), """\s*:\s*""")
                    if parts.Length <> 2 || parts[1] <> "u4" || not (identifierPattern.IsMatch parts[0]) then
                        raise (PackageError("parse", "parameters must be `name: u4`"))
                    parts[0])
                |> Array.toList

            if (parameters |> List.distinct).Length <> parameters.Length then
                raise (PackageError("validate", "duplicate parameter"))

            parameters

    let private parse (source: string) =
        if source.Length > maxSourceLength then
            raise (PackageError("parse", $"source exceeds {maxSourceLength} characters"))

        let functions = ResizeArray<NibFunction>()
        let mutable cursor = 0
        for matched in functionPattern.Matches(source) |> Seq.cast<Match> do
            if not (String.IsNullOrWhiteSpace(source.Substring(cursor, matched.Index - cursor))) then
                raise (PackageError("parse", "unexpected text before function"))

            functions.Add(
                NibFunction(
                    matched.Groups[1].Value,
                    parseParameters matched.Groups[2].Value,
                    matched.Groups[3].Value
                )
            )
            cursor <- matched.Index + matched.Length

        if functions.Count = 0 || not (String.IsNullOrWhiteSpace(source[cursor..])) then
            raise (PackageError("parse", "expected one or more Nib functions"))

        functions |> Seq.toList

    let private indexFunctions (functions: NibFunction list) =
        let indexed = Dictionary<string, NibFunction * int>(StringComparer.Ordinal)
        functions
        |> List.iteri (fun index functionValue ->
            if not (indexed.TryAdd(functionValue.Name, (functionValue, index))) then
                raise (PackageError("validate", $"duplicate function `{functionValue.Name}`")))
        indexed

    let private parameterMap (functionValue: NibFunction) =
        functionValue.Parameters
        |> List.mapi (fun index name -> name, index)
        |> Map.ofList

    let private append (target: ResizeArray<byte>) (bytes: seq<byte>) =
        target.AddRange(bytes)

    let private u32 (target: ResizeArray<byte>) value =
        append target (WasmLeb128.encodeUnsignedInt value)

    let private i32 (target: ResizeArray<byte>) value =
        target.Add(0x41uy)
        append target (WasmLeb128.encodeSigned value)

    let private splitTopLevel (text: string) (delimiter: string) =
        let parts = ResizeArray<string>()
        let mutable nesting = 0
        let mutable start = 0
        let mutable index = 0
        while index < text.Length do
            match text[index] with
            | '(' -> nesting <- nesting + 1
            | ')' -> nesting <- nesting - 1
            | _ -> ()

            if nesting = 0 && index + delimiter.Length <= text.Length && text.Substring(index, delimiter.Length) = delimiter then
                parts.Add(text.Substring(start, index - start).Trim())
                index <- index + delimiter.Length
                start <- index
            else
                index <- index + 1

        parts.Add(text[start..].Trim())
        parts |> Seq.toList

    let private splitArguments text =
        if String.IsNullOrWhiteSpace text then [] else splitTopLevel text ","

    let rec private emitExpression
        (target: ResizeArray<byte>)
        expression
        (functions: IReadOnlyDictionary<string, NibFunction * int>)
        (parameters: Map<string, int>)
        emit
        depth
        =
        if depth > maxExpressionNesting then
            raise (PackageError("validate", $"expression nesting exceeds {maxExpressionNesting}"))

        let addition = splitTopLevel expression "+%"
        if addition.Length > 1 then
            emitExpression target addition.Head functions parameters emit (depth + 1)
            for part in addition.Tail do
                emitExpression target part functions parameters emit (depth + 1)
                if emit then
                    target.Add(0x6Auy)
                    i32 target 15
                    target.Add(0x71uy)
        else
            let trimmed = expression.Trim()
            let mutable literal = 0
            if Int32.TryParse(trimmed, &literal) then
                if literal < 0 || literal > 15 then
                    raise (PackageError("validate", $"u4 literal out of range: {literal}"))
                if emit then i32 target literal
            else
                let call = callPattern.Match trimmed
                if call.Success then
                    let name = call.Groups[1].Value
                    match functions.TryGetValue name with
                    | false, _ -> raise (PackageError("validate", $"unknown function `{name}`"))
                    | true, (calledFunction, functionIndex) ->
                        let arguments = splitArguments call.Groups[2].Value
                        if arguments.Length <> calledFunction.Parameters.Length then
                            raise (PackageError("validate", $"wrong arity for `{calledFunction.Name}`"))
                        for argument in arguments do
                            emitExpression target argument functions parameters emit (depth + 1)
                        if emit then
                            target.Add(0x10uy)
                            u32 target functionIndex
                else
                    match parameters.TryFind trimmed with
                    | Some parameterIndex ->
                        if emit then
                            target.Add(0x20uy)
                            u32 target parameterIndex
                    | None -> raise (PackageError("validate", $"unsupported expression `{expression}`"))

    let private emitModule (functions: NibFunction list) (indexed: IReadOnlyDictionary<string, NibFunction * int>) =
        let moduleValue = WasmModule()
        functions
        |> List.iteri (fun functionIndex (functionValue: NibFunction) ->
            moduleValue.Types.Add(
                WasmTypes.makeFuncType
                    (List.replicate functionValue.Parameters.Length ValueType.I32)
                    [ ValueType.I32 ]
            )
            moduleValue.Functions.Add(functionIndex)
            moduleValue.Exports.Add(
                { Name = functionValue.Name; Kind = ExternalKind.FUNCTION; Index = functionIndex }
            )
            let body = ResizeArray<byte>()
            emitExpression body functionValue.Expression indexed (parameterMap functionValue) true 0
            body.Add(0x0Buy)
            moduleValue.Code.Add(FunctionBody([], body.ToArray())))
        WasmModuleEncoder.encodeModule moduleValue

    let compileSource (source: string) =
        if isNull source then nullArg "source"
        let functions = parse source
        let indexed = indexFunctions functions
        for functionValue in functions do
            emitExpression (ResizeArray<byte>()) functionValue.Expression indexed (parameterMap functionValue) false 0
        PackageResult(source, functions, emitModule functions indexed, None)

    let packSource source = compileSource source

    let writeWasmFile source path =
        if isNull path then nullArg "path"
        let result = compileSource source
        try
            File.WriteAllBytes(path, result.WasmBytes)
        with
        | :? IOException as error -> raise (PackageError("write", error.Message))
        | :? UnauthorizedAccessException as error -> raise (PackageError("write", error.Message))
        PackageResult(result.Source, result.Functions, result.WasmBytes, Some path)
