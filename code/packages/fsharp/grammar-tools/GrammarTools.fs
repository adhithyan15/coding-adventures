namespace CodingAdventures.GrammarTools.FSharp

open System
open System.Collections.Generic
open System.Text

type TokenGrammarError(message: string, lineNumber: int) =
    inherit Exception(sprintf "Line %d: %s" lineNumber message)

    member _.LineNumber = lineNumber

type TokenDefinition(name: string, pattern: string, isRegex: bool, lineNumber: int, ?alias: string) =
    member _.Name = name
    member _.Pattern = pattern
    member _.IsRegex = isRegex
    member _.LineNumber = lineNumber
    member _.Alias = defaultArg alias null

type PatternGroup(name: string, definitions: IReadOnlyList<TokenDefinition>) =
    member _.Name = name
    member _.Definitions = definitions

type TokenGrammar
    (
        definitions: IReadOnlyList<TokenDefinition>,
        keywords: IReadOnlyList<string>,
        ?mode: string,
        ?escapeMode: string,
        ?skipDefinitions: IReadOnlyList<TokenDefinition>,
        ?reservedKeywords: IReadOnlyList<string>,
        ?groups: IReadOnlyDictionary<string, PatternGroup>,
        ?caseSensitive: bool,
        ?version: int,
        ?caseInsensitive: bool,
        ?contextKeywords: IReadOnlyList<string>,
        ?softKeywords: IReadOnlyList<string>,
        ?errorDefinitions: IReadOnlyList<TokenDefinition>
    ) =
    let emptyDefinitions = Array.Empty<TokenDefinition>() :> IReadOnlyList<TokenDefinition>
    let emptyStrings = Array.Empty<string>() :> IReadOnlyList<string>
    let emptyGroups =
        Dictionary<string, PatternGroup>(StringComparer.Ordinal) :> IReadOnlyDictionary<string, PatternGroup>

    member _.Definitions = definitions
    member _.Keywords = keywords
    member _.Mode = defaultArg mode null
    member _.EscapeMode = defaultArg escapeMode null
    member _.SkipDefinitions = defaultArg skipDefinitions emptyDefinitions
    member _.ReservedKeywords = defaultArg reservedKeywords emptyStrings
    member _.Groups = defaultArg groups emptyGroups
    member _.CaseSensitive = defaultArg caseSensitive true
    member _.Version = defaultArg version 0
    member _.CaseInsensitive = defaultArg caseInsensitive false
    member _.ContextKeywords = defaultArg contextKeywords emptyStrings
    member _.SoftKeywords = defaultArg softKeywords emptyStrings
    member _.ErrorDefinitions = defaultArg errorDefinitions emptyDefinitions

[<AbstractClass>]
type GrammarElement() = class end

type RuleReference(name: string, isToken: bool) =
    inherit GrammarElement()

    member _.Name = name
    member _.IsToken = isToken

type Literal(value: string) =
    inherit GrammarElement()

    member _.Value = value

type Group(element: GrammarElement) =
    inherit GrammarElement()

    member _.Element = element

type Optional(element: GrammarElement) =
    inherit GrammarElement()

    member _.Element = element

type Repetition(element: GrammarElement) =
    inherit GrammarElement()

    member _.Element = element

type Alternation(choices: IReadOnlyList<GrammarElement>) =
    inherit GrammarElement()

    member _.Choices = choices

type Sequence(elements: IReadOnlyList<GrammarElement>) =
    inherit GrammarElement()

    member _.Elements = elements

type PositiveLookahead(element: GrammarElement) =
    inherit GrammarElement()

    member _.Element = element

type NegativeLookahead(element: GrammarElement) =
    inherit GrammarElement()

    member _.Element = element

type OneOrMoreRepetition(element: GrammarElement) =
    inherit GrammarElement()

    member _.Element = element

type SeparatedRepetition(element: GrammarElement, separator: GrammarElement, atLeastOne: bool) =
    inherit GrammarElement()

    member _.Element = element
    member _.Separator = separator
    member _.AtLeastOne = atLeastOne

