// ============================================================================
// LexerContext.swift — Callback interface for controlling the lexer.
// ============================================================================
//
// When a callback is registered via `GrammarLexer.setOnToken()`, it
// receives a `LexerContext` on every token match. The context provides
// controlled access to the group stack, token emission, and skip control.
//
// Think of the context as a "request form" that the callback fills out.
// The lexer's main loop reads the form after the callback returns and
// applies the requested actions in order:
//
//     1. Suppress the current token (if requested)
//     2. Append any emitted synthetic tokens
//     3. Apply group stack changes (push/pop)
//     4. Toggle skip processing (if requested)
//
// This decoupled design means callbacks cannot corrupt the lexer's state
// mid-match -- all mutations are deferred and applied atomically by the
// lexer after the callback returns.
//
// Example -- XML lexer callback:
//
//     lexer.setOnToken { token, ctx in
//         if token.type == "OPEN_TAG_START" {
//             ctx.pushGroup("tag")
//         } else if token.type == "TAG_CLOSE" || token.type == "SELF_CLOSE" {
//             ctx.popGroup()
//         }
//     }
//
// ============================================================================

import Foundation

/// Interface that on-token callbacks use to control the lexer.
///
/// Methods that modify state (push/pop/emit/suppress) take effect **after**
/// the callback returns -- they do not interrupt the current match.
///
/// This is a class (reference type) because it accumulates state during
/// a callback invocation. The lexer creates a fresh context for each
/// token match, reads the accumulated actions, then discards it.
///
public final class LexerContext: @unchecked Sendable {

    // -- Internal references (not exposed to callbacks) --

    /// Reference to the lexer (for reading group stack state).
    private let _lexer: GrammarLexer

    /// The full source string being tokenized.
    private let _source: String

    /// Position in the source immediately after the current token.
    private let _posAfter: Int

    /// The most recently emitted token (for lookbehind).
    private let _previousToken: Token?

    /// The current token's line number (for newline detection).
    private let _currentTokenLine: Int

    // -- Accumulated actions (read by the lexer after callback returns) --

    /// Whether the current token should be suppressed from output.
    internal var suppressed: Bool = false

    /// Synthetic tokens to inject after the current one.
    internal var emitted: [Token] = []

    /// Group stack actions recorded by the callback: ("push", name) or ("pop", "").
    internal var groupActions: [(String, String)] = []

    /// New skip-enabled state, or nil if unchanged.
    internal var skipEnabledOverride: Bool? = nil

    // -- Initialization --

    /// Create a new LexerContext for a single token match.
    ///
    /// - Parameters:
    ///   - lexer: The GrammarLexer instance (for group stack queries).
    ///   - source: The full source string being tokenized.
    ///   - posAfterToken: Position in the source immediately after the matched token.
    ///   - previousToken: The most recently emitted token, or nil at start of input.
    ///   - currentTokenLine: The line number of the current token.
    ///
    internal init(
        lexer: GrammarLexer,
        source: String,
        posAfterToken: Int,
        previousToken: Token?,
        currentTokenLine: Int
    ) {
        self._lexer = lexer
        self._source = source
        self._posAfter = posAfterToken
        self._previousToken = previousToken
        self._currentTokenLine = currentTokenLine
    }

    // -----------------------------------------------------------------------
    // Group Stack Operations
    // -----------------------------------------------------------------------

    /// Push a pattern group onto the stack.
    ///
    /// The pushed group becomes active for the **next** token match.
    /// Throws a fatal error if the group name is not defined in the grammar.
    ///
    /// Multiple pushes in a single callback are applied in order, so you
    /// can stack multiple groups if needed (though this is rare).
    ///
    /// - Parameter groupName: Name of the group to activate.
    ///
    public func pushGroup(_ groupName: String) {
        guard _lexer.hasGroup(groupName) else {
            let available = _lexer.availableGroups().sorted().joined(separator: ", ")
            fatalError(
                "Unknown pattern group: '\(groupName)'. "
                + "Available groups: \(available)"
            )
        }
        groupActions.append(("push", groupName))
    }

    /// Pop the current group from the stack.
    ///
    /// If only the "default" group remains (stack depth = 1), this is a
    /// no-op. The default group is the floor and cannot be popped -- this
    /// prevents accidental stack underflow in recursive structures.
    ///
    public func popGroup() {
        groupActions.append(("pop", ""))
    }

    /// Return the name of the currently active group.
    ///
    /// The active group is the top of the group stack. When no groups
    /// have been pushed, this is always "default".
    ///
    public func activeGroup() -> String {
        return _lexer.activeGroup()
    }

    /// Return the depth of the group stack (always >= 1).
    ///
    /// A depth of 1 means only the "default" group is on the stack.
    /// A depth of 2 means one group has been pushed on top of default.
    ///
    public func groupStackDepth() -> Int {
        return _lexer.groupStackDepth()
    }

    // -----------------------------------------------------------------------
    // Token Emission and Suppression
    // -----------------------------------------------------------------------

    /// Inject a synthetic token after the current one.
    ///
    /// Emitted tokens do NOT trigger the callback (this prevents infinite
    /// loops -- a callback that emits tokens which trigger the callback
    /// which emits more tokens...). Multiple `emit()` calls produce
    /// tokens in call order.
    ///
    /// - Parameter token: The synthetic token to inject into the output.
    ///
    public func emit(_ token: Token) {
        emitted.append(token)
    }

    /// Suppress the current token -- do not include it in output.
    ///
    /// Combined with `emit()`, this enables **token replacement**: suppress
    /// the original token and emit a modified version in its place.
    ///
    /// Example -- replacing a token:
    ///
    ///     lexer.setOnToken { token, ctx in
    ///         if token.type == "OLD_TYPE" {
    ///             ctx.suppress()
    ///             ctx.emit(Token(type: "NEW_TYPE", value: token.value,
    ///                            line: token.line, column: token.column))
    ///         }
    ///     }
    ///
    public func suppress() {
        suppressed = true
    }

