namespace CodingAdventures.CliBuilder.FSharp

open System.Collections.Generic

module private Core =
    let toList (items: seq<'a>) = items |> Seq.toList

type ValueType = global.CodingAdventures.CliBuilder.ValueType
type ParsingMode = global.CodingAdventures.CliBuilder.ParsingMode
type BuiltinFlags = global.CodingAdventures.CliBuilder.BuiltinFlags
type FlagDef = global.CodingAdventures.CliBuilder.FlagDef
type ArgDef = global.CodingAdventures.CliBuilder.ArgDef
type ExclusiveGroup = global.CodingAdventures.CliBuilder.ExclusiveGroup
type CommandDef = global.CodingAdventures.CliBuilder.CommandDef
type CliSpec = global.CodingAdventures.CliBuilder.CliSpec
type ValidationResult = global.CodingAdventures.CliBuilder.ValidationResult
type ParseError = global.CodingAdventures.CliBuilder.ParseError
type ParserResult = global.CodingAdventures.CliBuilder.ParserResult
type ParseResult = global.CodingAdventures.CliBuilder.ParseResult
type HelpResult = global.CodingAdventures.CliBuilder.HelpResult
type VersionResult = global.CodingAdventures.CliBuilder.VersionResult
type CliBuilderError = global.CodingAdventures.CliBuilder.CliBuilderError
type SpecError = global.CodingAdventures.CliBuilder.SpecError
type ParseErrors = global.CodingAdventures.CliBuilder.ParseErrors
type TokenEventType = global.CodingAdventures.CliBuilder.TokenEventType
type TokenEvent = global.CodingAdventures.CliBuilder.TokenEvent
type EndOfFlagsToken = global.CodingAdventures.CliBuilder.EndOfFlagsToken
type LongFlagToken = global.CodingAdventures.CliBuilder.LongFlagToken
type LongFlagWithValueToken = global.CodingAdventures.CliBuilder.LongFlagWithValueToken
type SingleDashLongToken = global.CodingAdventures.CliBuilder.SingleDashLongToken
type ShortFlagToken = global.CodingAdventures.CliBuilder.ShortFlagToken
type ShortFlagWithValueToken = global.CodingAdventures.CliBuilder.ShortFlagWithValueToken
type StackedFlagsToken = global.CodingAdventures.CliBuilder.StackedFlagsToken
type PositionalToken = global.CodingAdventures.CliBuilder.PositionalToken
type UnknownFlagToken = global.CodingAdventures.CliBuilder.UnknownFlagToken

type SpecLoader(specFilePath: string) =
    let inner = global.CodingAdventures.CliBuilder.SpecLoader(specFilePath)

    member _.Load() = inner.Load()

    member _.LoadFromObject(raw: IDictionary<string, obj>) =
        inner.LoadFromObject(raw :> IDictionary<string, obj>)

    static member ValidateSpec(specFilePath: string) =
        global.CodingAdventures.CliBuilder.SpecLoader.ValidateSpec(specFilePath)

    static member ValidateSpecObject(raw: IDictionary<string, obj>) =
        global.CodingAdventures.CliBuilder.SpecLoader.ValidateSpecObject(raw :> IDictionary<string, obj>)

type TokenClassifier(activeFlags: seq<FlagDef>) =
    let inner = global.CodingAdventures.CliBuilder.TokenClassifier(Core.toList activeFlags)

    member _.Classify(token: string) = inner.Classify(token)

type PositionalResolver(argumentDefinitions: seq<ArgDef>) =
    let inner = global.CodingAdventures.CliBuilder.PositionalResolver(Core.toList argumentDefinitions)

    member _.Resolve(tokens: seq<string>, parsedFlags: IReadOnlyDictionary<string, obj>, context: seq<string>) =
        inner.Resolve(Core.toList tokens, parsedFlags, Core.toList context)

    static member CoerceValue(raw: string, valueType: ValueType, argumentId: string, context: seq<string>, ?enumValues: IReadOnlyList<string>) =
        global.CodingAdventures.CliBuilder.PositionalResolver.CoerceValue(raw, valueType, argumentId, Core.toList context, ?enumValues = enumValues)

type FlagValidator(activeFlags: seq<FlagDef>, exclusiveGroups: seq<ExclusiveGroup>) =
    let inner = global.CodingAdventures.CliBuilder.FlagValidator(Core.toList activeFlags, Core.toList exclusiveGroups)

    member _.Validate(parsedFlags: IReadOnlyDictionary<string, obj>, context: seq<string>) =
        inner.Validate(parsedFlags, Core.toList context)

type HelpGenerator(spec: CliSpec, commandSegments: seq<string>) =
    let inner = global.CodingAdventures.CliBuilder.HelpGenerator(spec, Core.toList commandSegments)

    member _.Generate() = inner.Generate()

type Parser private (inner: global.CodingAdventures.CliBuilder.Parser) =
    new (specFilePath: string, argv: seq<string>) =
        Parser(global.CodingAdventures.CliBuilder.Parser(specFilePath, Core.toList argv))

    new (spec: CliSpec, argv: seq<string>) =
        Parser(global.CodingAdventures.CliBuilder.Parser(spec, Core.toList argv))

    member _.Parse() = inner.Parse()
