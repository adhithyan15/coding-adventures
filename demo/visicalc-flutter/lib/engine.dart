// engine.dart — the Flutter/Dart host glue for the VisiCalc demo, computing on
// the shared Rust `spreadsheet-core` engine through its C ABI (spreadsheet-capi).
//
// This is the Dart sibling of the SwiftUI demo's Engine.swift and the Qt demo's
// SpreadsheetModel: it owns NO spreadsheet logic. The Rust engine (cells,
// dependency graph, recalc, formulas) lives behind the C ABI; this file marshals
// Dart Strings across `dart:ffi` and maps the engine's JSON value shape into the
// display text a spreadsheet should show — the same engine, and the same JSON
// contract, the web demos drive as WebAssembly and the native demos link.
//
// Zero extra pub dependencies: rather than pull in `package:ffi` for its `Utf8`
// /`malloc` helpers, we bind libc's `malloc`/`free` through
// `DynamicLibrary.process()` and marshal UTF-8 by hand with `dart:convert`. The
// only imports are core SDK libraries.

import 'dart:convert';
import 'dart:ffi';
import 'dart:io' show Directory, File, Platform;
import 'dart:math' show max;

// ---------------------------------------------------------------------------
// C ABI function signatures (see spreadsheet-capi/include/spreadsheet.h).
// Each takes/returns opaque pointers or `char*`; the memory contract is in the
// header: every returned char* is heap-allocated by the engine and must be
// freed with sc_string_free (NOT libc free — different allocator).
// ---------------------------------------------------------------------------

typedef _SessionNewC = Pointer<Void> Function();
typedef _SessionNewD = Pointer<Void> Function();

typedef _SessionFreeC = Void Function(Pointer<Void>);
typedef _SessionFreeD = void Function(Pointer<Void>);

typedef _SetCellC = Pointer<Uint8> Function(
    Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>);
typedef _SetCellD = Pointer<Uint8> Function(
    Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>);

typedef _GetC = Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>);
typedef _GetD = Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>);

typedef _StringFreeC = Void Function(Pointer<Uint8>);
typedef _StringFreeD = void Function(Pointer<Uint8>);

// libc malloc/free, for the strings we pass INTO the engine. (Bound from the
// process so we don't need package:ffi's allocator.)
typedef _MallocC = Pointer<Uint8> Function(IntPtr);
typedef _MallocD = Pointer<Uint8> Function(int);
typedef _FreeC = Void Function(Pointer<Uint8>);
typedef _FreeD = void Function(Pointer<Uint8>);

// Viewport primitive: integer coords in, JSON char* out (current_revision
// returns a u64 directly). Uint32/Uint64 in the native signature; plain `int`
// in the Dart signature.
typedef _WindowC = Pointer<Uint8> Function(Pointer<Void>, Uint32, Uint32, Uint32, Uint32);
typedef _WindowD = Pointer<Uint8> Function(Pointer<Void>, int, int, int, int);
typedef _NoArgC = Pointer<Uint8> Function(Pointer<Void>);
typedef _NoArgD = Pointer<Uint8> Function(Pointer<Void>);
typedef _ColLettersC = Pointer<Uint8> Function(Pointer<Void>, Uint32);
typedef _ColLettersD = Pointer<Uint8> Function(Pointer<Void>, int);
typedef _CurrentRevC = Uint64 Function(Pointer<Void>);
typedef _CurrentRevD = int Function(Pointer<Void>);
typedef _ChangedSinceC = Pointer<Uint8> Function(Pointer<Void>, Uint64);
typedef _ChangedSinceD = Pointer<Uint8> Function(Pointer<Void>, int);

/// A single spreadsheet session, owning the opaque C handle.
class SpreadsheetSession {
  final DynamicLibrary _lib;
  late final Pointer<Void> _handle;

