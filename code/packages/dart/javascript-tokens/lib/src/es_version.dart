/// Every ECMAScript edition with a grammar file under
/// `code/grammars/ecmascript/`, in chronological order.
/// ES2 and ES5.1 are not separate variants — they alias ES1 and ES5
/// respectively (editorial changes only), and the grammar tree omits
/// them.
library;

/// Every ECMAScript edition with a grammar file.
enum EsVersion implements Comparable<EsVersion> {
  es1('es1'),
  es3('es3'),
  es5('es5'),
  es2015('es2015'),
  es2016('es2016'),
  es2017('es2017'),
  es2018('es2018'),
  es2019('es2019'),
  es2020('es2020'),
  es2021('es2021'),
  es2022('es2022'),
  es2023('es2023'),
  es2024('es2024'),
  es2025('es2025');

  const EsVersion(this._name);

  final String _name;

  /// The most recent edition with a grammar. Used as the default when
  /// callers don't specify a version.
  static const EsVersion latest = EsVersion.es2025;

  /// Every variant in chronological order.
  static const List<EsVersion> all = EsVersion.values;

  /// The basename of the matching grammar file under
  /// `code/grammars/ecmascript/`, and the identifier the lexer and
  /// parser `SUPPORTED_VERSIONS` lists use.
  String asStr() => _name;

  /// Parses back the strings [asStr] emits. Anything else throws
  /// [UnknownEsVersion] — the empty string included (the legacy
  /// "generic" version is not a valid `EsVersion` and never will be).
  static EsVersion parse(String s) {
    final parsed = tryParse(s);
    if (parsed == null) {
      throw UnknownEsVersion(s);
    }
    return parsed;
  }

  /// Nullable twin of [parse]: `null` instead of throwing.
  static EsVersion? tryParse(String s) {
    for (final v in EsVersion.values) {
      if (v._name == s) {
        return v;
      }
    }
    return null;
  }

  /// Chronological order (declaration order).
  @override
  int compareTo(EsVersion other) => index.compareTo(other.index);

  @override
  String toString() => _name;
}

/// Thrown by [EsVersion.parse] when the input matches no known ES
/// edition. Mirrors the reference's `UnknownEsVersion` error,
/// including its message.
class UnknownEsVersion implements Exception {
  const UnknownEsVersion(this.input);

  final String input;

  String get message {
    final valid = EsVersion.values.map((v) => '"${v.asStr()}"').join(', ');
    return 'unknown ECMAScript version "$input"; valid values are $valid';
  }

  @override
  String toString() => message;
}