type GrammarRule(name: string, body: GrammarElement) =
    member _.Name = name
    member _.Body = body

type ParserGrammar(?rules: IReadOnlyList<GrammarRule>) =
    let emptyRules = Array.Empty<GrammarRule>() :> IReadOnlyList<GrammarRule>

    member _.Rules = defaultArg rules emptyRules

type ParserGrammarError(message: string, lineNumber: int) =
    inherit Exception(sprintf "Line %d: %s" lineNumber message)

    member _.LineNumber = lineNumber

module private TokenGrammarParsing =
    let tryParseMagicComment (line: string) =
        let trimmed = line.Trim()
        if not (trimmed.StartsWith("# @", StringComparison.Ordinal)) then
            false, String.Empty, String.Empty
        else
            let rest = trimmed.Substring(3).Trim()
            let split = rest.Split([| ' ' |], 2, StringSplitOptions.RemoveEmptyEntries)
            if split.Length = 0 then
                false, String.Empty, String.Empty
            else
                let value = if split.Length = 2 then split[1].Trim() else String.Empty
                true, split[0], value

    let unescapeQuoted (text: string) =
        let result = StringBuilder(text.Length)
        let mutable i = 0

        while i < text.Length do
            if text[i] = '\\' && i + 1 < text.Length then
                i <- i + 1

                result.Append(
                    match text[i] with
                    | 'n' -> '\n'
                    | 't' -> '\t'
                    | 'r' -> '\r'
                    | '"' -> '"'
                    | '\\' -> '\\'
                    | ch -> ch)
                |> ignore
            else
                result.Append(text[i]) |> ignore

            i <- i + 1

        result.ToString()

    let findAliasMarker (rhs: string) =
        let mutable inRegex = false
        let mutable inString = false
        let mutable escaped = false
        let mutable found = -1
        let mutable index = 0

        while found < 0 && index < rhs.Length - 1 do
            let ch = rhs[index]

            if escaped then
                escaped <- false
            elif ch = '\\' then
                escaped <- true
            elif ch = '"' && not inRegex then
                inString <- not inString
            elif ch = '/' && not inString then
                inRegex <- not inRegex
            elif not inRegex && not inString && ch = '-' && rhs[index + 1] = '>' then
                found <- index

            index <- index + 1

        found

    let parseDefinition (line: string) lineNumber =
        let equalsIndex = line.IndexOf('=')
        if equalsIndex < 1 then
            raise (TokenGrammarError("Expected TOKEN_NAME = PATTERN", lineNumber))

        let name = line.Substring(0, equalsIndex).Trim()
        let mutable rhs = line.Substring(equalsIndex + 1).Trim()
        let mutable alias = None
        let aliasIndex = findAliasMarker rhs

        if aliasIndex >= 0 then
            alias <- Some(rhs.Substring(aliasIndex + 2).Trim())
            rhs <- rhs.Substring(0, aliasIndex).Trim()

        if rhs.StartsWith("/", StringComparison.Ordinal) && rhs.EndsWith("/", StringComparison.Ordinal) then
            TokenDefinition(name, rhs.Substring(1, rhs.Length - 2), true, lineNumber, ?alias = alias)
        elif rhs.StartsWith("\"", StringComparison.Ordinal) && rhs.EndsWith("\"", StringComparison.Ordinal) then
            TokenDefinition(name, unescapeQuoted (rhs.Substring(1, rhs.Length - 2)), false, lineNumber, ?alias = alias)
        else
            raise (TokenGrammarError("Pattern must be /regex/ or \"literal\"", lineNumber))