  late final _SessionFreeD _sessionFree;
  late final _SetCellD _setCell;
  late final _GetD _getValue;
  late final _GetD _getRaw;
  late final _StringFreeD _stringFree;
  late final _WindowD _getWindow;
  late final _NoArgD _usedRangeFn;
  late final _ColLettersD _columnLettersFn;
  late final _CurrentRevD _currentRevisionFn;
  late final _ChangedSinceD _changedSinceFn;

  static final DynamicLibrary _proc = DynamicLibrary.process();
  static final _MallocD _malloc =
      _proc.lookupFunction<_MallocC, _MallocD>('malloc');
  static final _FreeD _free = _proc.lookupFunction<_FreeC, _FreeD>('free');

  SpreadsheetSession._(this._lib) {
    final sessionNew = _lib.lookupFunction<_SessionNewC, _SessionNewD>('sc_session_new');
    _sessionFree = _lib.lookupFunction<_SessionFreeC, _SessionFreeD>('sc_session_free');
    _setCell = _lib.lookupFunction<_SetCellC, _SetCellD>('sc_set_cell');
    _getValue = _lib.lookupFunction<_GetC, _GetD>('sc_get_value');
    _getRaw = _lib.lookupFunction<_GetC, _GetD>('sc_get_raw');
    _stringFree = _lib.lookupFunction<_StringFreeC, _StringFreeD>('sc_string_free');
    _getWindow = _lib.lookupFunction<_WindowC, _WindowD>('sc_get_window');
    _usedRangeFn = _lib.lookupFunction<_NoArgC, _NoArgD>('sc_used_range');
    _columnLettersFn = _lib.lookupFunction<_ColLettersC, _ColLettersD>('sc_column_letters');
    _currentRevisionFn = _lib.lookupFunction<_CurrentRevC, _CurrentRevD>('sc_current_revision');
    _changedSinceFn = _lib.lookupFunction<_ChangedSinceC, _ChangedSinceD>('sc_changed_since');
    _handle = sessionNew();
  }

  /// Open a session, loading the vendored engine dynamic library. Pass an
  /// explicit [libraryPath], or let it resolve `native/libspreadsheet_capi.*`
  /// relative to the current directory (walking up a few levels so it works
  /// from the package root or a subdir).
  factory SpreadsheetSession({String? libraryPath}) {
    final path = libraryPath ?? _resolveLibraryPath();
    return SpreadsheetSession._(DynamicLibrary.open(path));
  }

  static String _resolveLibraryPath() {
    final name = Platform.isMacOS
        ? 'libspreadsheet_capi.dylib'
        : Platform.isWindows
            ? 'spreadsheet_capi.dll'
            : 'libspreadsheet_capi.so';
    var dir = Directory.current;
    for (var i = 0; i < 5; i++) {
      final candidate = File('${dir.path}/native/$name');
      if (candidate.existsSync()) return candidate.path;
      final parent = dir.parent;
      if (parent.path == dir.path) break;
      dir = parent;
    }
    // Fall back to the conventional location relative to CWD; DynamicLibrary
    // .open will throw a clear error if it isn't there.
    return 'native/$name';
  }

  void dispose() => _sessionFree(_handle);

  /// Allocate a NUL-terminated UTF-8 C string with libc malloc. Caller frees
  /// it with [_freeCString].
  Pointer<Uint8> _toCString(String s) {
    final bytes = utf8.encode(s);
    final ptr = _malloc(bytes.length + 1);
    final view = ptr.asTypedList(bytes.length + 1);
    view.setRange(0, bytes.length, bytes);
    view[bytes.length] = 0; // NUL terminator
    return ptr;
  }

  void _freeCString(Pointer<Uint8> p) => _free(p);

  /// Read an engine-returned char* into a Dart String and free it with the
  /// engine's allocator (sc_string_free). A NULL pointer becomes ''.
  String _takeString(Pointer<Uint8> p) {
    if (p == nullptr) return '';
    var len = 0;
    while (p[len] != 0) {
      len++;
    }
    final s = utf8.decode(p.asTypedList(len));
    _stringFree(p);
    return s;
  }