    // -----------------------------------------------------------------------
    // Lookahead
    // -----------------------------------------------------------------------

    /// Peek at a source character past the current token.
    ///
    /// This provides lookahead capability without advancing the lexer's
    /// position. Useful for making group-switching decisions based on
    /// what comes next in the source.
    ///
    /// - Parameter offset: Number of characters ahead (1 = immediately
    ///   after token). Defaults to 1.
    /// - Returns: The character as a String, or empty string if past EOF.
    ///
    /// Example:
    ///
    ///     let nextChar = ctx.peek()       // Character right after token
    ///     let twoAhead = ctx.peek(2)      // Two characters after token
    ///
    public func peek(_ offset: Int = 1) -> String {
        let idx = _source.index(_source.startIndex, offsetBy: _posAfter + offset - 1, limitedBy: _source.endIndex)
        guard let idx = idx, idx < _source.endIndex else {
            return ""
        }
        return String(_source[idx])
    }

    /// Peek at the next `length` characters past the current token.
    ///
    /// Returns a substring starting immediately after the current token.
    /// If fewer than `length` characters remain, returns whatever is left.
    ///
    /// - Parameter length: Number of characters to peek.
    /// - Returns: A string of up to `length` characters.
    ///
    public func peekStr(_ length: Int) -> String {
        let startIdx = _source.index(_source.startIndex, offsetBy: _posAfter, limitedBy: _source.endIndex) ?? _source.endIndex
        let endIdx = _source.index(startIdx, offsetBy: length, limitedBy: _source.endIndex) ?? _source.endIndex
        return String(_source[startIdx..<endIdx])
    }

    // -----------------------------------------------------------------------
    // Skip Control
    // -----------------------------------------------------------------------

    /// Toggle skip pattern processing.
    ///
    /// When disabled, skip patterns (whitespace, comments) are not tried.
    /// This is useful for groups where whitespace is significant -- for
    /// example, CDATA sections in XML where spaces must be preserved
    /// as part of the content rather than being silently consumed.
    ///
    /// - Parameter enabled: true to enable skip patterns, false to disable.
    ///
    public func setSkipEnabled(_ enabled: Bool) {
        skipEnabledOverride = enabled
    }

    // -----------------------------------------------------------------------
    // Extension: Token Lookbehind
    // -----------------------------------------------------------------------

    /// Return the most recently emitted token, or nil at the start of input.
    ///
    /// "Emitted" means the token actually made it into the output list --
    /// suppressed tokens are not counted. This provides **lookbehind**
    /// capability for context-sensitive decisions.
    ///
    /// For example, in JavaScript `/` is a regex literal after `=`, `(`
    /// or `,` but a division operator after `)`, `]`, identifiers, or
    /// numbers. The callback can check `ctx.previousToken()?.type` to
    /// decide which interpretation to use.
    ///
    /// - Returns: The last token in the output list, or nil if no tokens
    ///            have been emitted yet.
    ///
    public func previousToken() -> Token? {
        return _previousToken
    }

    // -----------------------------------------------------------------------
    // Extension: Bracket Depth Tracking
    // -----------------------------------------------------------------------

    /// Return the current nesting depth for a specific bracket type,
    /// or the total depth across all types if no argument is given.
    ///
    /// Depth starts at 0 and increments on each opener (`(`, `[`, `{`),
    /// decrements on each closer (`)`, `]`, `}`). The count never goes
    /// below 0 -- unmatched closers are clamped.
    ///
    /// This is essential for template literal interpolation in languages
    /// like JavaScript, Kotlin, and Ruby, where `}` at brace-depth 0
    /// closes the interpolation rather than being part of a nested
    /// expression.
    ///
    /// - Parameter kind: Optional bracket type to query. If nil, returns
    ///                   the sum of all three depths.
    /// - Returns: The nesting depth (0 = not inside any brackets of that type).
    ///
    /// Example:
    ///
    ///     // Source: "f(a, [b, {c}])"
    ///     // After tokenizing "(":  bracketDepth(.paren)   == 1
    ///     // After tokenizing "[":  bracketDepth(.bracket)  == 1
    ///     // After tokenizing "{":  bracketDepth(.brace)    == 1
    ///     // After tokenizing "}":  bracketDepth(.brace)    == 0
    ///     // bracketDepth()  (no arg) == sum of all three
    ///
    public func bracketDepth(kind: BracketKind? = nil) -> Int {
        return _lexer.bracketDepth(kind: kind)
    }

    // -----------------------------------------------------------------------
    // Extension: Newline Detection
    // -----------------------------------------------------------------------

    /// Return true if a newline appeared between the previous token
    /// and the current token (i.e., they are on different lines).
    ///
    /// This is used by languages with automatic semicolon insertion
    /// (JavaScript, Go) to detect line breaks that trigger implicit
    /// statement termination. The lexer exposes this as a convenience
    /// so callbacks and post-tokenize hooks can set the
    /// `TOKEN_PRECEDED_BY_NEWLINE` flag on tokens that need it.
    ///
    /// Returns false if there is no previous token (start of input).
    ///
    /// Truth table:
    ///
    ///     Previous token | Current token | Result
    ///     --------------|---------------|-------
    ///     nil           | any           | false
    ///     line 1        | line 1        | false
    ///     line 1        | line 2        | true
    ///     line 3        | line 7        | true
    ///
    public func precededByNewline() -> Bool {
        guard let prev = _previousToken else { return false }
        return prev.line < _currentTokenLine
    }
}
