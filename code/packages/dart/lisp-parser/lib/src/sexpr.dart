/// The typed Lisp AST: [SExpr] and [AtomKind], mirroring the
/// reference's public types exactly.
library;

/// The kind of atom (terminal value) in a Lisp expression.
///
/// Atoms are the leaves of the S-expression tree. They have no
/// children — they are raw values.
enum AtomKind {
  /// An integer literal. The string value is the number text (e.g.,
  /// `"42"`, `"-7"`).
  number,

  /// A symbol (identifier or operator name). Example values:
  /// `"define"`, `"+"`, `"car"`.
  symbol,

  /// A string literal. The value includes surrounding quotes (e.g.,
  /// `"\"hello\""`).
  string,
}

/// An S-expression — the universal data structure of Lisp.
///
/// Everything in Lisp is either an atom (a single value) or a list
/// (a collection of S-expressions).
sealed class SExpr {
  const SExpr();

  /// Recursively collect all atom values in this S-expression tree.
  ///
  /// This is useful for testing — it lets us verify that the parser
  /// captured all the terminal values without worrying about tree
  /// structure.
  List<String> findAtoms() {
    final results = <String>[];
    _collectAtoms(results);
    return results;
  }

  void _collectAtoms(List<String> results) {
    final self = this;
    if (self is SAtom) {
      results.add(self.value);
    } else if (self is SList) {
      for (final child in self.children) {
        child._collectAtoms(results);
      }
    } else if (self is SDottedPair) {
      for (final child in self.elements) {
        child._collectAtoms(results);
      }
      self.tail._collectAtoms(results);
    } else if (self is SQuoted) {
      self.inner._collectAtoms(results);
    }
  }

  /// Count how many list nodes appear in the tree.
  int countLists() {
    final self = this;
    if (self is SList) {
      return 1 + self.children.fold<int>(0, (n, c) => n + c.countLists());
    } else if (self is SDottedPair) {
      return 1 +
          self.elements.fold<int>(0, (n, c) => n + c.countLists()) +
          self.tail.countLists();
    } else if (self is SQuoted) {
      return self.inner.countLists();
    }
    return 0;
  }

  /// Count how many quoted nodes appear in the tree.
  int countQuoted() {
    final self = this;
    if (self is SQuoted) {
      return 1 + self.inner.countQuoted();
    } else if (self is SList) {
      return self.children.fold<int>(0, (n, c) => n + c.countQuoted());
    } else if (self is SDottedPair) {
      return self.elements.fold<int>(0, (n, c) => n + c.countQuoted()) +
          self.tail.countQuoted();
    }
    return 0;
  }
}

/// A terminal value: number, symbol, or string.
///
/// ```text
/// 42      -> SAtom(AtomKind.number, "42")
/// define  -> SAtom(AtomKind.symbol, "define")
/// "hello" -> SAtom(AtomKind.string, "\"hello\"")
/// ```
final class SAtom extends SExpr {
  const SAtom(this.kind, this.value);

  final AtomKind kind;
  final String value;

  @override
  bool operator ==(Object other) =>
      other is SAtom && other.kind == kind && other.value == value;

  @override
  int get hashCode => Object.hash(kind, value);

  @override
  String toString() => 'SAtom($kind, "$value")';
}

/// A parenthesized list of S-expressions.
///
/// ```text
/// (+ 1 2)   -> SList([SAtom(symbol, "+"), SAtom(number, "1"), ...])
/// ()        -> SList([])
/// ```
final class SList extends SExpr {
  const SList(this.children);

  final List<SExpr> children;

  @override
  bool operator ==(Object other) =>
      other is SList && _exprsEqual(other.children, children);

  @override
  int get hashCode => Object.hashAll(children);

  @override
  String toString() => 'SList($children)';
}

/// A dotted pair: `(elements... . tail)`.
///
/// ```text
/// (a . b)   -> SDottedPair([SAtom(symbol, "a")], SAtom(symbol, "b"))
/// (1 2 . 3) -> SDottedPair([SAtom(number, "1"), ...], SAtom(number, "3"))
/// ```
final class SDottedPair extends SExpr {
  const SDottedPair(this.elements, this.tail);

  final List<SExpr> elements;
  final SExpr tail;

  @override
  bool operator ==(Object other) =>
      other is SDottedPair &&
      _exprsEqual(other.elements, elements) &&
      other.tail == tail;

  @override
  int get hashCode => Object.hash(Object.hashAll(elements), tail);

  @override
  String toString() => 'SDottedPair($elements, $tail)';
}

/// A quoted form: `'expr` is sugar for `(quote expr)`.
///
/// ```text
/// 'foo      -> SQuoted(SAtom(AtomKind.symbol, "foo"))
/// '(1 2 3)  -> SQuoted(SList([...]))
/// ```
final class SQuoted extends SExpr {
  const SQuoted(this.inner);

  final SExpr inner;

  @override
  bool operator ==(Object other) => other is SQuoted && other.inner == inner;

  @override
  int get hashCode => inner.hashCode;

  @override
  String toString() => 'SQuoted($inner)';
}

bool _exprsEqual(List<SExpr> a, List<SExpr> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