  /// Set a cell from a raw string (literal or formula). Returns the engine's
  /// {"ok":...} JSON (ignored by callers that just want the side effect).
  String setCell(String a1, String raw) {
    final a1Ptr = _toCString(a1);
    final rawPtr = _toCString(raw);
    try {
      return _takeString(_setCell(_handle, a1Ptr, rawPtr));
    } finally {
      _freeCString(a1Ptr);
      _freeCString(rawPtr);
    }
  }

  /// The value JSON for a cell (`{"kind":...}`).
  String getValueJson(String a1) {
    final a1Ptr = _toCString(a1);
    try {
      return _takeString(_getValue(_handle, a1Ptr));
    } finally {
      _freeCString(a1Ptr);
    }
  }

  /// The raw source (formula/literal) of a cell.
  String getRaw(String a1) {
    final a1Ptr = _toCString(a1);
    try {
      return _takeString(_getRaw(_handle, a1Ptr));
    } finally {
      _freeCString(a1Ptr);
    }
  }

  /// Map one decoded value object (`{"kind":...}`) to the string a spreadsheet
  /// cell should show. Shared by `display` (one cell) and `window` (a rectangle).
  static String _displayValue(Map obj) {
    switch (obj['kind']) {
      case 'empty':
        return '';
      case 'number':
        final n = (obj['value'] as num).toDouble();
        // Show integers without a trailing ".0".
        if (n == n.roundToDouble() && n.abs() < 1e15) {
          return n.toInt().toString();
        }
        return n.toString();
      case 'text':
        return (obj['value'] as String?) ?? '';
      case 'boolean':
        return (obj['value'] == true) ? 'TRUE' : 'FALSE';
      case 'error':
        return (obj['code'] as String?) ?? '#ERR';
      default:
        return '';
    }
  }

  /// The display string for a cell — what a spreadsheet should show. Parses the
  /// engine's JSON (the same shape the TS/WASM/Swift/Qt engines emit).
  String display(String a1) {
    final json = getValueJson(a1);
    if (json.isEmpty) return '';
    final Object? obj = jsonDecode(json);
    return (obj is Map) ? _displayValue(obj) : '';
  }

  // ── Viewport primitive (virtualized infinite sheet) ──────────────────
  // These mirror the engine's get_window / used_range / changed_since reads (the
  // C ABI's sc_get_window etc.), 1-based inclusive coords, so a windowed Flutter
  // grid can render only the visible rectangle of an unbounded sheet.

  /// Dense display strings for the inclusive 1-based rectangle, row-major
  /// (empty cells become ''). Empty list on a bad/oversized request.
  List<List<String>> window(int row0, int col0, int row1, int col1) {
    final json = _takeString(_getWindow(_handle, row0, col0, row1, col1));
    final Object? obj = jsonDecode(json);
    if (obj is! Map || obj['values'] is! List) return const [];
    return (obj['values'] as List)
        .map<List<String>>((row) => (row as List)
            .map<String>((c) => (c is Map) ? _displayValue(c) : '')
            .toList())
        .toList();
  }

  /// The data extent `{minRow,minCol,maxRow,maxCol}`, or null if the sheet is
  /// empty (the engine returns the JSON literal `null`).
  Map<String, int>? usedRange() {
    final json = _takeString(_usedRangeFn(_handle));
    final Object? obj = jsonDecode(json);
    if (obj is! Map) return null;
    return {
      'minRow': obj['minRow'] as int,
      'minCol': obj['minCol'] as int,
      'maxRow': obj['maxRow'] as int,
      'maxCol': obj['maxCol'] as int,
    };
  }

  /// Column letters for a 1-based index (`1` → `"A"`, `27` → `"AA"`).
  String columnLetters(int index) => _takeString(_columnLettersFn(_handle, index));

  /// The per-edit revision clock. Snapshot it, then pass to [changedSince].
  int currentRevision() => _currentRevisionFn(_handle);