[<AbstractClass; Sealed>]
type TokenGrammarParser =
    static member Parse(source: string) =
        let definitions = ResizeArray<TokenDefinition>()
        let skipDefinitions = ResizeArray<TokenDefinition>()
        let errorDefinitions = ResizeArray<TokenDefinition>()
        let keywords = ResizeArray<string>()
        let reserved = ResizeArray<string>()
        let contextKeywords = ResizeArray<string>()
        let softKeywords = ResizeArray<string>()
        let groups = Dictionary<string, PatternGroup>(StringComparer.Ordinal)
        let groupDefinitions = Dictionary<string, ResizeArray<TokenDefinition>>(StringComparer.Ordinal)
        let mutable mode = None
        let mutable escapeMode = None
        let mutable version = 0
        let mutable caseInsensitive = false
        let mutable section = "definitions"
        let mutable currentGroup = None

        let lines = source.Replace("\r\n", "\n").Split('\n')

        for index = 0 to lines.Length - 1 do
            let lineNumber = index + 1
            let trimmed = lines[index].Trim()

            if trimmed.Length <> 0 then
                if trimmed.StartsWith("#", StringComparison.Ordinal) then
                    let ok, key, value = TokenGrammarParsing.tryParseMagicComment trimmed

                    if ok then
                        if key = "version" then
                            match Int32.TryParse(value) with
                            | true, parsedVersion -> version <- parsedVersion
                            | _ -> ()
                        elif key = "case_insensitive" then
                            match Boolean.TryParse(value) with
                            | true, parsedBool -> caseInsensitive <- parsedBool
                            | _ -> ()
                elif trimmed.StartsWith("mode:", StringComparison.Ordinal) then
                    mode <- Some(trimmed.Substring("mode:".Length).Trim())
                elif trimmed.StartsWith("escape_mode:", StringComparison.Ordinal) then
                    escapeMode <- Some(trimmed.Substring("escape_mode:".Length).Trim())
                elif trimmed.StartsWith("escapes:", StringComparison.Ordinal) then
                    escapeMode <- Some(trimmed.Substring("escapes:".Length).Trim())
                elif trimmed.StartsWith("case_sensitive:", StringComparison.Ordinal) then
                    match Boolean.TryParse(trimmed.Substring("case_sensitive:".Length).Trim()) with
                    | true, parsedBool -> caseInsensitive <- not parsedBool
                    | _ -> ()
                elif
                    trimmed = "keywords:"
                    || trimmed = "reserved:"
                    || trimmed = "context_keywords:"
                    || trimmed = "soft_keywords:"
                    || trimmed = "skip:"
                    || trimmed = "errors:"
                then
                    section <- trimmed.Substring(0, trimmed.Length - 1)
                    currentGroup <- None
                elif trimmed.StartsWith("group ", StringComparison.Ordinal) && trimmed.EndsWith(":", StringComparison.Ordinal) then
                    let groupName = trimmed.Substring(6, trimmed.Length - 7).Trim()

                    if groupName.Length = 0 then
                        raise (TokenGrammarError("Group name cannot be empty", lineNumber))

                    section <- "group"
                    currentGroup <- Some groupName
                    groupDefinitions[groupName] <- ResizeArray<TokenDefinition>()
                else
                    if
                        (section = "skip" || section = "errors" || section = "group")
                        && not (Char.IsWhiteSpace(lines[index][0]))
                        && trimmed.Contains("=")
                    then
                        section <- "definitions"
                        currentGroup <- None

                    match section with
                    | "keywords" -> keywords.Add(trimmed)
                    | "reserved" -> reserved.Add(trimmed)
                    | "context_keywords" -> contextKeywords.Add(trimmed)
                    | "soft_keywords" -> softKeywords.Add(trimmed)
                    | "definitions"
                    | "skip"
                    | "errors"
                    | "group" ->
                        let definition = TokenGrammarParsing.parseDefinition trimmed lineNumber

                        match section with
                        | "skip" -> skipDefinitions.Add(definition)
                        | "errors" -> errorDefinitions.Add(definition)
                        | "group" -> groupDefinitions[currentGroup.Value].Add(definition)
                        | _ -> definitions.Add(definition)
                    | _ -> raise (TokenGrammarError(sprintf "Unsupported section '%s'" section, lineNumber))

        for KeyValue(name, defs) in groupDefinitions do
            groups[name] <- PatternGroup(name, defs :> IReadOnlyList<TokenDefinition>)

        TokenGrammar(
            definitions :> IReadOnlyList<TokenDefinition>,
            keywords :> IReadOnlyList<string>,
            ?mode = mode,
            ?escapeMode = escapeMode,
            ?skipDefinitions = Some(skipDefinitions :> IReadOnlyList<TokenDefinition>),
            ?reservedKeywords = Some(reserved :> IReadOnlyList<string>),
            ?groups = Some(groups :> IReadOnlyDictionary<string, PatternGroup>),
            ?caseSensitive = Some(not caseInsensitive),
            ?version = Some(version),
            ?caseInsensitive = Some(caseInsensitive),
            ?contextKeywords = Some(contextKeywords :> IReadOnlyList<string>),
            ?softKeywords = Some(softKeywords :> IReadOnlyList<string>),
            ?errorDefinitions = Some(errorDefinitions :> IReadOnlyList<TokenDefinition>))

