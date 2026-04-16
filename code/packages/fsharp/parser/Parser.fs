namespace CodingAdventures.Parser.FSharp

open System
open System.Collections.Generic
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Lexer.FSharp

type GrammarParseError(message: string, ?token: Token) =
    inherit
        Exception(
            match token with
            | Some value -> sprintf "Parse error at %d:%d: %s" value.Line value.Column message
            | None -> sprintf "Parse error: %s" message)

    member _.Token = defaultArg token Unchecked.defaultof<Token>

type ASTNode(ruleName: string, children: IReadOnlyList<obj>, ?startLine: int, ?startColumn: int, ?endLine: int, ?endColumn: int) =
    let startLine = defaultArg startLine 0
    let startColumn = defaultArg startColumn 0
    let endLine = defaultArg endLine 0
    let endColumn = defaultArg endColumn 0

    new (ruleName: string) = ASTNode(ruleName, ResizeArray<obj>() :> IReadOnlyList<obj>)

    member _.RuleName = ruleName
    member _.Children = children
    member _.StartLine = startLine
    member _.StartColumn = startColumn
    member _.EndLine = endLine
    member _.EndColumn = endColumn
    member _.IsLeaf = children.Count = 1 && children.[0] :? Token
    member this.Token = if this.IsLeaf then children.[0] :?> Token else Unchecked.defaultof<Token>

    member this.DescendantCount() =
        children
        |> Seq.sumBy (fun child ->
            match child with
            | :? ASTNode as node -> 1 + node.DescendantCount()
            | _ -> 1)

type private MemoEntry =
    {
        Children: IReadOnlyList<obj> option
        EndPos: int
        Ok: bool
    }

type private MatchResult =
    {
        Children: IReadOnlyList<obj>
        EndPos: int
    }

