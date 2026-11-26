/// The broad classification of a JavaScript / TypeScript token.
library;

/// The broad classification of a JavaScript / TypeScript token.
///
/// The categories common to every ES edition — [TokenKind.name],
/// [TokenKind.number], [TokenKind.string], etc. — plus ones that
/// only appear from a specific edition onward ([TokenKind.regex]
/// from ES3, [TokenKind.templateHead] et al from ES2015,
/// [TokenKind.bigInt] from ES2020, [TokenKind.privateName] from
/// ES2022, [TokenKind.hashbang] from ES2023). It is intentionally
/// **not** exhaustive over operators and punctuation; per-version
/// token names live in the `.tokens` grammars and travel via
/// [TokenKindOther] when a consumer needs the exact name.
///
/// A sealed class (not an enum) because [TokenKindOther] carries a
/// payload. Equality and hashing are by value, so kinds work as map
/// keys.
sealed class TokenKind {
  const TokenKind();

  static const name = TokenKindName();
  static const number = TokenKindNumber();
  static const string = TokenKindString();
  static const regex = TokenKindRegex();
  static const templateNoSub = TokenKindTemplateNoSub();
  static const templateHead = TokenKindTemplateHead();
  static const templateMiddle = TokenKindTemplateMiddle();
  static const templateTail = TokenKindTemplateTail();
  static const bigInt = TokenKindBigInt();
  static const privateName = TokenKindPrivateName();
  static const keyword = TokenKindKeyword();
  static const operator = TokenKindOperator();
  static const punctuation = TokenKindPunctuation();
  static const comment = TokenKindComment();
  static const whitespace = TokenKindWhitespace();
  static const newline = TokenKindNewline();
  static const hashbang = TokenKindHashbang();
  static const error = TokenKindError();
  static const eof = TokenKindEof();

  /// `true` for the kinds the parser typically skips ([comment],
  /// [whitespace], [newline]). Note that [newline] is sometimes
  /// *not* skipped — ASI needs to observe newlines. A hint, not a
  /// hard rule. Implemented as an exhaustive switch so adding a
  /// variant breaks compilation until its trivia-ness is decided.
  bool get isTrivia => switch (this) {
        TokenKindComment() ||
        TokenKindWhitespace() ||
        TokenKindNewline() =>
          true,
        _ => false,
      };

  /// `true` for the end-of-input sentinel only.
  bool get isEof => this is TokenKindEof;
}

/// An identifier (`foo`, `_bar`, `$baz`).
final class TokenKindName extends TokenKind {
  const TokenKindName();
  @override
  bool operator ==(Object other) => other is TokenKindName;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'TokenKind.name';
}

/// A numeric literal (`42`, `3.14`, `0xFF`, `1_000`).
final class TokenKindNumber extends TokenKind {
  const TokenKindNumber();
  @override
  bool operator ==(Object other) => other is TokenKindNumber;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'TokenKind.number';
}

/// A string literal (single- or double-quoted).
final class TokenKindString extends TokenKind {
  const TokenKindString();
  @override
  bool operator ==(Object other) => other is TokenKindString;
  @override
  int get hashCode => 3;
  @override
  String toString() => 'TokenKind.string';
}

/// A regular-expression literal (ES3+).
final class TokenKindRegex extends TokenKind {
  const TokenKindRegex();
  @override
  bool operator ==(Object other) => other is TokenKindRegex;
  @override
  int get hashCode => 4;
  @override
  String toString() => 'TokenKind.regex';
}

/// A complete template literal with no substitutions (ES2015+).
final class TokenKindTemplateNoSub extends TokenKind {
  const TokenKindTemplateNoSub();
  @override
  bool operator ==(Object other) => other is TokenKindTemplateNoSub;
  @override
  int get hashCode => 5;
  @override
  String toString() => 'TokenKind.templateNoSub';
}

/// The opening chunk of a template with substitutions (ES2015+).
final class TokenKindTemplateHead extends TokenKind {
  const TokenKindTemplateHead();
  @override
  bool operator ==(Object other) => other is TokenKindTemplateHead;
  @override
  int get hashCode => 6;
  @override
  String toString() => 'TokenKind.templateHead';
}

/// A middle chunk between substitutions (ES2015+).
final class TokenKindTemplateMiddle extends TokenKind {
  const TokenKindTemplateMiddle();
  @override
  bool operator ==(Object other) => other is TokenKindTemplateMiddle;
  @override
  int get hashCode => 7;
  @override
  String toString() => 'TokenKind.templateMiddle';
}

