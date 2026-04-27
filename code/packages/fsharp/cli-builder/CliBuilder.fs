namespace CodingAdventures.CliBuilder.FSharp

open System
open System.Collections
open System.Collections.Generic
open System.Globalization
open System.IO
open System.Text
open CodingAdventures.DirectedGraph.FSharp

type JsonNode = CodingAdventures.JsonValue.FSharp.JsonValue

type ValueType =
    | Boolean = 0
    | Count = 1
    | String = 2
    | Integer = 3
    | Float = 4
    | Path = 5
    | File = 6
    | Directory = 7
    | Enum = 8

type ParsingMode =
    | Gnu = 0
    | Posix = 1
    | SubcommandFirst = 2
    | Traditional = 3

[<CLIMutable>]
type BuiltinFlags =
    { Help: bool
      Version: bool }

[<CLIMutable>]
type FlagDef =
    { Id: string
      Short: string
      Long: string
      SingleDashLong: string
      Description: string
      Type: ValueType
      Required: bool
      Default: obj
      ValueName: string
      EnumValues: IReadOnlyList<string>
      DefaultWhenPresent: string
      ConflictsWith: IReadOnlyList<string>
      Requires: IReadOnlyList<string>
      RequiredUnless: IReadOnlyList<string>
      Repeatable: bool }

[<CLIMutable>]
type ArgDef =
    { Id: string
      DisplayName: string
      Description: string
      Type: ValueType
      Required: bool
      Variadic: bool
      VariadicMin: int
      VariadicMax: Nullable<int>
      Default: obj
      EnumValues: IReadOnlyList<string>
      RequiredUnlessFlag: IReadOnlyList<string> }

[<CLIMutable>]
type ExclusiveGroup =
    { Id: string
      FlagIds: IReadOnlyList<string>
      Required: bool }

[<CLIMutable>]
type CommandDef =
    { Id: string
      Name: string
      Aliases: IReadOnlyList<string>
      Description: string
      InheritGlobalFlags: bool
      Flags: IReadOnlyList<FlagDef>
      Arguments: IReadOnlyList<ArgDef>
      Commands: IReadOnlyList<CommandDef>
      MutuallyExclusiveGroups: IReadOnlyList<ExclusiveGroup> }

[<CLIMutable>]
type CliSpec =
    { SpecVersion: string
      Name: string
      DisplayName: string
      Description: string
      Version: string
      ParsingMode: ParsingMode
      BuiltinFlags: BuiltinFlags
      GlobalFlags: IReadOnlyList<FlagDef>
      Flags: IReadOnlyList<FlagDef>
      Arguments: IReadOnlyList<ArgDef>
      Commands: IReadOnlyList<CommandDef>
      MutuallyExclusiveGroups: IReadOnlyList<ExclusiveGroup> }

[<CLIMutable>]
type ValidationResult =
    { IsValid: bool
      Errors: IReadOnlyList<string> }

[<CLIMutable>]
type ParseError =
    { ErrorType: string
      Message: string
      Suggestion: string
      Context: IReadOnlyList<string> }

[<AbstractClass>]
type ParserResult() = class end

type ParseResult(program: string, commandPath: IReadOnlyList<string>, flags: IReadOnlyDictionary<string, obj>, arguments: IReadOnlyDictionary<string, obj>, explicitFlags: IReadOnlyList<string>) =
    inherit ParserResult()

    member _.Program = program
    member _.CommandPath = commandPath
    member _.Flags = flags
    member _.Arguments = arguments
    member _.ExplicitFlags = explicitFlags

type HelpResult(text: string, commandPath: IReadOnlyList<string>) =
    inherit ParserResult()

    member _.Text = text
    member _.CommandPath = commandPath

type VersionResult(version: string) =
    inherit ParserResult()

    member _.Version = version

type CliBuilderError(message: string) =
    inherit Exception(message)

type SpecError(message: string) =
    inherit CliBuilderError(message)

type ParseErrors(errors: IReadOnlyList<ParseError>) =
    inherit CliBuilderError(
        if errors.Count = 1 then
            errors.[0].Message
        else
            let details =
                errors
                |> Seq.map (fun error -> error.Message)
                |> String.concat "\n  - "

            sprintf "%d parse errors:\n  - %s" errors.Count details)

    member _.Errors = errors

type TokenEventType =
    | EndOfFlags = 0
    | LongFlag = 1
    | LongFlagWithValue = 2
    | SingleDashLong = 3
    | ShortFlag = 4
    | ShortFlagWithValue = 5
    | StackedFlags = 6
    | Positional = 7
    | UnknownFlag = 8

[<AbstractClass>]
type TokenEvent(eventType: TokenEventType) =
    member _.Type = eventType

type EndOfFlagsToken() =
    inherit TokenEvent(TokenEventType.EndOfFlags)

type LongFlagToken(name: string) =
    inherit TokenEvent(TokenEventType.LongFlag)

    member _.Name = name

type LongFlagWithValueToken(name: string, value: string) =
    inherit TokenEvent(TokenEventType.LongFlagWithValue)

    member _.Name = name
    member _.Value = value

type SingleDashLongToken(name: string) =
    inherit TokenEvent(TokenEventType.SingleDashLong)

    member _.Name = name

type ShortFlagToken(charValue: string) =
    inherit TokenEvent(TokenEventType.ShortFlag)

    member _.Char = charValue

type ShortFlagWithValueToken(charValue: string, value: string) =
    inherit TokenEvent(TokenEventType.ShortFlagWithValue)

    member _.Char = charValue
    member _.Value = value

type StackedFlagsToken(chars: IReadOnlyList<string>) =
    inherit TokenEvent(TokenEventType.StackedFlags)

    member _.Chars = chars

type PositionalToken(value: string) =
    inherit TokenEvent(TokenEventType.Positional)

    member _.Value = value

type UnknownFlagToken(raw: string) =
    inherit TokenEvent(TokenEventType.UnknownFlag)

    member _.Raw = raw