[<AbstractClass; Sealed>]
type TokenGrammarValidator =
    static member Validate(grammar: TokenGrammar) =
        let seen = HashSet<string>(StringComparer.Ordinal)

        for definition in grammar.Definitions do
            if not (seen.Add(definition.Name)) then
                raise (InvalidOperationException(sprintf "Duplicate token definition: %s" definition.Name))

module private ParserGrammarParsing =
    type Kind =
        | Identifier
        | String
        | Equals
        | Pipe
        | LBrace
        | RBrace
        | LBracket
        | RBracket
        | LParen
        | RParen
        | Semicolon
        | Ampersand
        | Bang
        | DoubleSlash
        | Plus
        | End

    type MetaToken =
        {
            Kind: Kind
            Text: string
            Line: int
        }

    let setOf (values: seq<Kind>) = HashSet<Kind>(values)

    type Parser(tokens: ResizeArray<MetaToken>) =
        let mutable pos = 0

        member private _.Peek() = tokens[pos]

        member private this.Match(kind: Kind) =
            if this.Peek().Kind = kind then
                let token = tokens[pos]
                pos <- pos + 1
                Some token
            else
                None

        member private this.Expect(kind: Kind) =
            match this.Match(kind) with
            | Some token -> token
            | None ->
                raise (ParserGrammarError(sprintf "Expected %A but found '%s'" kind (this.Peek().Text), this.Peek().Line))

        member this.Parse() =
            let rules = ResizeArray<GrammarRule>()

            while this.Peek().Kind <> Kind.End do
                let name = this.Expect(Kind.Identifier).Text
                this.Expect(Kind.Equals) |> ignore
                let body = this.ParseAlternation(setOf [ Kind.Semicolon ])
                this.Expect(Kind.Semicolon) |> ignore
                rules.Add(GrammarRule(name, body))

            ParserGrammar(rules :> IReadOnlyList<GrammarRule>)

        member private this.ParseAlternation(terminators: HashSet<Kind>) =
            let choices = ResizeArray<GrammarElement>()
            choices.Add(this.ParseSequence(setOf (seq { yield! terminators; yield Kind.Pipe })))

            while this.Match(Kind.Pipe).IsSome do
                choices.Add(this.ParseSequence(setOf (seq { yield! terminators; yield Kind.Pipe })))

            if choices.Count = 1 then
                choices[0]
            else
                Alternation(choices :> IReadOnlyList<GrammarElement>) :> GrammarElement

        member private this.ParseSequence(terminators: HashSet<Kind>) =
            let elements = ResizeArray<GrammarElement>()

            while not (terminators.Contains(this.Peek().Kind)) && this.Peek().Kind <> Kind.End do
                elements.Add(this.ParseElement())

            match elements.Count with
            | 0 -> Sequence(Array.Empty<GrammarElement>() :> IReadOnlyList<GrammarElement>) :> GrammarElement
            | 1 -> elements[0]
            | _ -> Sequence(elements :> IReadOnlyList<GrammarElement>) :> GrammarElement

        member private this.ParseElement() =
            match this.Match(Kind.Ampersand) with
            | Some _ -> PositiveLookahead(this.ParseElement()) :> GrammarElement
            | None ->
                match this.Match(Kind.Bang) with
                | Some _ -> NegativeLookahead(this.ParseElement()) :> GrammarElement
                | None ->
                    match this.Match(Kind.Identifier) with
                    | Some identifier ->
                        let isToken = identifier.Text |> Seq.forall (fun ch -> Char.IsUpper(ch) || ch = '_')
                        RuleReference(identifier.Text, isToken) :> GrammarElement
                    | None ->
                        match this.Match(Kind.String) with
                        | Some literal -> Literal(literal.Text) :> GrammarElement
                        | None ->
                            match this.Match(Kind.LParen) with
                            | Some _ ->
                                let inner = this.ParseAlternation(setOf [ Kind.RParen ])
                                this.Expect(Kind.RParen) |> ignore
                                Group(inner) :> GrammarElement
                            | None ->
                                match this.Match(Kind.LBracket) with
                                | Some _ ->
                                    let inner = this.ParseAlternation(setOf [ Kind.RBracket ])
                                    this.Expect(Kind.RBracket) |> ignore
                                    Optional(inner) :> GrammarElement
                                | None ->
                                    match this.Match(Kind.LBrace) with
                                    | Some _ ->
                                        let inner = this.ParseAlternation(setOf [ Kind.RBrace; Kind.DoubleSlash ])

                                        let repetition =
                                            match this.Match(Kind.DoubleSlash) with
                                            | Some _ ->
                                                let separator = this.ParseAlternation(setOf [ Kind.RBrace ])
                                                this.Expect(Kind.RBrace) |> ignore
                                                SeparatedRepetition(inner, separator, false) :> GrammarElement
                                            | None ->
                                                this.Expect(Kind.RBrace) |> ignore
                                                Repetition(inner) :> GrammarElement

                                        match this.Match(Kind.Plus) with
                                        | Some _ ->
                                            match repetition with
                                            | :? SeparatedRepetition as separated ->
                                                SeparatedRepetition(separated.Element, separated.Separator, true) :> GrammarElement
                                            | :? Repetition as repeated -> OneOrMoreRepetition(repeated.Element) :> GrammarElement
                                            | _ -> repetition
                                        | None -> repetition
                                    | None ->
                                        raise (ParserGrammarError(sprintf "Unexpected token '%s'" (this.Peek().Text), this.Peek().Line))

    let tokenize (source: string) =
        let tokens = ResizeArray<MetaToken>()
        let mutable line = 1
        let mutable i = 0

        while i < source.Length do
            let ch = source[i]

            if ch = '\r' then
                i <- i + 1
            elif ch = '\n' then
                line <- line + 1
                i <- i + 1
            elif Char.IsWhiteSpace(ch) then
                i <- i + 1
            elif ch = '#' then
                while i < source.Length && source[i] <> '\n' do
                    i <- i + 1
            elif Char.IsLetter(ch) || ch = '_' then
                let start = i

                while i < source.Length && (Char.IsLetterOrDigit(source[i]) || source[i] = '_' || source[i] = '-') do
                    i <- i + 1

                tokens.Add({ Kind = Kind.Identifier; Text = source.Substring(start, i - start); Line = line })
            elif ch = '"' then
                let builder = StringBuilder()
                let mutable closed = false
                i <- i + 1

                while i < source.Length && not closed do
                    if source[i] = '"' && source[i - 1] <> '\\' then
                        i <- i + 1
                        closed <- true
                    elif source[i] = '\\' && i + 1 < source.Length then
                        i <- i + 1

                        builder.Append(
                            match source[i] with
                            | 'n' -> '\n'
                            | 't' -> '\t'
                            | 'r' -> '\r'
                            | '"' -> '"'
                            | '\\' -> '\\'
                            | escaped -> escaped)
                        |> ignore

                        i <- i + 1
                    else
                        builder.Append(source[i]) |> ignore
                        i <- i + 1

                tokens.Add({ Kind = Kind.String; Text = builder.ToString(); Line = line })
            elif ch = '/' && i + 1 < source.Length && source[i + 1] = '/' then
                tokens.Add({ Kind = Kind.DoubleSlash; Text = "//"; Line = line })
                i <- i + 2
            else
                let token =
                    match ch with
                    | '=' -> { Kind = Kind.Equals; Text = "="; Line = line }
                    | '|' -> { Kind = Kind.Pipe; Text = "|"; Line = line }
                    | '{' -> { Kind = Kind.LBrace; Text = "{"; Line = line }
                    | '}' -> { Kind = Kind.RBrace; Text = "}"; Line = line }
                    | '[' -> { Kind = Kind.LBracket; Text = "["; Line = line }
                    | ']' -> { Kind = Kind.RBracket; Text = "]"; Line = line }
                    | '(' -> { Kind = Kind.LParen; Text = "("; Line = line }
                    | ')' -> { Kind = Kind.RParen; Text = ")"; Line = line }
                    | ';' -> { Kind = Kind.Semicolon; Text = ";"; Line = line }
                    | '&' -> { Kind = Kind.Ampersand; Text = "&"; Line = line }
                    | '!' -> { Kind = Kind.Bang; Text = "!"; Line = line }
                    | '+' -> { Kind = Kind.Plus; Text = "+"; Line = line }
                    | _ -> raise (ParserGrammarError(sprintf "Unexpected character '%c'" ch, line))

                tokens.Add(token)
                i <- i + 1

        tokens.Add({ Kind = Kind.End; Text = String.Empty; Line = line })
        tokens