/// The closing chunk of a template (ES2015+).
final class TokenKindTemplateTail extends TokenKind {
  const TokenKindTemplateTail();
  @override
  bool operator ==(Object other) => other is TokenKindTemplateTail;
  @override
  int get hashCode => 8;
  @override
  String toString() => 'TokenKind.templateTail';
}

/// A `BigInt` literal: `42n`, `0xFFn` (ES2020+).
final class TokenKindBigInt extends TokenKind {
  const TokenKindBigInt();
  @override
  bool operator ==(Object other) => other is TokenKindBigInt;
  @override
  int get hashCode => 9;
  @override
  String toString() => 'TokenKind.bigInt';
}

/// A private class member name: `#count` (ES2022+).
final class TokenKindPrivateName extends TokenKind {
  const TokenKindPrivateName();
  @override
  bool operator ==(Object other) => other is TokenKindPrivateName;
  @override
  int get hashCode => 10;
  @override
  String toString() => 'TokenKind.privateName';
}

/// A reserved keyword (`var`, `let`, `class`, `async`, …).
final class TokenKindKeyword extends TokenKind {
  const TokenKindKeyword();
  @override
  bool operator ==(Object other) => other is TokenKindKeyword;
  @override
  int get hashCode => 11;
  @override
  String toString() => 'TokenKind.keyword';
}

/// An operator (`+`, `=>`, `?.`, `||=`, …).
final class TokenKindOperator extends TokenKind {
  const TokenKindOperator();
  @override
  bool operator ==(Object other) => other is TokenKindOperator;
  @override
  int get hashCode => 12;
  @override
  String toString() => 'TokenKind.operator';
}

/// A punctuation symbol (`(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`).
final class TokenKindPunctuation extends TokenKind {
  const TokenKindPunctuation();
  @override
  bool operator ==(Object other) => other is TokenKindPunctuation;
  @override
  int get hashCode => 13;
  @override
  String toString() => 'TokenKind.punctuation';
}

/// A line or block comment. Trivia.
final class TokenKindComment extends TokenKind {
  const TokenKindComment();
  @override
  bool operator ==(Object other) => other is TokenKindComment;
  @override
  int get hashCode => 14;
  @override
  String toString() => 'TokenKind.comment';
}

/// A run of inter-token whitespace. Trivia.
final class TokenKindWhitespace extends TokenKind {
  const TokenKindWhitespace();
  @override
  bool operator ==(Object other) => other is TokenKindWhitespace;
  @override
  int get hashCode => 15;
  @override
  String toString() => 'TokenKind.whitespace';
}

/// A line terminator. Trivia (carried separately from [TokenKind.whitespace] because ASI cares about newlines).
final class TokenKindNewline extends TokenKind {
  const TokenKindNewline();
  @override
  bool operator ==(Object other) => other is TokenKindNewline;
  @override
  int get hashCode => 16;
  @override
  String toString() => 'TokenKind.newline';
}

/// A `#!` shebang at the very start of a module (ES2023+).
final class TokenKindHashbang extends TokenKind {
  const TokenKindHashbang();
  @override
  bool operator ==(Object other) => other is TokenKindHashbang;
  @override
  int get hashCode => 17;
  @override
  String toString() => 'TokenKind.hashbang';
}

/// A lexer error token (e.g. unterminated string).
final class TokenKindError extends TokenKind {
  const TokenKindError();
  @override
  bool operator ==(Object other) => other is TokenKindError;
  @override
  int get hashCode => 18;
  @override
  String toString() => 'TokenKind.error';
}

/// The end-of-input sentinel.
final class TokenKindEof extends TokenKind {
  const TokenKindEof();
  @override
  bool operator ==(Object other) => other is TokenKindEof;
  @override
  int get hashCode => 19;
  @override
  String toString() => 'TokenKind.eof';
}

/// Catch-all for grammar-driven tokens whose specific name doesn't
/// fit any other variant. The wrapped [name] is the token name from
/// the `.tokens` file (e.g. `"OPTIONAL_CHAIN"`).
final class TokenKindOther extends TokenKind {
  const TokenKindOther(this.name);
  final String name;
  @override
  bool operator ==(Object other) =>
      other is TokenKindOther && other.name == name;
  @override
  int get hashCode => Object.hash(20, name);
  @override
  String toString() => 'TokenKind.other($name)';
}
