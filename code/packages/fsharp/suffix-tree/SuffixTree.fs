namespace CodingAdventures.SuffixTree

open System

/// Simple suffix tree facade with substring search helpers.
type SuffixTree private (text: string) =
    static let searchPositions (text: string) (pattern: string) =
        if pattern.Length = 0 then
            [ 0..text.Length ]
        elif pattern.Length > text.Length then
            []
        else
            [ for start in 0 .. text.Length - pattern.Length do
                  if String.CompareOrdinal(text, start, pattern, 0, pattern.Length) = 0 then
                      start ]

    static let allSuffixesIn (text: string) =
        [ for start in 0 .. text.Length - 1 -> text[start..] ]

    static let commonPrefix (left: string) (right: string) =
        let mutable index = 0
        let limit = min left.Length right.Length

        while index < limit && left[index] = right[index] do
            index <- index + 1

        left[.. index - 1]

    static let longestRepeatedSubstringIn (text: string) =
        let suffixes = allSuffixesIn text
        let mutable best = String.Empty

        for i in 0 .. suffixes.Length - 1 do
            for j in i + 1 .. suffixes.Length - 1 do
                let prefix = commonPrefix suffixes[i] suffixes[j]

                if prefix.Length > best.Length then
                    best <- prefix

        best

    static member Build(value: string) =
        if isNull value then
            nullArg (nameof value)

        SuffixTree(value)

    static member BuildUkkonen(value: string) =
        SuffixTree.Build(value)

    static member Search(tree: SuffixTree, pattern: string) =
        if isNull (box tree) then
            nullArg (nameof tree)

        tree.Search(pattern)

    static member CountOccurrences(tree: SuffixTree, pattern: string) =
        if isNull (box tree) then
            nullArg (nameof tree)

        tree.CountOccurrences(pattern)

    static member LongestRepeatedSubstring(tree: SuffixTree) =
        if isNull (box tree) then
            nullArg (nameof tree)

        tree.LongestRepeatedSubstring()

    static member LongestCommonSubstring(left: string, right: string) =
        if isNull left then
            nullArg (nameof left)

        if isNull right then
            nullArg (nameof right)

        if left.Length = 0 || right.Length = 0 then
            String.Empty
        else
            let dp = Array2D.zeroCreate<int> (left.Length + 1) (right.Length + 1)
            let mutable bestLength = 0
            let mutable bestEnd = 0

            for i in 1 .. left.Length do
                for j in 1 .. right.Length do
                    if left[i - 1] = right[j - 1] then
                        dp[i, j] <- dp[i - 1, j - 1] + 1

                        if dp[i, j] > bestLength then
                            bestLength <- dp[i, j]
                            bestEnd <- i

            left.Substring(bestEnd - bestLength, bestLength)

    static member AllSuffixes(tree: SuffixTree) =
        if isNull (box tree) then
            nullArg (nameof tree)

        tree.AllSuffixes()

    static member NodeCount(tree: SuffixTree) =
        if isNull (box tree) then
            nullArg (nameof tree)

        tree.NodeCount()

    member _.Text = text

    member _.Search(pattern: string) =
        if isNull pattern then
            nullArg (nameof pattern)

        searchPositions text pattern

    member this.CountOccurrences(pattern: string) =
        this.Search(pattern).Length

    member _.LongestRepeatedSubstring() =
        longestRepeatedSubstringIn text

    member _.AllSuffixes() =
        allSuffixesIn text

    member _.NodeCount() =
        1 + text.Length