[<AbstractClass; Sealed>]
type ParserGrammarParser =
    static member Parse(source: string) =
        ParserGrammarParsing.Parser(ParserGrammarParsing.tokenize source).Parse()

[<AbstractClass; Sealed>]
type ParserGrammarValidator =
    static member Validate(grammar: ParserGrammar) =
        let names = HashSet<string>(StringComparer.Ordinal)

        for rule in grammar.Rules do
            if not (names.Add(rule.Name)) then
                raise (InvalidOperationException(sprintf "Duplicate grammar rule: %s" rule.Name))

[<AbstractClass; Sealed>]
type CrossValidator =
    static member Validate(tokenGrammar: TokenGrammar, parserGrammar: ParserGrammar) =
        let tokenNames = HashSet<string>(StringComparer.Ordinal)

        for definition in tokenGrammar.Definitions do
            let exportedName =
                if String.IsNullOrEmpty(definition.Alias) then definition.Name else definition.Alias

            tokenNames.Add(exportedName) |> ignore

        if tokenGrammar.Keywords.Count > 0 then
            tokenNames.Add("KEYWORD") |> ignore

        let rec visit (element: GrammarElement) =
            match element with
            | :? RuleReference as reference when reference.IsToken && not (tokenNames.Contains(reference.Name)) ->
                raise (InvalidOperationException(sprintf "Unknown token reference: %s" reference.Name))
            | :? Group as group -> visit group.Element
            | :? Optional as optional -> visit optional.Element
            | :? Repetition as repetition -> visit repetition.Element
            | :? OneOrMoreRepetition as repetition -> visit repetition.Element
            | :? PositiveLookahead as lookahead -> visit lookahead.Element
            | :? NegativeLookahead as lookahead -> visit lookahead.Element
            | :? SeparatedRepetition as separated ->
                visit separated.Element
                visit separated.Separator
            | :? Alternation as alternation ->
                for choice in alternation.Choices do
                    visit choice
            | :? Sequence as sequence ->
                for child in sequence.Elements do
                    visit child
            | _ -> ()

        for rule in parserGrammar.Rules do
            visit rule.Body