  /// Cells changed since [since]. `stale` means re-read the whole window.
  ({List<String> changed, bool stale}) changedSince(int since) {
    final json = _takeString(_changedSinceFn(_handle, since));
    final Object? obj = jsonDecode(json);
    if (obj is! Map) return (changed: const [], stale: false);
    if (obj['stale'] == true) return (changed: const [], stale: true);
    final list = (obj['changed'] as List?)?.cast<String>() ?? const [];
    return (changed: list, stale: false);
  }
}

/// An engine-backed 5×5 spreadsheet model the Flutter host drives. Mirrors the
/// SwiftUI `SpreadsheetModel` and the Qt one: it seeds the cross-footing budget,
/// exposes the computed display matrix, and writes through to the engine on edit.
class SpreadsheetModel {
  final SpreadsheetSession _session;
  static const int rows = 5;
  static const int cols = 5; // A..E

  /// Display matrix fed to the Grid: each row is [rowLabel, A, B, C, D, E].
  List<List<String>> viewportRows = const [];

  SpreadsheetModel({SpreadsheetSession? session})
      : _session = session ?? SpreadsheetSession() {
    _seed();
    recompute();
  }

  void dispose() => _session.dispose();

  /// A1 address for grid display row `r` (0-based) and column `c` (1..5).
  static String address(int r, int c) {
    final letter = String.fromCharCode(65 + c - 1);
    return '$letter${r + 1}';
  }

  /// The classic cross-footing budget — identical seed to the other demos:
  /// column E totals each row, row 5 totals each column, E5 is the grand total.
  void _seed() {
    const cells = <List<String>>[
      ['A1', '15'], ['B1', '3'], ['C1', '12'], ['D1', '8'], ['E1', '=SUM(A1:D1)'],
      ['A2', '8'], ['B2', '14'], ['C2', '7'], ['D2', '22'], ['E2', '=SUM(A2:D2)'],
      ['A3', '12'], ['B3', '9'], ['C3', '18'], ['D3', '6'], ['E3', '=SUM(A3:D3)'],
      ['A4', '4'], ['B4', '11'], ['C4', '3'], ['D4', '17'], ['E4', '=SUM(A4:D4)'],
      ['A5', '=SUM(A1:A4)'], ['B5', '=SUM(B1:B4)'], ['C5', '=SUM(C1:C4)'],
      ['D5', '=SUM(D1:D4)'], ['E5', '=SUM(E1:E4)'],
    ];
    for (final cell in cells) {
      _session.setCell(cell[0], cell[1]);
    }
  }

  /// Rebuild the display matrix from the engine's computed values.
  void recompute() {
    final matrix = <List<String>>[];
    for (var r = 0; r < rows; r++) {
      final row = <String>['${r + 1}']; // row-label gutter
      for (var c = 1; c <= cols; c++) {
        row.add(_session.display(address(r, c)));
      }
      matrix.add(row);
    }
    viewportRows = matrix;
  }

  /// The raw source of the cell at display row/col (col 1..5; 0 = gutter).
  String rawAt(int r, int c) => c < 1 ? '' : _session.getRaw(address(r, c));

  /// The value JSON of a cell — used by tests to assert the engine contract.
  String valueJson(String a1) => _session.getValueJson(a1);

  /// Write `raw` into the cell at display row/col and recompute everything.
  void setCell(int r, int c, String raw) {
    if (c < 1) return;
    _session.setCell(address(r, c), raw);
    recompute();
  }
}

/// Engine-backed model for the VIRTUALIZED infinite sheet — the Dart sibling of
/// the SwiftUI `WindowedSheetModel`, the Qt `SpreadsheetModel` infinite-view
/// state, and the web demo's infinite.html. It seeds a deliberately far-flung,
/// sparse dataset and exposes one-row windowed reads plus the data extent, so a
/// `ListView.builder`-virtualized grid can render only the visible rectangle of
/// an effectively-unbounded (u32 × u32) sheet.
///
/// Plain Dart (no ChangeNotifier): the host `StatefulWidget` mutates it inside
/// `setState`, exactly as `main.dart` drives [SpreadsheetModel]. All coordinates
/// here are 1-based (row/col ≥ 1, col 1 = "A"), matching the engine.
class InfiniteSheetModel {
  final SpreadsheetSession _session;