type GrammarParser(grammar: ParserGrammar) =
    let ruleMap =
        let result = Dictionary<string, GrammarRule>(StringComparer.Ordinal)
        for rule in grammar.Rules do
            result.[rule.Name] <- rule
        result

    static member private AsReadOnly(items: seq<obj>) =
        ResizeArray<obj>(items) :> IReadOnlyList<obj>

    static member private FindFirstToken(children: IReadOnlyList<obj>) =
        children
        |> Seq.tryPick (fun child ->
            match child with
            | :? Token as token -> Some token
            | :? ASTNode as node -> GrammarParser.FindFirstToken(node.Children)
            | _ -> None)

    static member private FindLastToken(children: IReadOnlyList<obj>) =
        children
        |> Seq.toArray
        |> Array.rev
        |> Array.tryPick (fun child ->
            match child with
            | :? Token as token -> Some token
            | :? ASTNode as node -> GrammarParser.FindLastToken(node.Children)
            | _ -> None)

    static member private BuildNode(ruleName: string, children: IReadOnlyList<obj>) =
        let first = GrammarParser.FindFirstToken(children)
        let last = GrammarParser.FindLastToken(children)
        ASTNode(
            ruleName,
            children,
            first |> Option.map (fun token -> token.Line) |> Option.defaultValue 0,
            first |> Option.map (fun token -> token.Column) |> Option.defaultValue 0,
            last |> Option.map (fun token -> token.Line) |> Option.defaultValue 0,
            last |> Option.map (fun token -> token.Column) |> Option.defaultValue 0)

    member this.Parse(tokens: IReadOnlyList<Token>) =
        if grammar.Rules.Count = 0 then
            raise (GrammarParseError("No rules in grammar", if tokens.Count > 0 then tokens.[tokens.Count - 1] else Unchecked.defaultof<Token>))

        let startRule = grammar.Rules.[0].Name
        let memo = Dictionary<struct (string * int), MemoEntry>()
        let recursion = HashSet<struct (string * int)>()

        match this.MatchRule(startRule, tokens, 0, memo, recursion) with
        | None ->
            raise (GrammarParseError(sprintf "Failed to parse starting rule '%s'" startRule, if tokens.Count > 0 then tokens.[0] else Unchecked.defaultof<Token>))
        | Some result ->
            if result.Children.Count = 1 && result.Children.[0] :? ASTNode then
                let node = result.Children.[0] :?> ASTNode
                if node.RuleName = startRule then node else GrammarParser.BuildNode(startRule, result.Children)
            else
                GrammarParser.BuildNode(startRule, result.Children)

    member private this.MatchRule(ruleName: string, tokens: IReadOnlyList<Token>, pos: int, memo: IDictionary<struct (string * int), MemoEntry>, recursion: HashSet<struct (string * int)>) =
        match memo.TryGetValue(struct (ruleName, pos)) with
        | true, cached ->
            if cached.Ok then
                Some { Children = cached.Children.Value; EndPos = cached.EndPos }
            else
                None
        | _ when not (recursion.Add(struct (ruleName, pos))) ->
            memo.[struct (ruleName, pos)] <- { Children = None; EndPos = pos; Ok = false }
            None
        | _ ->
            try
                match ruleMap.TryGetValue(ruleName) with
                | false, _ ->
                    memo.[struct (ruleName, pos)] <- { Children = None; EndPos = pos; Ok = false }
                    None
                | true, rule ->
                    match this.MatchElement(rule.Body, tokens, pos, memo, recursion) with
                    | None ->
                        memo.[struct (ruleName, pos)] <- { Children = None; EndPos = pos; Ok = false }
                        None
                    | Some result ->
                        let wrapped = GrammarParser.AsReadOnly [ box (GrammarParser.BuildNode(ruleName, result.Children)) ]
                        memo.[struct (ruleName, pos)] <- { Children = Some wrapped; EndPos = result.EndPos; Ok = true }
                        Some { Children = wrapped; EndPos = result.EndPos }
            finally
                recursion.Remove(struct (ruleName, pos)) |> ignore

    member private this.MatchRepeated(element: GrammarElement, tokens: IReadOnlyList<Token>, pos: int, memo: IDictionary<struct (string * int), MemoEntry>, recursion: HashSet<struct (string * int)>, requireOne: bool) =
        let children = ResizeArray<obj>()
        let mutable currentPos = pos
        let mutable matchedOne = false
        let mutable keepGoing = true

        while keepGoing do
            match this.MatchElement(element, tokens, currentPos, memo, recursion) with
            | Some result when result.EndPos <> currentPos ->
                matchedOne <- true
                children.AddRange(result.Children)
                currentPos <- result.EndPos
            | _ ->
                keepGoing <- false

        if requireOne && not matchedOne then
            None
        else
            Some { Children = children :> IReadOnlyList<obj>; EndPos = currentPos }

    member private this.MatchElement(element: GrammarElement, tokens: IReadOnlyList<Token>, pos: int, memo: IDictionary<struct (string * int), MemoEntry>, recursion: HashSet<struct (string * int)>) =
        match element with
        | :? RuleReference as reference ->
            if reference.IsToken then
                if pos < tokens.Count && tokens.[pos].EffectiveTypeName = reference.Name then
                    Some { Children = GrammarParser.AsReadOnly [ box tokens.[pos] ]; EndPos = pos + 1 }
                else
                    None
            else
                this.MatchRule(reference.Name, tokens, pos, memo, recursion)
        | :? Literal as literal ->
            if pos < tokens.Count && tokens.[pos].Value = literal.Value then
                Some { Children = GrammarParser.AsReadOnly [ box tokens.[pos] ]; EndPos = pos + 1 }
            else
                None
        | :? Sequence as sequence ->
            let children = ResizeArray<obj>()
            let mutable currentPos = pos
            let mutable failed = false
            for child in sequence.Elements do
                if not failed then
                    match this.MatchElement(child, tokens, currentPos, memo, recursion) with
                    | Some result ->
                        children.AddRange(result.Children)
                        currentPos <- result.EndPos
                    | None ->
                        failed <- true

            if failed then None else Some { Children = children :> IReadOnlyList<obj>; EndPos = currentPos }
        | :? Alternation as alternation ->
            alternation.Choices |> Seq.tryPick (fun choice -> this.MatchElement(choice, tokens, pos, memo, recursion))
        | :? Repetition as repetition ->
            this.MatchRepeated(repetition.Element, tokens, pos, memo, recursion, false)
        | :? OneOrMoreRepetition as repetition ->
            this.MatchRepeated(repetition.Element, tokens, pos, memo, recursion, true)
        | :? Optional as optional ->
            match this.MatchElement(optional.Element, tokens, pos, memo, recursion) with
            | Some result -> Some result
            | None -> Some { Children = GrammarParser.AsReadOnly []; EndPos = pos }
        | :? Group as group ->
            this.MatchElement(group.Element, tokens, pos, memo, recursion)
        | :? PositiveLookahead as lookahead ->
            if this.MatchElement(lookahead.Element, tokens, pos, memo, recursion) |> Option.isSome then
                Some { Children = GrammarParser.AsReadOnly []; EndPos = pos }
            else
                None
        | :? NegativeLookahead as lookahead ->
            if this.MatchElement(lookahead.Element, tokens, pos, memo, recursion) |> Option.isNone then
                Some { Children = GrammarParser.AsReadOnly []; EndPos = pos }
            else
                None
        | :? SeparatedRepetition as separated ->
            match this.MatchElement(separated.Element, tokens, pos, memo, recursion) with
            | None when separated.AtLeastOne -> None
            | None -> Some { Children = GrammarParser.AsReadOnly []; EndPos = pos }
            | Some first ->
                let children = ResizeArray<obj>(first.Children)
                let mutable currentPos = first.EndPos
                let mutable keepGoing = true

                while keepGoing do
                    match this.MatchElement(separated.Separator, tokens, currentPos, memo, recursion) with
                    | None ->
                        keepGoing <- false
                    | Some separatorResult ->
                        match this.MatchElement(separated.Element, tokens, separatorResult.EndPos, memo, recursion) with
                        | None ->
                            keepGoing <- false
                        | Some elementResult ->
                            children.AddRange(separatorResult.Children)
                            children.AddRange(elementResult.Children)
                            currentPos <- elementResult.EndPos

                Some { Children = children :> IReadOnlyList<obj>; EndPos = currentPos }
        | _ ->
            invalidOp (sprintf "Unsupported grammar element: %s" (element.GetType().Name))

module Parser =
    let parseGrammar source = ParserGrammarParser.Parse(source)
    let parse (tokens: IReadOnlyList<Token>) grammar = GrammarParser(grammar).Parse(tokens)
