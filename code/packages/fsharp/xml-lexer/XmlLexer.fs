namespace CodingAdventures.XmlLexer.FSharp

open System
open System.Collections.Generic
open CodingAdventures.Lexer.FSharp

type private Mode =
    | Content
    | Tag
    | Comment
    | CData
    | ProcessingInstruction

/// Tokenizes XML using the token names defined by the shared XML grammar.
type XmlLexer(input: string) =
    let source =
        if isNull input then
            nullArg "input"

        input

    let mutable position = 0
    let mutable line = 1
    let mutable column = 1
    let mutable precededByNewline = false
    let mutable mode = Content
    let mutable processingInstructionNeedsTarget = false

    let isXmlWhitespace character =
        character = ' ' || character = '\t' || character = '\r' || character = '\n'

    let isAsciiLetter character =
        (character >= 'A' && character <= 'Z') || (character >= 'a' && character <= 'z')

    let isAsciiDigit character = character >= '0' && character <= '9'

    let isHexDigit character =
        isAsciiDigit character
        || (character >= 'A' && character <= 'F')
        || (character >= 'a' && character <= 'f')

    let isNameStart character = isAsciiLetter character || character = '_'

    let isNamePart character =
        isNameStart character
        || isAsciiDigit character
        || character = ':'
        || character = '.'
        || character = '-'

    let startsWith (literal: string) =
        position + literal.Length <= source.Length
        && String.CompareOrdinal(source, position, literal, 0, literal.Length) = 0

    let unexpected () =
        raise (LexerError(sprintf "Unexpected character '%c'" source[position], line, column))

    let advanceToken (value: string) =
        for character in value do
            if character = '\n' then
                line <- line + 1
                column <- 1
            else
                column <- column + 1

        position <- position + value.Length

    let emit (tokens: ResizeArray<Token>) typeName value =
        let flags =
            if precededByNewline then
                Token.FlagPrecededByNewline
            else
                0

        tokens.Add(Token(TokenType.Grammar, value, line, column, typeName, flags))
        advanceToken value
        precededByNewline <- false

    let tryEmitLiteral tokens literal typeName =
        if startsWith literal then
            emit tokens typeName literal
            true
        else
            false

    let skipWhitespace () =
        while position < source.Length && isXmlWhitespace source[position] do
            let character = source[position]
            position <- position + 1

            if character = '\n' then
                line <- line + 1
                column <- 1
                precededByNewline <- true
            else
                column <- column + 1

    let scanReference tokens =
        let mutable finish = position + 1
        let mutable typeName = "ENTITY_REF"

        if finish < source.Length && source[finish] = '#' then
            typeName <- "CHAR_REF"
            finish <- finish + 1
            let hexadecimal = finish < source.Length && source[finish] = 'x'

            if hexadecimal then
                finish <- finish + 1

            let digitStart = finish

            while
                finish < source.Length
                && (if hexadecimal then isHexDigit source[finish] else isAsciiDigit source[finish])
                do
                finish <- finish + 1

            if finish = digitStart then
                unexpected ()
        else
            if finish >= source.Length || not (isAsciiLetter source[finish]) then
                unexpected ()

            finish <- finish + 1

            while finish < source.Length && (isAsciiLetter source[finish] || isAsciiDigit source[finish]) do
                finish <- finish + 1

        if finish >= source.Length || source[finish] <> ';' then
            unexpected ()

        emit tokens typeName (source.Substring(position, finish - position + 1))

    let scanContent tokens =
        if isXmlWhitespace source[position] then
            skipWhitespace ()
        elif tryEmitLiteral tokens "<!--" "COMMENT_START" then
            mode <- Comment
        elif tryEmitLiteral tokens "<![CDATA[" "CDATA_START" then
            mode <- CData
        elif tryEmitLiteral tokens "<?" "PI_START" then
            mode <- ProcessingInstruction
            processingInstructionNeedsTarget <- true
        elif tryEmitLiteral tokens "</" "CLOSE_TAG_START" then
            mode <- Tag
        elif tryEmitLiteral tokens "<" "OPEN_TAG_START" then
            mode <- Tag
        elif source[position] = '&' then
            scanReference tokens
        else
            let mutable finish = position

            while finish < source.Length && source[finish] <> '<' && source[finish] <> '&' do
                finish <- finish + 1

            emit tokens "TEXT" (source.Substring(position, finish - position))

    let scanTag tokens =
        if isXmlWhitespace source[position] then
            skipWhitespace ()
        elif tryEmitLiteral tokens "/>" "SELF_CLOSE" then
            mode <- Content
        elif tryEmitLiteral tokens ">" "TAG_CLOSE" then
            mode <- Content
        elif tryEmitLiteral tokens "=" "ATTR_EQUALS" then
            ()
        elif tryEmitLiteral tokens "/" "SLASH" then
            ()
        else
            let current = source[position]

            if current = '\'' || current = '"' then
                let finish = source.IndexOf(current, position + 1)

                if finish < 0 then
                    unexpected ()

                emit tokens "ATTR_VALUE" (source.Substring(position, finish - position + 1))
            elif isNameStart current then
                let mutable finish = position + 1

                while finish < source.Length && isNamePart source[finish] do
                    finish <- finish + 1

                emit tokens "TAG_NAME" (source.Substring(position, finish - position))
            else
                unexpected ()

    let scanDelimitedContent tokens delimiter textType endType =
        if tryEmitLiteral tokens delimiter endType then
            mode <- Content
        else
            let delimiterPosition = source.IndexOf(delimiter, position, StringComparison.Ordinal)

            let finish =
                if delimiterPosition < 0 then source.Length else delimiterPosition

            emit tokens textType (source.Substring(position, finish - position))

    let scanProcessingInstruction tokens =
        if tryEmitLiteral tokens "?>" "PI_END" then
            mode <- Content
        elif processingInstructionNeedsTarget && isNameStart source[position] then
            let mutable finish = position + 1

            while finish < source.Length && isNamePart source[finish] do
                finish <- finish + 1

            emit tokens "PI_TARGET" (source.Substring(position, finish - position))
            processingInstructionNeedsTarget <- false
        else
            let delimiterPosition = source.IndexOf("?>", position, StringComparison.Ordinal)

            let finish =
                if delimiterPosition < 0 then source.Length else delimiterPosition

            emit tokens "PI_TEXT" (source.Substring(position, finish - position))
            processingInstructionNeedsTarget <- false

    /// Tokenizes the configured XML source and appends an EOF token.
    member _.Tokenize() : IReadOnlyList<Token> =
        let tokens = ResizeArray<Token>()

        while position < source.Length do
            match mode with
            | Content -> scanContent tokens
            | Tag -> scanTag tokens
            | Comment -> scanDelimitedContent tokens "-->" "COMMENT_TEXT" "COMMENT_END"
            | CData -> scanDelimitedContent tokens "]]>" "CDATA_TEXT" "CDATA_END"
            | ProcessingInstruction -> scanProcessingInstruction tokens

        tokens.Add(Token(TokenType.EOF, String.Empty, line, column, "EOF"))
        tokens :> IReadOnlyList<Token>

/// Convenience factory and one-shot XML tokenization helpers.
[<RequireQualifiedAccess>]
module XmlTokenizer =
    let createXmlLexer source = XmlLexer(source)

    let tokenizeXml source = (createXmlLexer source).Tokenize()
