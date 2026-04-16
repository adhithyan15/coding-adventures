namespace CodingAdventures.Lexer.FSharp

open System
open System.Collections.Generic
open System.Text.RegularExpressions
open CodingAdventures.GrammarTools.FSharp

type TokenType =
    | Name = 0
    | Number = 1
    | String = 2
    | Keyword = 3
    | Plus = 4
    | Minus = 5
    | Star = 6
    | Slash = 7
    | Equals = 8
    | EqualsEquals = 9
    | LParen = 10
    | RParen = 11
    | Comma = 12
    | Colon = 13
    | Semicolon = 14
    | LBrace = 15
    | RBrace = 16
    | LBracket = 17
    | RBracket = 18
    | Dot = 19
    | Bang = 20
    | Newline = 21
    | EOF = 22
    | Grammar = 23

type Token(tokenType: TokenType, value: string, line: int, column: int, ?typeName: string, ?flags: int) =
    let typeName = defaultArg typeName null
    let flags = defaultArg flags 0

    static member FlagPrecededByNewline = 1
    static member FlagContextKeyword = 2

    member _.Type = tokenType
    member _.Value = value
    member _.Line = line
    member _.Column = column
    member _.TypeName = typeName
    member _.Flags = flags
    member this.EffectiveTypeName = if String.IsNullOrEmpty(typeName) then tokenType.ToString().ToUpperInvariant() else typeName
    member _.HasFlag(flag: int) = (flags &&& flag) <> 0

    override this.ToString() = sprintf "Token(%s, \"%s\", %d:%d)" this.EffectiveTypeName value line column

    override this.Equals(other: obj) =
        match other with
        | :? Token as token ->
            token.Type = tokenType
            && token.Value = value
            && token.Line = line
            && token.Column = column
            && token.TypeName = typeName
            && token.Flags = flags
        | _ ->
            false

    override _.GetHashCode() =
        HashCode.Combine(tokenType, value, line, column, typeName, flags)

type LexerError(message: string, line: int, column: int) =
    inherit Exception(sprintf "Lexer error at %d:%d: %s" line column message)

    member _.Line = line
    member _.Column = column

type private CompiledPattern =
    {
        Name: string
        Regex: Regex
        Alias: string
    }

type private MatcherNode =
    {
        Stage: string
        Pattern: CompiledPattern
    }

type GrammarLexer(grammar: TokenGrammar) =
    let definitionsOrEmpty (definitions: IReadOnlyList<TokenDefinition>) =
        if isNull definitions then Seq.empty else definitions :> seq<TokenDefinition>

    let compileDefinitions definitions =
        let mutable options = RegexOptions.Compiled ||| RegexOptions.CultureInvariant
        if not grammar.CaseSensitive then
            options <- options ||| RegexOptions.IgnoreCase

        definitions
        |> Seq.map (fun (definition: TokenDefinition) ->
            let pattern =
                if definition.IsRegex then
                    sprintf @"\G(?:%s)" definition.Pattern
                else
                    sprintf @"\G%s" (Regex.Escape(definition.Pattern))

            {
                Name = definition.Name
                Regex = Regex(pattern, options)
                Alias =
                    if String.IsNullOrEmpty(definition.Alias) then
                        null
                    else
                        definition.Alias
            })
        |> Seq.toList

    let matcherPipeline =
        [
            yield!
                compileDefinitions (definitionsOrEmpty grammar.SkipDefinitions)
                |> List.map (fun pattern -> { Stage = "skip"; Pattern = pattern })
            yield!
                compileDefinitions grammar.Definitions
                |> List.map (fun pattern -> { Stage = "token"; Pattern = pattern })
            yield!
                compileDefinitions (definitionsOrEmpty grammar.ErrorDefinitions)
                |> List.map (fun pattern -> { Stage = "error"; Pattern = pattern })
        ]

    let keywords = HashSet<string>(grammar.Keywords, StringComparer.Ordinal)
    let reserved =
        let values =
            if isNull grammar.ReservedKeywords then
                Seq.empty
            else
                grammar.ReservedKeywords :> seq<string>

        HashSet<string>(
            values,
            StringComparer.Ordinal)

    let contextKeywords =
        let values =
            if isNull grammar.ContextKeywords then
                Seq.empty
            else
                grammar.ContextKeywords :> seq<string>

        HashSet<string>(
            values,
            StringComparer.Ordinal)

    let normalizeCase (value: string) =
        if grammar.CaseSensitive then value else value.ToLowerInvariant()

    let advance (text: string) (line: byref<int>) (column: byref<int>) (precededByNewline: byref<bool>) =
        for ch in text do
            if ch = '\n' then
                line <- line + 1
                column <- 1
                precededByNewline <- true
            else
                column <- column + 1

    let promoteKeywords (tokens: ResizeArray<Token>) =
        if keywords.Count > 0 then
            for index in 0 .. tokens.Count - 1 do
                let token = tokens.[index]
                if token.TypeName = "NAME" && keywords.Contains(normalizeCase token.Value) then
                    tokens.[index] <- Token(TokenType.Keyword, token.Value, token.Line, token.Column, "KEYWORD", token.Flags)

    member _.Tokenize(source: string) =
        let workingSource = if grammar.CaseSensitive then source else source.ToLowerInvariant()
        let tokens = ResizeArray<Token>()
        let mutable pos = 0
        let mutable line = 1
        let mutable column = 1
        let mutable precededByNewline = false

        while pos < workingSource.Length do
            let mutable matched = false

            for node in matcherPipeline do
                if not matched then
                    let ``match`` = node.Pattern.Regex.Match(workingSource, pos)
                    if ``match``.Success && ``match``.Index = pos then
                        let value = source.Substring(pos, ``match``.Length)
                        match node.Stage with
                        | "skip" ->
                            advance value &line &column &precededByNewline
                            pos <- pos + value.Length
                        | _ ->
                            let typeName =
                                if String.IsNullOrEmpty(node.Pattern.Alias) then
                                    node.Pattern.Name
                                else
                                    node.Pattern.Alias

                            if typeName = "NAME" && reserved.Contains(value) then
                                raise (LexerError(sprintf "Reserved keyword '%s'" value, line, column))

                            let mutable tokenFlags = 0
                            if precededByNewline then
                                tokenFlags <- tokenFlags ||| Token.FlagPrecededByNewline

                            if typeName = "NAME" && contextKeywords.Contains(normalizeCase value) then
                                tokenFlags <- tokenFlags ||| Token.FlagContextKeyword

                            tokens.Add(Token(TokenType.Grammar, value, line, column, typeName, tokenFlags))

                            let mutable ignoredPrecededByNewline = false
                            advance value &line &column &ignoredPrecededByNewline
                            precededByNewline <- false
                            pos <- pos + value.Length

                        matched <- true

            if not matched then
                raise (LexerError(sprintf "Unexpected character '%c'" source.[pos], line, column))

        promoteKeywords tokens
        tokens.Add(Token(TokenType.EOF, String.Empty, line, column, "EOF"))
        tokens |> Seq.toList

module Lexer =
    let parseGrammar source = TokenGrammarParser.Parse(source)
    let tokenize source grammar = GrammarLexer(grammar).Tokenize(source)