module private Core =
    let ordinal = StringComparer.Ordinal

    let emptyReadOnly<'a> : IReadOnlyList<'a> = ResizeArray<'a>() :> IReadOnlyList<'a>

    let toReadOnlyList (items: seq<'a>) : IReadOnlyList<'a> =
        ResizeArray<'a>(items) :> IReadOnlyList<'a>

    let toDictionary (pairs: seq<string * 'a>) =
        let dictionary = Dictionary<string, 'a>(ordinal)
        for key, value in pairs do
            dictionary.[key] <- value

        dictionary

    let tryGetValue (values: IReadOnlyDictionary<string, 'a>) key =
        match values.TryGetValue(key) with
        | true, value -> Some value
        | _ -> None

    let asObject (raw: obj) path =
        match raw with
        | :? IDictionary<string, obj> as dictionary ->
            dictionary
        | :? IDictionary as dictionary ->
            let typed = Dictionary<string, obj>(ordinal)

            for entry in dictionary do
                let dictionaryEntry = unbox<DictionaryEntry> entry

                match dictionaryEntry.Key with
                | :? string as key ->
                    typed.[key] <- dictionaryEntry.Value
                | _ ->
                    raise (SpecError(sprintf "%s must be an object." path))

            typed :> IDictionary<string, obj>
        | _ ->
            raise (SpecError(sprintf "%s must be an object." path))

    let asArray (raw: obj) path =
        match raw with
        | :? IReadOnlyList<obj> as items ->
            items
        | :? IEnumerable as enumerable when not (raw :? string) ->
            enumerable |> Seq.cast<obj> |> toReadOnlyList
        | _ ->
            raise (SpecError(sprintf "%s must be an array." path))

    let optionalString (raw: IDictionary<string, obj>) fieldName =
        match raw.TryGetValue(fieldName) with
        | true, (:? string as value) -> value
        | _ -> null

    let requireString (raw: IDictionary<string, obj>) fieldName path =
        let value = optionalString raw fieldName

        if isNull value then
            raise (SpecError(sprintf "%s.%s must be a string." path fieldName))

        value

    let convertToBoolean (raw: obj) path =
        match raw with
        | :? bool as value -> value
        | _ -> raise (SpecError(sprintf "%s must be a boolean." path))

    let convertToInteger (raw: obj) path =
        match raw with
        | :? int as value -> value
        | :? int64 as value when value >= int64 Int32.MinValue && value <= int64 Int32.MaxValue -> int value
        | :? double as value when not (Double.IsNaN(value)) && not (Double.IsInfinity(value)) && Math.Truncate(value) = value && value >= double Int32.MinValue && value <= double Int32.MaxValue ->
            int value
        | _ ->
            raise (SpecError(sprintf "%s must be an integer." path))

    let optionalBoolean (raw: IDictionary<string, obj>) fieldName =
        match raw.TryGetValue(fieldName) with
        | true, value when not (isNull value) -> Some (convertToBoolean value fieldName)
        | _ -> None

    let optionalInteger (raw: IDictionary<string, obj>) fieldName =
        match raw.TryGetValue(fieldName) with
        | true, value when not (isNull value) -> Some (convertToInteger value fieldName)
        | _ -> None

    let tryGetMutableValue (values: IDictionary<string, obj>) key =
        match values.TryGetValue(key) with
        | true, value -> Some value
        | _ -> None

    let optionalStringArray (raw: IDictionary<string, obj>) fieldName =
        match raw.TryGetValue(fieldName) with
        | false, _
        | true, null ->
            emptyReadOnly
        | true, value ->
            asArray value fieldName
            |> Seq.map (function
                | :? string as item -> item
                | _ -> raise (SpecError(sprintf "%s entries must be strings." fieldName)))
            |> toReadOnlyList

    let getRepeatableValues (parsedFlags: IDictionary<string, obj>) flagId =
        match tryGetMutableValue parsedFlags flagId with
        | Some (:? List<obj> as list) ->
            list
        | Some (:? IList<obj> as values) ->
            let list = List<obj>(values :> IEnumerable<obj>)
            parsedFlags.[flagId] <- box list
            list
        | _ ->
            let list = List<obj>()
            parsedFlags.[flagId] <- box list
            list

module private FlagPresence =
    let isPresentValue (value: obj) =
        match value with
        | null -> false
        | :? bool as boolValue -> boolValue
        | :? string as stringValue -> not (String.IsNullOrEmpty(stringValue))
        | :? IList<obj> as list -> list.Count > 0
        | :? int64 as integerValue -> integerValue > 0L
        | :? int as integerValue -> integerValue > 0
        | _ -> true

    let isPresentFlag (flag: FlagDef) (value: obj) =
        if flag.Repeatable then
            match value with
            | :? IList<obj> as list -> list.Count > 0
            | _ -> false
        else
            match flag.Type, value with
            | ValueType.Boolean, (:? bool as boolValue) -> boolValue
            | ValueType.Count, (:? int64 as countValue) -> countValue > 0L
            | ValueType.Count, (:? int as countValue) -> countValue > 0
            | _ -> isPresentValue value

module private SpecParsing =
    let private parseParsingMode (raw: obj) path =
        match raw with
        | null -> ParsingMode.Gnu
        | :? string as value ->
            match value with
            | "gnu" -> ParsingMode.Gnu
            | "posix" -> ParsingMode.Posix
            | "subcommand_first" -> ParsingMode.SubcommandFirst
            | "traditional" -> ParsingMode.Traditional
            | _ ->
                raise (SpecError(sprintf "parsing_mode at %s must be one of gnu, posix, subcommand_first, or traditional." path))
        | _ ->
            raise (SpecError(sprintf "parsing_mode at %s must be one of gnu, posix, subcommand_first, or traditional." path))

    let private parseValueType raw path =
        match raw with
        | "boolean" -> ValueType.Boolean
        | "count" -> ValueType.Count
        | "string" -> ValueType.String
        | "integer" -> ValueType.Integer
        | "float" -> ValueType.Float
        | "path" -> ValueType.Path
        | "file" -> ValueType.File
        | "directory" -> ValueType.Directory
        | "enum" -> ValueType.Enum
        | _ ->
            raise (SpecError(sprintf "Unsupported value type \"%s\" at %s." raw path))

    let private parseBuiltinFlags raw =
        if isNull raw then
            { Help = true; Version = true }
        else
            let map = Core.asObject raw "builtin_flags"

            { Help =
                match map.TryGetValue("help") with
                | true, value when not (isNull value) -> Core.convertToBoolean value "builtin_flags.help"
                | _ -> true
              Version =
                match map.TryGetValue("version") with
                | true, value when not (isNull value) -> Core.convertToBoolean value "builtin_flags.version"
                | _ -> true }

    let rec parseCommandArray raw fieldPath (globalFlags: IReadOnlyList<FlagDef>) =
        if isNull raw then
            Core.emptyReadOnly
        else
            let items = Core.asArray raw fieldPath
            let commands = ResizeArray<CommandDef>()
            let seenIds = HashSet<string>(Core.ordinal)

            for index in 0 .. items.Count - 1 do
                let command = parseCommand (Core.asObject items.[index] (sprintf "%s[%d]" fieldPath index)) (sprintf "%s[%d]" fieldPath index) globalFlags
                if not (seenIds.Add(command.Id)) then
                    raise (SpecError(sprintf "Duplicate command id \"%s\" in %s" command.Id fieldPath))

                commands.Add(command)

            commands :> IReadOnlyList<CommandDef>

    and parseExclusiveGroups raw fieldPath (localFlags: IReadOnlyList<FlagDef>) (globalFlags: IReadOnlyList<FlagDef>) =
        if isNull raw then
            Core.emptyReadOnly
        else
            let validFlagIds =
                seq {
                    for flag in globalFlags do
                        yield flag.Id

                    for flag in localFlags do
                        yield flag.Id
                }
                |> HashSet<string>

            let groups = ResizeArray<ExclusiveGroup>()
            let items = Core.asArray raw fieldPath

            for index in 0 .. items.Count - 1 do
                let map = Core.asObject items.[index] (sprintf "%s[%d]" fieldPath index)
                let groupId = Core.requireString map "id" (sprintf "%s[%d]" fieldPath index)
                let ids = Core.optionalStringArray map "flag_ids"

                for flagId in ids do
                    if not (validFlagIds.Contains(flagId)) then
                        raise (SpecError(sprintf "Exclusive group \"%s\" references unknown flag id \"%s\" in %s" groupId flagId fieldPath))

                groups.Add(
                    { Id = groupId
                      FlagIds = ids
                      Required = defaultArg (Core.optionalBoolean map "required") false })

            groups :> IReadOnlyList<ExclusiveGroup>

    and parseFlag raw path =
        let flagId = Core.requireString raw "id" path
        let description = Core.requireString raw "description" path
        let valueType = parseValueType (Core.requireString raw "type" path) path
        let shortName = Core.optionalString raw "short"
        let longName = Core.optionalString raw "long"
        let singleDashLong = Core.optionalString raw "single_dash_long"

        if isNull shortName && isNull longName && isNull singleDashLong then
            raise (SpecError(sprintf "Flag \"%s\" at %s must have at least one of short, long, or single_dash_long." flagId path))

        let enumValues = Core.optionalStringArray raw "enum_values"
        if valueType = ValueType.Enum && enumValues.Count = 0 then
            raise (SpecError(sprintf "Flag \"%s\" at %s has type enum but enum_values is empty." flagId path))

        let defaultWhenPresent = Core.optionalString raw "default_when_present"

        if not (isNull defaultWhenPresent) then
            if valueType <> ValueType.Enum then
                raise (SpecError(sprintf "Flag \"%s\" at %s uses default_when_present but is not an enum." flagId path))

            if not (enumValues |> Seq.contains defaultWhenPresent) then
                raise (SpecError(sprintf "Flag \"%s\" at %s has default_when_present \"%s\" outside enum_values." flagId path defaultWhenPresent))

        { Id = flagId
          Short = shortName
          Long = longName
          SingleDashLong = singleDashLong
          Description = description
          Type = valueType
          Required = defaultArg (Core.optionalBoolean raw "required") false
          Default =
            match raw.TryGetValue("default") with
            | true, defaultValue -> defaultValue
            | _ -> null
          ValueName = Core.optionalString raw "value_name"
          EnumValues = enumValues
          DefaultWhenPresent = defaultWhenPresent
          ConflictsWith = Core.optionalStringArray raw "conflicts_with"
          Requires = Core.optionalStringArray raw "requires"
          RequiredUnless = Core.optionalStringArray raw "required_unless"
          Repeatable = defaultArg (Core.optionalBoolean raw "repeatable") false }

    and parseFlagArray raw fieldPath (globalFlags: IReadOnlyList<FlagDef>) =
        if isNull raw then
            Core.emptyReadOnly
        else
            let items = Core.asArray raw fieldPath
            let flags = ResizeArray<FlagDef>()
            let seenIds = HashSet<string>(Core.ordinal)

            for index in 0 .. items.Count - 1 do
                let flag = parseFlag (Core.asObject items.[index] (sprintf "%s[%d]" fieldPath index)) (sprintf "%s[%d]" fieldPath index)

                if not (seenIds.Add(flag.Id)) then
                    raise (SpecError(sprintf "Duplicate flag id \"%s\" in %s" flag.Id fieldPath))

                flags.Add(flag)

            let validFlagIds =
                seq {
                    for flag in globalFlags do
                        yield flag.Id

                    for flag in flags do
                        yield flag.Id
                }
                |> HashSet<string>

            for flag in flags do
                for reference in Seq.concat [ flag.ConflictsWith; flag.Requires; flag.RequiredUnless ] do
                    if not (validFlagIds.Contains(reference)) then
                        raise (SpecError(sprintf "Flag \"%s\" references unknown flag id \"%s\" in %s" flag.Id reference fieldPath))

            flags :> IReadOnlyList<FlagDef>

    and private parseArg raw path =
        let argId = Core.requireString raw "id" path

        let displayName =
            let explicitDisplay = Core.optionalString raw "display_name"

            if not (isNull explicitDisplay) then
                explicitDisplay
            else
                let fallback = Core.optionalString raw "name"
                if isNull fallback then
                    raise (SpecError(sprintf "Argument \"%s\" at %s is missing display_name." argId path))

                fallback

        let description = Core.requireString raw "description" path
        let valueType = parseValueType (Core.requireString raw "type" path) path
        let required = defaultArg (Core.optionalBoolean raw "required") true
        let variadic = defaultArg (Core.optionalBoolean raw "variadic") false
        let variadicMin =
            defaultArg
                (Core.optionalInteger raw "variadic_min")
                (if variadic then
                     if required then 1 else 0
                 else
                     0)

        let variadicMax =
            match raw.TryGetValue("variadic_max") with
            | true, value when not (isNull value) -> Nullable(Core.convertToInteger value (sprintf "%s.variadic_max" path))
            | _ -> Nullable()

        let enumValues = Core.optionalStringArray raw "enum_values"
        if valueType = ValueType.Enum && enumValues.Count = 0 then
            raise (SpecError(sprintf "Argument \"%s\" at %s has type enum but enum_values is empty." argId path))

        { Id = argId
          DisplayName = displayName
          Description = description
          Type = valueType
          Required = required
          Variadic = variadic
          VariadicMin = variadicMin
          VariadicMax = variadicMax
          Default =
            match raw.TryGetValue("default") with
            | true, defaultValue -> defaultValue
            | _ -> null
          EnumValues = enumValues
          RequiredUnlessFlag = Core.optionalStringArray raw "required_unless_flag" }

    and parseArgArray raw fieldPath =
        if isNull raw then
            Core.emptyReadOnly
        else
            let items = Core.asArray raw fieldPath
            let args = ResizeArray<ArgDef>()
            let seenIds = HashSet<string>(Core.ordinal)

            for index in 0 .. items.Count - 1 do
                let argument = parseArg (Core.asObject items.[index] (sprintf "%s[%d]" fieldPath index)) (sprintf "%s[%d]" fieldPath index)

                if not (seenIds.Add(argument.Id)) then
                    raise (SpecError(sprintf "Duplicate argument id \"%s\" in %s" argument.Id fieldPath))

                args.Add(argument)

            args :> IReadOnlyList<ArgDef>

    and parseCommand raw path (globalFlags: IReadOnlyList<FlagDef>) =
        let commandId = Core.requireString raw "id" path
        let name = Core.requireString raw "name" path
        let description = Core.requireString raw "description" path
        let inheritGlobalFlags = defaultArg (Core.optionalBoolean raw "inherit_global_flags") true
        let visibleGlobalFlags = if inheritGlobalFlags then globalFlags else Core.emptyReadOnly
        let flags = parseFlagArray (match raw.TryGetValue("flags") with | true, value -> value | _ -> null) (sprintf "%s.flags" path) visibleGlobalFlags
        let arguments = parseArgArray (match raw.TryGetValue("arguments") with | true, value -> value | _ -> null) (sprintf "%s.arguments" path)
        let commands = parseCommandArray (match raw.TryGetValue("commands") with | true, value -> value | _ -> null) (sprintf "%s.commands" path) globalFlags
        let groups =
            parseExclusiveGroups
                (match raw.TryGetValue("mutually_exclusive_groups") with | true, value -> value | _ -> null)
                (sprintf "%s.mutually_exclusive_groups" path)
                flags
                visibleGlobalFlags

        let aliases = Core.optionalStringArray raw "aliases"
        checkVariadicCount arguments path
        checkFlagRequiresGraph (seq { yield! visibleGlobalFlags; yield! flags } |> Core.toReadOnlyList) path

        { Id = commandId
          Name = name
          Aliases = aliases
          Description = description
          InheritGlobalFlags = inheritGlobalFlags
          Flags = flags
          Arguments = arguments
          Commands = commands
          MutuallyExclusiveGroups = groups }

    and checkVariadicCount (arguments: IReadOnlyList<ArgDef>) path =
        let variadicCount = arguments |> Seq.filter (fun argument -> argument.Variadic) |> Seq.length
        if variadicCount > 1 then
            raise (SpecError(sprintf "At most one variadic argument is allowed in %s." path))

    and checkFlagRequiresGraph (flags: IReadOnlyList<FlagDef>) path =
        let graph = Graph()

        for flag in flags do
            graph.AddNode(flag.Id)

        for flag in flags do
            for requiredFlagId in flag.Requires do
                if graph.HasNode(requiredFlagId) then
                    graph.AddEdge(flag.Id, requiredFlagId)

        try
            graph.TopologicalSort() |> ignore
        with
        | :? CycleError as error ->
            raise (SpecError(sprintf "Circular requires dependency detected in %s: %s" path (String.concat " -> " error.Cycle)))

    let parseSpec (raw: IDictionary<string, obj>) =
        let specVersion = Core.requireString raw "cli_builder_spec_version" "root"
        if specVersion <> "1.0" then
            raise (SpecError(sprintf "cli_builder_spec_version must be \"1.0\", got: %s" specVersion))

        let name = Core.requireString raw "name" "root"
        let description = Core.requireString raw "description" "root"
        let displayName = Core.optionalString raw "display_name"
        let version = Core.optionalString raw "version"
        let parsingMode = parseParsingMode (match raw.TryGetValue("parsing_mode") with | true, value -> value | _ -> null) "root"
        let builtinFlags = parseBuiltinFlags (match raw.TryGetValue("builtin_flags") with | true, value -> value | _ -> null)
        let globalFlags = parseFlagArray (match raw.TryGetValue("global_flags") with | true, value -> value | _ -> null) "global_flags" Core.emptyReadOnly
        let flags = parseFlagArray (match raw.TryGetValue("flags") with | true, value -> value | _ -> null) "flags" globalFlags
        let arguments = parseArgArray (match raw.TryGetValue("arguments") with | true, value -> value | _ -> null) "arguments"
        let commands = parseCommandArray (match raw.TryGetValue("commands") with | true, value -> value | _ -> null) "commands" globalFlags
        let groups =
            parseExclusiveGroups
                (match raw.TryGetValue("mutually_exclusive_groups") with | true, value -> value | _ -> null)
                "mutually_exclusive_groups"
                flags
                globalFlags

        checkVariadicCount arguments "root"
        checkFlagRequiresGraph (seq { yield! globalFlags; yield! flags } |> Core.toReadOnlyList) "root"

        { SpecVersion = "1.0"
          Name = name
          DisplayName = displayName
          Description = description
          Version = version
          ParsingMode = parsingMode
          BuiltinFlags = builtinFlags
          GlobalFlags = globalFlags
          Flags = flags
          Arguments = arguments
          Commands = commands
          MutuallyExclusiveGroups = groups }

type TokenClassifier(activeFlags: seq<FlagDef>) =
    let shortMap = Dictionary<string, FlagDef>(Core.ordinal)
    let longMap = Dictionary<string, FlagDef>(Core.ordinal)
    let singleDashLongMap = Dictionary<string, FlagDef>(Core.ordinal)

    do
        for flag in activeFlags do
            if not (String.IsNullOrEmpty(flag.Short)) && not (shortMap.ContainsKey(flag.Short)) then
                shortMap.[flag.Short] <- flag

            if not (String.IsNullOrEmpty(flag.Long)) && not (longMap.ContainsKey(flag.Long)) then
                longMap.[flag.Long] <- flag

            if not (String.IsNullOrEmpty(flag.SingleDashLong)) && not (singleDashLongMap.ContainsKey(flag.SingleDashLong)) then
                singleDashLongMap.[flag.SingleDashLong] <- flag

    let classifyStack (chars: string) =
        let values = ResizeArray<string>()
        let mutable index = 0
        let mutable unknown: string option = None

        while index < chars.Length && unknown.IsNone do
            let flagChar = chars.Substring(index, 1)

            match shortMap.TryGetValue(flagChar) with
            | false, _ ->
                unknown <- Some(sprintf "-%s" chars)
            | true, flag when flag.Type <> ValueType.Boolean && flag.Type <> ValueType.Count ->
                if index = chars.Length - 1 then
                    values.Add(flagChar)
                else
                    unknown <- Some(sprintf "-%s" chars)
            | true, _ ->
                values.Add(flagChar)

            index <- index + 1

        match unknown with
        | Some raw -> UnknownFlagToken(raw) :> TokenEvent
        | None -> StackedFlagsToken(values :> IReadOnlyList<string>) :> TokenEvent

    let classifySingleDash (token: string) =
        let rest = token.Substring(1)

        match singleDashLongMap.TryGetValue(rest) with
        | true, _ -> SingleDashLongToken(rest) :> TokenEvent
        | _ ->
            let first = rest.Substring(0, 1)

            match shortMap.TryGetValue(first) with
            | true, flag ->
                let remainder = rest.Substring(1)
                let consumesNoValue = flag.Type = ValueType.Boolean || flag.Type = ValueType.Count

                if consumesNoValue then
                    if remainder.Length = 0 then
                        ShortFlagToken(first) :> TokenEvent
                    else
                        classifyStack rest
                else if remainder.Length = 0 then
                    ShortFlagToken(first) :> TokenEvent
                else
                    ShortFlagWithValueToken(first, remainder) :> TokenEvent
            | _ ->
                classifyStack rest

    member _.Classify(token: string) =
        if token = "--" then
            EndOfFlagsToken() :> TokenEvent
        elif token.StartsWith("--", StringComparison.Ordinal) then
            let rest = token.Substring(2)
            let separatorIndex = rest.IndexOf('=')

            if separatorIndex >= 0 then
                LongFlagWithValueToken(rest.Substring(0, separatorIndex), rest.Substring(separatorIndex + 1)) :> TokenEvent
            else
                LongFlagToken(rest) :> TokenEvent
        elif token = "-" then
            PositionalToken(token) :> TokenEvent
        elif token.StartsWith("-", StringComparison.Ordinal) && token.Length > 1 then
            classifySingleDash token
        else
            PositionalToken(token) :> TokenEvent

type SpecLoader(specFilePath: string) =
    let mutable cached: CliSpec option = None

    member _.Load() =
        match cached with
        | Some spec -> spec
        | None ->
            if String.IsNullOrWhiteSpace(specFilePath) then
                raise (SpecError("No spec file path was provided."))

            try
                let text = File.ReadAllText(specFilePath)
                let raw = Core.asObject (JsonNode.ParseNative(text)) "root"
                let spec = SpecParsing.parseSpec raw
                cached <- Some spec
                spec
            with
            | :? SpecError ->
                reraise ()
            | ex ->
                raise (SpecError(sprintf "Failed to read spec file '%s': %s" specFilePath ex.Message))

    member _.LoadFromObject(raw: IDictionary<string, obj>) =
        match cached with
        | Some spec -> spec
        | None ->
            let spec = SpecParsing.parseSpec raw
            cached <- Some spec
            spec

    static member ValidateSpec(specFilePath: string) =
        try
            SpecLoader(specFilePath).Load() |> ignore
            { IsValid = true; Errors = Core.emptyReadOnly }
        with
        | :? SpecError as error ->
            { IsValid = false; Errors = Core.toReadOnlyList [ error.Message ] }

    static member ValidateSpecObject(raw: IDictionary<string, obj>) =
        try
            SpecLoader("<memory>").LoadFromObject(raw) |> ignore
            { IsValid = true; Errors = Core.emptyReadOnly }
        with
        | :? SpecError as error ->
            { IsValid = false; Errors = Core.toReadOnlyList [ error.Message ] }

type PositionalResolver(argumentDefinitions: seq<ArgDef>) =
    let argumentDefinitions = argumentDefinitions |> Core.toReadOnlyList

    let isArgumentRequired (argument: ArgDef) (parsedFlags: IReadOnlyDictionary<string, obj>) =
        if not argument.Required then
            false
        elif argument.RequiredUnlessFlag.Count = 0 then
            true
        else
            argument.RequiredUnlessFlag
            |> Seq.exists (fun flagId ->
                Core.tryGetValue parsedFlags flagId
                |> Option.defaultValue null
                |> FlagPresence.isPresentValue)
            |> not

    let buildMinimumSuffixRequirements (parsedFlags: IReadOnlyDictionary<string, obj>) =
        let suffix = Array.zeroCreate<int> (argumentDefinitions.Count + 1)

        for index in [ argumentDefinitions.Count - 1 .. -1 .. 0 ] do
            let argument = argumentDefinitions.[index]
            let requirement =
                if isArgumentRequired argument parsedFlags then
                    if argument.Variadic then argument.VariadicMin else 1
                else
                    0

            suffix.[index] <- suffix.[index + 1] + requirement

        suffix

    member _.Resolve(tokens: seq<string>, parsedFlags: IReadOnlyDictionary<string, obj>, context: seq<string>) =
        let positionalTokens = tokens |> Core.toReadOnlyList
        let context = context |> Core.toReadOnlyList
        let result = Dictionary<string, obj>(Core.ordinal)
        let errors = ResizeArray<ParseError>()
        let minimumSuffix = buildMinimumSuffixRequirements parsedFlags
        let mutable index = 0

        for argumentIndex in 0 .. argumentDefinitions.Count - 1 do
            let argument = argumentDefinitions.[argumentIndex]

            if argument.Variadic then
                let remaining = positionalTokens.Count - index
                let mustReserveForSuffix = minimumSuffix.[argumentIndex + 1]
                let available = max 0 (remaining - mustReserveForSuffix)
                let maxCount = if argument.VariadicMax.HasValue then argument.VariadicMax.Value else Int32.MaxValue
                let mutable take = min available maxCount
                let requiredMinimum = if isArgumentRequired argument parsedFlags then argument.VariadicMin else 0

                if take < requiredMinimum then
                    errors.Add(
                        { ErrorType = "too_few_arguments"
                          Message = sprintf "Argument \"%s\" expects at least %d values." argument.DisplayName requiredMinimum
                          Suggestion = null
                          Context = context })

                    take <- max 0 take

                let values = ResizeArray<obj>()

                for _ in 0 .. take - 1 do
                    let struct (value, error) =
                        PositionalResolver.CoerceValue(positionalTokens.[index], argument.Type, argument.Id, context, argument.EnumValues)

                    index <- index + 1

                    match error with
                    | Some parseError ->
                        errors.Add(parseError)
                    | None ->
                        values.Add(value |> Option.defaultValue null)

                result.[argument.Id] <- box values
            else
                let shouldConsume = positionalTokens.Count - index > minimumSuffix.[argumentIndex + 1]

                if shouldConsume then
                    let struct (value, error) =
                        PositionalResolver.CoerceValue(positionalTokens.[index], argument.Type, argument.Id, context, argument.EnumValues)

                    index <- index + 1

                    match error with
                    | Some parseError ->
                        errors.Add(parseError)
                        result.[argument.Id] <- argument.Default
                    | None ->
                        result.[argument.Id] <- value |> Option.defaultValue null
                elif isArgumentRequired argument parsedFlags then
                    errors.Add(
                        { ErrorType = "missing_required_argument"
                          Message = sprintf "Argument \"%s\" is required." argument.DisplayName
                          Suggestion = null
                          Context = context })

                    result.[argument.Id] <- argument.Default
                else
                    result.[argument.Id] <- argument.Default

        if index < positionalTokens.Count then
            errors.Add(
                { ErrorType = "too_many_arguments"
                  Message = sprintf "Received too many positional arguments: %s" (String.Join(" ", positionalTokens |> Seq.skip index))
                  Suggestion = null
                  Context = context })

        for argument in argumentDefinitions do
            if not (result.ContainsKey(argument.Id)) then
                result.[argument.Id] <-
                    if argument.Variadic then
                        box (ResizeArray<obj>())
                    else
                        argument.Default

        struct (result, errors)

    static member CoerceValue(raw: string, valueType: ValueType, argumentId: string, context: seq<string>, ?enumValues: IReadOnlyList<string>) =
        let context = context |> Core.toReadOnlyList
        let enumValues = defaultArg enumValues Core.emptyReadOnly

        match valueType with
        | ValueType.Boolean
        | ValueType.Count
        | ValueType.String
        | ValueType.Path ->
            struct (Some(box raw), None)
        | ValueType.Integer ->
            match Int64.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture) with
            | true, integerValue -> struct (Some(box integerValue), None)
            | _ ->
                struct (
                    None,
                    Some
                        { ErrorType = "invalid_value"
                          Message = sprintf "Invalid integer for \"%s\": '%s'" argumentId raw
                          Suggestion = null
                          Context = context }
                )
        | ValueType.Float ->
            match Double.TryParse(raw, NumberStyles.Float ||| NumberStyles.AllowThousands, CultureInfo.InvariantCulture) with
            | true, floatValue -> struct (Some(box floatValue), None)
            | _ ->
                struct (
                    None,
                    Some
                        { ErrorType = "invalid_value"
                          Message = sprintf "Invalid float for \"%s\": '%s'" argumentId raw
                          Suggestion = null
                          Context = context }
                )
        | ValueType.File ->
            if File.Exists(raw) then
                struct (Some(box raw), None)
            else
                struct (
                    None,
                    Some
                        { ErrorType = "invalid_value"
                          Message = sprintf "File not found: \"%s\"" raw
                          Suggestion = null
                          Context = context }
                )
        | ValueType.Directory ->
            if Directory.Exists(raw) then
                struct (Some(box raw), None)
            else
                struct (
                    None,
                    Some
                        { ErrorType = "invalid_value"
                          Message = sprintf "Directory not found: \"%s\"" raw
                          Suggestion = null
                          Context = context }
                )
        | ValueType.Enum ->
            if enumValues |> Seq.contains raw then
                struct (Some(box raw), None)
            else
                struct (
                    None,
                    Some
                        { ErrorType = "invalid_enum_value"
                          Message = sprintf "Invalid value '%s' for \"%s\". Must be one of: %s" raw argumentId (String.Join(", ", enumValues))
                          Suggestion = null
                          Context = context }
                )

type FlagValidator(activeFlags: seq<FlagDef>, exclusiveGroups: seq<ExclusiveGroup>) =
    let activeFlags = activeFlags |> Core.toReadOnlyList
    let exclusiveGroups = exclusiveGroups |> Core.toReadOnlyList
    let byId = Core.toDictionary (activeFlags |> Seq.map (fun flag -> flag.Id, flag))
    let requiresGraph = Graph()

    do
        for flag in activeFlags do
            requiresGraph.AddNode(flag.Id)

        for flag in activeFlags do
            for requiredFlag in flag.Requires do
                if requiresGraph.HasNode(requiredFlag) then
                    requiresGraph.AddEdge(flag.Id, requiredFlag)

    let displayFlag flag =
        let parts = ResizeArray<string>()

        if not (String.IsNullOrWhiteSpace(flag.Short)) then
            parts.Add(sprintf "-%s" flag.Short)

        if not (String.IsNullOrWhiteSpace(flag.Long)) then
            parts.Add(sprintf "--%s" flag.Long)

        if not (String.IsNullOrWhiteSpace(flag.SingleDashLong)) then
            parts.Add(sprintf "-%s" flag.SingleDashLong)

        String.Join("/", parts)

    member _.Validate(parsedFlags: IReadOnlyDictionary<string, obj>, context: seq<string>) =
        let context = context |> Core.toReadOnlyList
        let errors = ResizeArray<ParseError>()
        let presentFlags = HashSet<string>(Core.ordinal)
        let reportedConflicts = HashSet<string>(Core.ordinal)

        for flag in activeFlags do
            if FlagPresence.isPresentFlag flag (Core.tryGetValue parsedFlags flag.Id |> Option.defaultValue null) then
                presentFlags.Add(flag.Id) |> ignore

        for flagId in presentFlags do
            let flag = byId.[flagId]

            for otherId in flag.ConflictsWith do
                if presentFlags.Contains(otherId) then
                    let ordered =
                        if Core.ordinal.Compare(flagId, otherId) <= 0 then
                            flagId, otherId
                        else
                            otherId, flagId

                    let key = fst ordered + "\000" + snd ordered

                    if reportedConflicts.Add(key) then
                        errors.Add(
                            { ErrorType = "conflicting_flags"
                              Message = sprintf "%s and %s cannot be used together." (displayFlag flag) (displayFlag byId.[otherId])
                              Suggestion = null
                              Context = context })

            for requiredFlag in requiresGraph.TransitiveClosure(flagId) do
                if not (presentFlags.Contains(requiredFlag)) then
                    errors.Add(
                        { ErrorType = "missing_dependency_flag"
                          Message = sprintf "%s requires %s." (displayFlag flag) (displayFlag byId.[requiredFlag])
                          Suggestion = null
                          Context = context })

        for flag in activeFlags do
            if flag.Required && not (presentFlags.Contains(flag.Id)) && not (flag.RequiredUnless |> Seq.exists presentFlags.Contains) then
                errors.Add(
                    { ErrorType = "missing_required_flag"
                      Message = sprintf "%s is required." (displayFlag flag)
                      Suggestion = null
                      Context = context })

        for group in exclusiveGroups do
            let presentInGroup = group.FlagIds |> Seq.filter presentFlags.Contains |> Seq.toList

            if presentInGroup.Length > 1 then
                errors.Add(
                    { ErrorType = "exclusive_group_violation"
                      Message = sprintf "Only one of %s may be used." (presentInGroup |> Seq.map (fun id -> displayFlag byId.[id]) |> String.concat ", ")
                      Suggestion = null
                      Context = context })

            if group.Required && presentInGroup.IsEmpty then
                errors.Add(
                    { ErrorType = "missing_exclusive_group"
                      Message = sprintf "One of %s is required." (group.FlagIds |> Seq.map (fun id -> displayFlag byId.[id]) |> String.concat ", ")
                      Suggestion = null
                      Context = context })

        errors

type HelpGenerator(spec: CliSpec, commandSegments: seq<string>) =
    let commandSegments = commandSegments |> Core.toReadOnlyList

    let resolveCommand () =
        let mutable commands = spec.Commands
        let mutable current: CommandDef option = None

        for segment in commandSegments do
            let next =
                commands
                |> Seq.tryFind (fun command -> command.Name = segment || command.Aliases |> Seq.exists ((=) segment))

            current <- next

            match next with
            | Some command ->
                commands <- command.Commands
            | None ->
                ()

        current

    let displayArgument (argument: ArgDef) =
        let label = if argument.Variadic then sprintf "%s..." argument.DisplayName else argument.DisplayName
        if argument.Required then sprintf "<%s>" label else sprintf "[%s]" label

    let buildSignature (flag: FlagDef) =
        let usesValue = flag.Type <> ValueType.Boolean && flag.Type <> ValueType.Count
        let parts = ResizeArray<string>()

        if not (String.IsNullOrWhiteSpace(flag.Short)) then
            parts.Add(sprintf "-%s" flag.Short)

        if not (String.IsNullOrWhiteSpace(flag.Long)) then
            parts.Add(
                if usesValue then
                    sprintf "--%s <%s>" flag.Long (if String.IsNullOrWhiteSpace(flag.ValueName) then flag.Type.ToString().ToUpperInvariant() else flag.ValueName)
                else
                    sprintf "--%s" flag.Long)

        if not (String.IsNullOrWhiteSpace(flag.SingleDashLong)) then
            parts.Add(
                if usesValue then
                    sprintf "-%s <%s>" flag.SingleDashLong (if String.IsNullOrWhiteSpace(flag.ValueName) then flag.Type.ToString().ToUpperInvariant() else flag.ValueName)
                else
                    sprintf "-%s" flag.SingleDashLong)

        String.Join(", ", parts)

    let buildDescription (flag: FlagDef) =
        if not (isNull flag.Default) && not flag.Required && flag.Type <> ValueType.Boolean && flag.Type <> ValueType.Count then
            sprintf "%s [default: %O]" flag.Description flag.Default
        else
            flag.Description

    let buildFlagLines (flags: seq<FlagDef>) =
        let entries =
            flags
            |> Seq.map (fun flag -> buildSignature flag, buildDescription flag)
            |> Seq.toList

        let width =
            if List.isEmpty entries then
                0
            else
                entries |> Seq.map fst |> Seq.map String.length |> Seq.max

        entries |> Seq.map (fun (signature, description) -> sprintf "%s%s%s" signature (String(' ', width - signature.Length + 4)) description)

    let builtinFlag id shortName longName description =
        { Id = id
          Short = shortName
          Long = longName
          SingleDashLong = null
          Description = description
          Type = ValueType.Boolean
          Required = false
          Default = null
          ValueName = null
          EnumValues = Core.emptyReadOnly
          DefaultWhenPresent = null
          ConflictsWith = Core.emptyReadOnly
          Requires = Core.emptyReadOnly
          RequiredUnless = Core.emptyReadOnly
          Repeatable = false }

    member _.Generate() =
        let command = resolveCommand ()
        let lines = ResizeArray<string>()
        lines.Add("USAGE")

        let usageLine =
            let parts = ResizeArray<string>()
            parts.Add(spec.Name)
            for segment in commandSegments do
                parts.Add(segment)

            let flags =
                (match command with
                 | Some commandDef -> commandDef.Flags.Count
                 | None -> spec.Flags.Count)
                + spec.GlobalFlags.Count

            if flags > 0 || spec.BuiltinFlags.Help then
                parts.Add("[OPTIONS]")

            if (match command with | Some commandDef -> commandDef.Commands.Count | None -> spec.Commands.Count) > 0 then
                parts.Add("[COMMAND]")

            let usageArguments =
                match command with
                | Some commandDef -> commandDef.Arguments
                | None -> spec.Arguments

            for argument in usageArguments do
                parts.Add(displayArgument argument)

            String.Join(" ", parts)

        lines.Add(sprintf "  %s" usageLine)
        lines.Add(String.Empty)
        lines.Add("DESCRIPTION")
        lines.Add(sprintf "  %s" (match command with | Some commandDef -> commandDef.Description | None -> spec.Description))

        let commands = match command with | Some commandDef -> commandDef.Commands | None -> spec.Commands
        if commands.Count > 0 then
            lines.Add(String.Empty)
            lines.Add("COMMANDS")
            let width = commands |> Seq.map (fun item -> item.Name.Length) |> Seq.max

            for child in commands do
                lines.Add(sprintf "  %s%s%s" child.Name (String(' ', width - child.Name.Length + 2)) child.Description)

        let localFlags = match command with | Some commandDef -> commandDef.Flags | None -> spec.Flags
        if localFlags.Count > 0 then
            lines.Add(String.Empty)
            lines.Add("OPTIONS")
            for line in buildFlagLines localFlags do
                lines.Add(sprintf "  %s" line)

        let arguments = match command with | Some commandDef -> commandDef.Arguments | None -> spec.Arguments
        if arguments.Count > 0 then
            lines.Add(String.Empty)
            lines.Add("ARGUMENTS")

            for argument in arguments do
                let mutable suffix = if argument.Required then "Required." else "Optional."
                if argument.Variadic then
                    suffix <- suffix + " Repeatable."

                lines.Add(sprintf "  %s%s%s. %s" (displayArgument argument) (String(' ', 18 - min 18 (displayArgument argument).Length)) argument.Description suffix)

        let globalFlags = ResizeArray<FlagDef>(spec.GlobalFlags)
        if spec.BuiltinFlags.Help then
            globalFlags.Add(builtinFlag "help" "h" "help" "Show this help message and exit.")

        if spec.BuiltinFlags.Version && not (String.IsNullOrWhiteSpace(spec.Version)) then
            globalFlags.Add(builtinFlag "version" null "version" "Show version and exit.")

        if globalFlags.Count > 0 then
            lines.Add(String.Empty)
            lines.Add("GLOBAL OPTIONS")
            for line in buildFlagLines globalFlags do
                lines.Add(sprintf "  %s" line)

        String.Join("\n", lines)

type private ParserExitResult(result: ParserResult) =
    inherit Exception("Parser exited with a non-error result.")

    member _.Result = result

type Parser private (argv: IReadOnlyList<string>, spec: CliSpec option, loader: SpecLoader option) =
    new (specFilePath: string, argv: seq<string>) = Parser(Core.toReadOnlyList argv, None, Some(SpecLoader(specFilePath)))
    new (spec: CliSpec, argv: seq<string>) = Parser(Core.toReadOnlyList argv, Some(spec), None)

    static member private BuildActiveFlags(spec: CliSpec, command: CommandDef option) =
        let globalFlags =
            match command with
            | Some commandDef when not commandDef.InheritGlobalFlags -> Seq.empty
            | _ -> spec.GlobalFlags :> seq<FlagDef>

        let localFlags =
            match command with
            | Some commandDef -> commandDef.Flags :> seq<FlagDef>
            | None -> spec.Flags :> seq<FlagDef>

        Seq.append globalFlags localFlags |> Core.toReadOnlyList

    static member private InitializeFlagValues(flags: seq<FlagDef>) =
        let values = Dictionary<string, obj>(Core.ordinal)

        for flag in flags do
            values.[flag.Id] <-
                if flag.Repeatable then
                    box (ResizeArray<obj>())
                else
                    match flag.Type with
                    | ValueType.Boolean -> box false
                    | ValueType.Count -> box 0L
                    | _ -> flag.Default

        values

    static member private PreserveMatchingFlagValues(previousValues: IReadOnlyDictionary<string, obj>, activeFlags: seq<FlagDef>) =
        let nextValues = Parser.InitializeFlagValues(activeFlags)

        for flag in activeFlags do
            match Core.tryGetValue previousValues flag.Id with
            | Some value -> nextValues.[flag.Id] <- value
            | None -> ()

        nextValues

    static member private BuiltinFlagsFor(spec: CliSpec) =
        seq {
            if spec.BuiltinFlags.Help then
                yield
                    { Id = "__builtin_help"
                      Short = "h"
                      Long = "help"
                      SingleDashLong = null
                      Description = "Show help."
                      Type = ValueType.Boolean
                      Required = false
                      Default = null
                      ValueName = null
                      EnumValues = Core.emptyReadOnly
                      DefaultWhenPresent = null
                      ConflictsWith = Core.emptyReadOnly
                      Requires = Core.emptyReadOnly
                      RequiredUnless = Core.emptyReadOnly
                      Repeatable = false }

            if spec.BuiltinFlags.Version && not (String.IsNullOrWhiteSpace(spec.Version)) then
                yield
                    { Id = "__builtin_version"
                      Short = null
                      Long = "version"
                      SingleDashLong = null
                      Description = "Show version."
                      Type = ValueType.Boolean
                      Required = false
                      Default = null
                      ValueName = null
                      EnumValues = Core.emptyReadOnly
                      DefaultWhenPresent = null
                      ConflictsWith = Core.emptyReadOnly
                      Requires = Core.emptyReadOnly
                      RequiredUnless = Core.emptyReadOnly
                      Repeatable = false }
        }

    static member private ResolveCommand(token: string, commands: IReadOnlyList<CommandDef>) =
        commands |> Seq.tryFind (fun candidate -> candidate.Name = token || candidate.Aliases |> Seq.exists ((=) token))

    static member private FindFlag(predicate: FlagDef -> bool, flags: IReadOnlyList<FlagDef>) =
        flags |> Seq.tryFind predicate

    static member private DisplayFlag(flag: FlagDef) =
        if not (String.IsNullOrWhiteSpace(flag.Long)) then
            sprintf "--%s" flag.Long
        elif not (String.IsNullOrWhiteSpace(flag.Short)) then
            sprintf "-%s" flag.Short
        else
            sprintf "-%s" flag.SingleDashLong

    static member private UnknownFlag(program: string, commandSegments: IReadOnlyList<string>, rawFlag: string) =
        let context = seq { yield program; yield! commandSegments } |> Core.toReadOnlyList

        ParseErrors(
            Core.toReadOnlyList
                [ { ErrorType = "unknown_flag"
                    Message = sprintf "Unknown flag '%s'." rawFlag
                    Suggestion = null
                    Context = context } ])

    static member private ApplyFlagPresence(flag: FlagDef, parsedFlags: IDictionary<string, obj>, explicitFlags: ResizeArray<string>, context: IReadOnlyList<string>, duplicateAsError: bool) =
        if flag.Type = ValueType.Count then
            let current =
                match Core.tryGetMutableValue parsedFlags flag.Id with
                | Some (:? int64 as currentCount) -> currentCount
                | Some (:? int as currentCount) -> int64 currentCount
                | _ -> 0L

            parsedFlags.[flag.Id] <- box (current + 1L)
            explicitFlags.Add(flag.Id)
        elif flag.Repeatable then
            let values = Core.getRepeatableValues parsedFlags flag.Id
            values.Add(box true)
            explicitFlags.Add(flag.Id)
        else
            if duplicateAsError && FlagPresence.isPresentFlag flag (Core.tryGetMutableValue parsedFlags flag.Id |> Option.defaultValue null) then
                raise (
                    ParseErrors(
                        Core.toReadOnlyList
                            [ { ErrorType = "duplicate_flag"
                                Message = sprintf "Flag %s was provided more than once." (Parser.DisplayFlag flag)
                                Suggestion = null
                                Context = context } ]))

            parsedFlags.[flag.Id] <- box true
            explicitFlags.Add(flag.Id)

    static member private ApplyFlagValue(flag: FlagDef, rawValue: string, parsedFlags: IDictionary<string, obj>, explicitFlags: ResizeArray<string>, context: IReadOnlyList<string>, duplicateAsError: bool) =
        let struct (value, error) = PositionalResolver.CoerceValue(rawValue, flag.Type, flag.Id, context, flag.EnumValues)

        match error with
        | Some parseError ->
            raise (ParseErrors(Core.toReadOnlyList [ parseError ]))
        | None ->
            if flag.Repeatable then
                let values = Core.getRepeatableValues parsedFlags flag.Id
                values.Add(value |> Option.defaultValue null)
                explicitFlags.Add(flag.Id)
            else
                if duplicateAsError && FlagPresence.isPresentFlag flag (Core.tryGetMutableValue parsedFlags flag.Id |> Option.defaultValue null) then
                    raise (
                        ParseErrors(
                            Core.toReadOnlyList
                                [ { ErrorType = "duplicate_flag"
                                    Message = sprintf "Flag %s was provided more than once." (Parser.DisplayFlag flag)
                                    Suggestion = null
                                    Context = context } ]))

                parsedFlags.[flag.Id] <- value |> Option.defaultValue null
                explicitFlags.Add(flag.Id)

    member _.Parse() =
        try
            let spec =
                match spec, loader with
                | Some cliSpec, _ -> cliSpec
                | None, Some specLoader -> specLoader.Load()
                | _ -> raise (SpecError("No CLI spec is available."))

            let program =
                if argv.Count > 0 then
                    argv.[0]
                else
                    spec.Name

            let commandSegments = ResizeArray<string>()
            let positionalTokens = ResizeArray<string>()
            let explicitFlags = ResizeArray<string>()
            let mutable currentCommand: CommandDef option = None
            let mutable currentFlags = Parser.BuildActiveFlags(spec, currentCommand)
            let mutable parsedFlags = Parser.InitializeFlagValues(currentFlags)
            let mutable routeFinalized = false
            let mutable endOfFlags = false
            let mutable pendingValueFlag: FlagDef option = None
            let mutable pendingValueContext = seq { yield program } |> Core.toReadOnlyList

            for index in 1 .. argv.Count - 1 do
                let mutable token = argv.[index]

                if spec.ParsingMode = ParsingMode.Traditional
                   && index = 1
                   && not (token.StartsWith("-", StringComparison.Ordinal))
                   && Parser.ResolveCommand(token, match currentCommand with | Some command -> command.Commands | None -> spec.Commands).IsNone then
                    token <- "-" + token

                match pendingValueFlag with
                | Some flag ->
                    Parser.ApplyFlagValue(flag, token, parsedFlags, explicitFlags, pendingValueContext, false)
                    pendingValueFlag <- None
                | None ->
                    match
                        if not endOfFlags
                           && not routeFinalized
                           && not (token.StartsWith("-", StringComparison.Ordinal)) then
                            Parser.ResolveCommand(token, match currentCommand with | Some command -> command.Commands | None -> spec.Commands)
                        else
                            None
                    with
                    | Some resolvedCommand ->
                        commandSegments.Add(resolvedCommand.Name)
                        currentCommand <- Some resolvedCommand
                        currentFlags <- Parser.BuildActiveFlags(spec, currentCommand)
                        parsedFlags <- Parser.PreserveMatchingFlagValues(parsedFlags, currentFlags)
                    | None ->
                        let classifier = TokenClassifier(seq { yield! currentFlags; yield! Parser.BuiltinFlagsFor(spec) })
                        let tokenEvent =
                            if endOfFlags then
                                PositionalToken(token) :> TokenEvent
                            else
                                classifier.Classify(token)

                        match tokenEvent with
                        | :? EndOfFlagsToken ->
                            endOfFlags <- true
                            routeFinalized <- true
                        | :? PositionalToken as positionalToken ->
                            routeFinalized <- true
                            positionalTokens.Add(positionalToken.Value)

                            if spec.ParsingMode = ParsingMode.Posix then
                                endOfFlags <- true

                            if spec.ParsingMode = ParsingMode.SubcommandFirst
                               && commandSegments.Count = 0
                               && ((spec.Commands.Count > 0) || (match currentCommand with | Some command -> command.Commands.Count > 0 | None -> false)) then
                                raise (
                                    ParseErrors(
                                        Core.toReadOnlyList
                                            [ { ErrorType = "unknown_command"
                                                Message = sprintf "Unknown command \"%s\"." positionalToken.Value
                                                Suggestion = null
                                                Context = Core.toReadOnlyList [ program ] } ]))
                        | :? LongFlagToken as longFlagToken when longFlagToken.Name = "help" ->
                            if spec.BuiltinFlags.Help then
                                let commandPath = seq { yield program; yield! commandSegments } |> Core.toReadOnlyList
                                raise (ParserExitResult(HelpResult(HelpGenerator(spec, commandSegments).Generate(), commandPath) :> ParserResult))
                            else
                                positionalTokens.Add(token)
                        | :? ShortFlagToken as shortFlagToken when shortFlagToken.Char = "h" ->
                            if spec.BuiltinFlags.Help then
                                let commandPath = seq { yield program; yield! commandSegments } |> Core.toReadOnlyList
                                raise (ParserExitResult(HelpResult(HelpGenerator(spec, commandSegments).Generate(), commandPath) :> ParserResult))
                            else
                                positionalTokens.Add(token)
                        | :? LongFlagToken as longFlagToken when longFlagToken.Name = "version" ->
                            if spec.BuiltinFlags.Version && not (String.IsNullOrWhiteSpace(spec.Version)) then
                                raise (ParserExitResult(VersionResult(spec.Version) :> ParserResult))
                            else
                                positionalTokens.Add(token)
                        | :? LongFlagWithValueToken as longFlagWithValue ->
                            match Parser.FindFlag((fun flag -> String.Equals(flag.Long, longFlagWithValue.Name, StringComparison.Ordinal)), currentFlags) with
                            | Some longFlag ->
                                Parser.ApplyFlagValue(longFlag, longFlagWithValue.Value, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, true)
                                routeFinalized <- true
                            | None ->
                                raise (Parser.UnknownFlag(program, commandSegments, sprintf "--%s" longFlagWithValue.Name))
                        | :? LongFlagToken as longFlagToken ->
                            match Parser.FindFlag((fun flag -> String.Equals(flag.Long, longFlagToken.Name, StringComparison.Ordinal)), currentFlags) with
                            | Some longFlagDefinition ->
                                if longFlagDefinition.Type = ValueType.Boolean || longFlagDefinition.Type = ValueType.Count then
                                    Parser.ApplyFlagPresence(longFlagDefinition, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, true)
                                else
                                    let nextTokenIsMissing = index = argv.Count - 1 || argv.[index + 1].StartsWith("-", StringComparison.Ordinal)

                                    if not (isNull longFlagDefinition.DefaultWhenPresent) && nextTokenIsMissing then
                                        Parser.ApplyFlagValue(longFlagDefinition, longFlagDefinition.DefaultWhenPresent, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, true)
                                    else
                                        pendingValueFlag <- Some longFlagDefinition
                                        pendingValueContext <- seq { yield program; yield! commandSegments } |> Core.toReadOnlyList

                                routeFinalized <- true
                            | None ->
                                raise (Parser.UnknownFlag(program, commandSegments, sprintf "--%s" longFlagToken.Name))
                        | :? SingleDashLongToken as singleDashLongToken ->
                            match Parser.FindFlag((fun flag -> String.Equals(flag.SingleDashLong, singleDashLongToken.Name, StringComparison.Ordinal)), currentFlags) with
                            | Some singleDashLongFlag ->
                                if singleDashLongFlag.Type = ValueType.Boolean || singleDashLongFlag.Type = ValueType.Count then
                                    Parser.ApplyFlagPresence(singleDashLongFlag, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, true)
                                else
                                    pendingValueFlag <- Some singleDashLongFlag
                                    pendingValueContext <- seq { yield program; yield! commandSegments } |> Core.toReadOnlyList

                                routeFinalized <- true
                            | None ->
                                raise (Parser.UnknownFlag(program, commandSegments, sprintf "-%s" singleDashLongToken.Name))
                        | :? ShortFlagWithValueToken as shortFlagWithValue ->
                            match Parser.FindFlag((fun flag -> String.Equals(flag.Short, shortFlagWithValue.Char, StringComparison.Ordinal)), currentFlags) with
                            | Some shortFlag ->
                                Parser.ApplyFlagValue(shortFlag, shortFlagWithValue.Value, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, true)
                                routeFinalized <- true
                            | None ->
                                raise (Parser.UnknownFlag(program, commandSegments, sprintf "-%s" shortFlagWithValue.Char))
                        | :? ShortFlagToken as shortFlagToken ->
                            match Parser.FindFlag((fun flag -> String.Equals(flag.Short, shortFlagToken.Char, StringComparison.Ordinal)), currentFlags) with
                            | Some shortFlagDefinition ->
                                if shortFlagDefinition.Type = ValueType.Boolean || shortFlagDefinition.Type = ValueType.Count then
                                    Parser.ApplyFlagPresence(shortFlagDefinition, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, true)
                                else
                                    pendingValueFlag <- Some shortFlagDefinition
                                    pendingValueContext <- seq { yield program; yield! commandSegments } |> Core.toReadOnlyList

                                routeFinalized <- true
                            | None ->
                                raise (Parser.UnknownFlag(program, commandSegments, sprintf "-%s" shortFlagToken.Char))
                        | :? StackedFlagsToken as stackedFlagsToken ->
                            for shortName in stackedFlagsToken.Chars do
                                match Parser.FindFlag((fun flag -> String.Equals(flag.Short, shortName, StringComparison.Ordinal)), currentFlags) with
                                | Some stackedFlag when stackedFlag.Type <> ValueType.Boolean && stackedFlag.Type <> ValueType.Count ->
                                    raise (
                                        ParseErrors(
                                            Core.toReadOnlyList
                                                [ { ErrorType = "invalid_stack"
                                                    Message = sprintf "Flag -%s cannot appear inside a stacked flag token." shortName
                                                    Suggestion = null
                                                    Context = seq { yield program; yield! commandSegments } |> Core.toReadOnlyList } ]))
                                | Some stackedFlag ->
                                    Parser.ApplyFlagPresence(stackedFlag, parsedFlags, explicitFlags, seq { yield program; yield! commandSegments } |> Core.toReadOnlyList, false)
                                | None ->
                                    raise (Parser.UnknownFlag(program, commandSegments, sprintf "-%s" shortName))

                            routeFinalized <- true
                        | :? UnknownFlagToken as unknownFlagToken ->
                            raise (Parser.UnknownFlag(program, commandSegments, unknownFlagToken.Raw))
                        | _ ->
                            ()

            match pendingValueFlag with
            | Some flag ->
                raise (
                    ParseErrors(
                        Core.toReadOnlyList
                            [ { ErrorType = "invalid_value"
                                Message = sprintf "Flag %s expects a value." (Parser.DisplayFlag flag)
                                Suggestion = null
                                Context = pendingValueContext } ]))
            | None ->
                let context = seq { yield program; yield! commandSegments } |> Core.toReadOnlyList
                let activeArguments = match currentCommand with | Some command -> command.Arguments | None -> spec.Arguments
                let activeGroups = match currentCommand with | Some command -> command.MutuallyExclusiveGroups | None -> spec.MutuallyExclusiveGroups
                let struct (resolvedArguments, positionalErrors) = PositionalResolver(activeArguments).Resolve(positionalTokens, parsedFlags, context)
                let validationErrors = FlagValidator(currentFlags, activeGroups).Validate(parsedFlags, context)
                let allErrors = Seq.append positionalErrors validationErrors |> Core.toReadOnlyList

                if allErrors.Count > 0 then
                    raise (ParseErrors(allErrors))

                ParseResult(program, context, parsedFlags, resolvedArguments, explicitFlags) :> ParserResult
        with
        | :? ParserExitResult as exit ->
            exit.Result
