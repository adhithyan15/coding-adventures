/// Glob matcher for capability targets.
///
/// Matching rules (from the spec and the Go conformance suite):
///
/// ```text
/// Pattern              Matches
/// foo                  exactly `foo`
/// *                    any single path component (no separators)
/// **                   any number of components
/// *.tokens             any single component ending in `.tokens`
/// ./grammars/*.tokens  one-level files under `./grammars/`
/// host:port            literal host and port
/// *:443                any host on port 443
/// api.weather.gov:*    any port on `api.weather.gov`
/// ```
///
/// Path separators are `/` (Windows backslashes normalize first). Net
/// targets use `:` between host and port.
library;

/// True iff [candidate] (a literal call argument) matches [pattern] (a
/// target string from the manifest).
bool matchTarget(String pattern, String candidate) {
  if (pattern == '*') return true;

  // Net targets use ':' between host and port; when both sides have one
  // and the pattern side looks like a net target, match host and port
  // independently.
  final pIdx = pattern.lastIndexOf(':');
  final cIdx = candidate.lastIndexOf(':');
  if (pIdx >= 0 && cIdx >= 0) {
    final pHost = pattern.substring(0, pIdx);
    final pPort = pattern.substring(pIdx + 1);
    final cHost = candidate.substring(0, cIdx);
    final cPort = candidate.substring(cIdx + 1);
    // Heuristic: no '/' or '\' in the pattern and no space in the
    // pattern's port. Path globs never hold ':' between segments.
    final looksLikeNet = !pattern.contains('/') &&
        !pattern.contains('\\') &&
        !pPort.contains(' ');
    if (looksLikeNet) {
      return _matchAtom(pHost, cHost) && _matchAtom(pPort, cPort);
    }
  }

  // Otherwise a path match, after separator normalization.
  return _matchPath(
      _normalizeSeparators(pattern), _normalizeSeparators(candidate));
}

String _normalizeSeparators(String s) => s.replaceAll('\\', '/');

/// Host/port atom: `*` matches anything, else exact equality (no partial
/// host globs).
bool _matchAtom(String pattern, String candidate) =>
    pattern == '*' || pattern == candidate;

bool _matchPath(String pattern, String candidate) =>
    _matchSegments(pattern.split('/'), candidate.split('/'));

bool _matchSegments(List<String> p, List<String> c) {
  if (p.isEmpty) return c.isEmpty;
  if (p.first == '**') {
    // '**' matches zero or more components: try the rest of the pattern
    // against increasing suffixes of the candidate.
    final rest = p.sublist(1);
    for (var idx = 0; idx <= c.length; idx++) {
      if (_matchSegments(rest, c.sublist(idx))) return true;
    }
    return false;
  }
  if (c.isEmpty) return false;
  if (!_matchOneSegment(p.first, c.first)) return false;
  return _matchSegments(p.sublist(1), c.sublist(1));
}

/// Matches one path component (neither side holds `/`).
///
/// `*` alone matches anything; otherwise literal pieces around `*` must
/// match greedily left-to-right with no backtracking: the first piece
/// anchors at the start, the last at the end, middle pieces at their
/// first occurrence from the cursor.
bool _matchOneSegment(String pattern, String candidate) {
  if (pattern == '*') return true;
  if (!pattern.contains('*')) return pattern == candidate;
  final pieces = pattern.split('*');
  var cursor = 0;
  for (var i = 0; i < pieces.length; i++) {
    final piece = pieces[i];
    if (piece.isEmpty) continue;
    if (i == 0) {
      if (!candidate.substring(cursor).startsWith(piece)) return false;
      cursor += piece.length;
    } else if (i == pieces.length - 1) {
      if (!candidate.substring(cursor).endsWith(piece)) return false;
      // NOTE: the reference also checks `len < cursor + piece.len()` here,
      // but that is provably redundant — a suffix of `candidate[cursor..]`
      // necessarily starts at or after `cursor` — so it is omitted.
    } else {
      final idx = candidate.substring(cursor).indexOf(piece);
      if (idx < 0) return false;
      cursor += idx + piece.length;
    }
  }
  return true;
}
