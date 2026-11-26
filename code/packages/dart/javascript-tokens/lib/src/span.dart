/// A byte range within a single source file.
library;

/// A byte range within a single source file.
///
/// [start] and [end] are **byte offsets**, not character or column
/// counts, following the half-open `[start, end)` convention — [end]
/// is exclusive, so an empty span at position `n` is `Span(n, n)`.
///
/// The reference stores `u32` (4 GiB cap); Dart's 64-bit [int] holds
/// the whole domain. Callers are expected to maintain
/// `start <= end` and stay within `u32` — exactly like the
/// reference, this is not enforced at construction (making it a
/// runtime check would force every call site into error handling for
/// what is, in practice, a static guarantee from the lexer). Unlike
/// the reference, [length] cannot underflow: a violated invariant
/// yields a negative length instead of a panic.
class Span implements Comparable<Span> {
  const Span(this.start, this.end);

  final int start;
  final int end;

  /// The number of bytes in this span: `end - start`.
  int get length => end - start;

  /// Whether this span covers zero bytes.
  bool get isEmpty => start == end;

  /// Lexicographic order: [start] first, then [end].
  @override
  int compareTo(Span other) {
    final byStart = start.compareTo(other.start);
    if (byStart != 0) {
      return byStart;
    }
    return end.compareTo(other.end);
  }

  @override
  bool operator ==(Object other) =>
      other is Span && other.start == start && other.end == end;

  @override
  int get hashCode => Object.hash(start, end);

  @override
  String toString() => 'Span($start, $end)';
}