  /// The virtual grid size, derived from the data extent plus a margin so you
  /// can scroll past the data into blank space.
  int totalRows = 1000;
  int totalCols = 60;

  /// The selected cell (1-based) and the formula-bar text (its raw source).
  int selRow = 1;
  int selCol = 1;
  String formula = '';

  InfiniteSheetModel({SpreadsheetSession? session})
      : _session = session ?? SpreadsheetSession() {
    _seed();
    computeExtent();
    selectInf(1, 1); // prime the selection + formula bar at A1
  }

  void dispose() => _session.dispose();

  /// The classic cross-footing budget PLUS far-flung cells (a formula at
  /// `Z1000`, a couple near `BA50`/`BB50`) to prove the sheet is sparse and
  /// unbounded — identical seed to the SwiftUI/Qt infinite views.
  void _seed() {
    const cells = <List<String>>[
      ['A1', '15'], ['B1', '3'], ['C1', '12'], ['D1', '8'], ['E1', '=SUM(A1:D1)'],
      ['A2', '8'], ['B2', '14'], ['C2', '7'], ['D2', '22'], ['E2', '=SUM(A2:D2)'],
      ['A3', '12'], ['B3', '9'], ['C3', '18'], ['D3', '6'], ['E3', '=SUM(A3:D3)'],
      ['A4', '4'], ['B4', '11'], ['C4', '3'], ['D4', '17'], ['E4', '=SUM(A4:D4)'],
      ['A5', '=SUM(A1:A4)'], ['B5', '=SUM(B1:B4)'], ['C5', '=SUM(C1:C4)'],
      ['D5', '=SUM(D1:D4)'], ['E5', '=SUM(E1:E4)'],
      ['Z1000', '=SUM(A1:A4)'], // 1000 rows down: 39
      ['BA50', 'far cell'], ['BB50', '=Z1000*2'], // col 53/54, row 50: 78
    ];
    for (final cell in cells) {
      _session.setCell(cell[0], cell[1]);
    }
  }

  /// Re-derive the virtual grid size from the engine's data extent plus a
  /// comfortable margin. Mirrors `WindowedSheetModel.resize()`.
  void computeExtent() {
    final u = _session.usedRange();
    totalRows = max((u?['maxRow'] ?? 1) + 200, 1000);
    totalCols = max((u?['maxCol'] ?? 1) + 30, 60);
  }

  /// Column letters for a 1-based index (`1` → `"A"`, `27` → `"AA"`).
  String columnLetters(int index) => _session.columnLetters(index);

  /// The A1 address of the selected cell (e.g. `"Z1000"`).
  String get infAddress => '${_session.columnLetters(selCol)}$selRow';

  /// One row's display strings (columns 1..totalCols) — what a virtualized
  /// `ListView` delegate renders. A single engine `get_window` over a 1×N strip;
  /// returns an empty list if the request was rejected/oversized.
  List<String> rowCells(int row) {
    if (row < 1) return const [];
    final w = _session.window(row, 1, row, totalCols);
    return w.isEmpty ? const [] : w[0];
  }

  /// Move the selection (clamped to the virtual grid; row/col ≥ 1) and pull the
  /// selected cell's raw source into the formula bar.
  void selectInf(int row, int col) {
    selRow = row.clamp(1, totalRows);
    selCol = col.clamp(1, totalCols);
    formula = _session.getRaw(infAddress);
  }

  /// Commit the formula bar into the selected cell: write through to the engine
  /// (which recomputes every dependent), grow the extent if the edit reached new
  /// ground, and re-read the canonicalised source back into the bar.
  void commitInf(String raw) {
    _session.setCell(infAddress, raw);
    computeExtent();
    formula = _session.getRaw(infAddress);
  }
}
