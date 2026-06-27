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

// sc_set_format(session, a1, code) → void. An empty code clears the format.
typedef _SetFormatC = Void Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>);
typedef _SetFormatD = void Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>);

// sc_fill(session, src, dst_start, dst_end) → void (drag-fill; three A1 strings).
typedef _FillC = Void Function(
    Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>);
typedef _FillD = void Function(
    Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Pointer<Uint8>);

// sc_sort_range(session, start, end, key_col, ascending) → int (1 applied / was
// already sorted, 0 no-op). Two A1 strings + a 1-based key column + a flag.
typedef _SortRangeC = Int32 Function(
    Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Uint32, Int32);
typedef _SortRangeD = int Function(
    Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, int, int);

// sc_find_all(session, query, in_formulas, match_case) → char* JSON
// {"matches":["A1",…]} (free with sc_string_free).
typedef _FindAllC = Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, Int32, Int32);
typedef _FindAllD = Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, int, int);
// sc_replace_all(session, query, replacement, match_case) → int (count changed).
typedef _ReplaceAllC = Int32 Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, Int32);
typedef _ReplaceAllD = int Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>, int);

// Multi-sheet workbook. sc_sheet_names(session) → char* JSON
// {"sheets":[…],"active":i} (free with sc_string_free); sc_active_sheet → u32;
// sc_set_active_sheet/sc_add_sheet/sc_rename_sheet/sc_delete_sheet → int 1/0.
typedef _SheetNamesC = Pointer<Uint8> Function(Pointer<Void>);
typedef _SheetNamesD = Pointer<Uint8> Function(Pointer<Void>);
typedef _ActiveSheetC = Uint32 Function(Pointer<Void>);
typedef _ActiveSheetD = int Function(Pointer<Void>);
typedef _SetActiveSheetC = Int32 Function(Pointer<Void>, Uint32);
typedef _SetActiveSheetD = int Function(Pointer<Void>, int);
typedef _AddSheetC = Int32 Function(Pointer<Void>, Pointer<Uint8>);
typedef _AddSheetD = int Function(Pointer<Void>, Pointer<Uint8>);
typedef _RenameSheetC = Int32 Function(Pointer<Void>, Uint32, Pointer<Uint8>);
typedef _RenameSheetD = int Function(Pointer<Void>, int, Pointer<Uint8>);
typedef _DeleteSheetC = Int32 Function(Pointer<Void>, Uint32);
typedef _DeleteSheetD = int Function(Pointer<Void>, int);

// sc_insert_rows / sc_delete_rows / sc_insert_cols / sc_delete_cols(session, at,
// count) → void. Structural edits at a 1-based position; the engine shifts
// formula references across the band.
typedef _StructC = Void Function(Pointer<Void>, Uint32, Uint32);
typedef _StructD = void Function(Pointer<Void>, int, int);

// sc_copy / sc_cut(session, start, end) → void (clipboard capture; two A1 strings).
typedef _ClipC = Void Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>);
typedef _ClipD = void Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>);
// sc_paste(session, dst_start) → int (1 applied, 0 no-op).
typedef _PasteC = Int32 Function(Pointer<Void>, Pointer<Uint8>);
typedef _PasteD = int Function(Pointer<Void>, Pointer<Uint8>);
// sc_undo / sc_redo / sc_can_undo / sc_can_redo(session) → int (1/0).
typedef _FlagC = Int32 Function(Pointer<Void>);
typedef _FlagD = int Function(Pointer<Void>);

/// A single spreadsheet session, owning the opaque C handle.
class SpreadsheetSession {
  final DynamicLibrary _lib;
  late final Pointer<Void> _handle;

  late final _SessionFreeD _sessionFree;
  late final _SetCellD _setCell;
  late final _GetD _getValue;
  late final _GetD _getRaw;
  late final _StringFreeD _stringFree;
  late final _WindowD _getDisplayWindow;
  late final _SetFormatD _setFormatFn;
  late final _FillD _fillFn;
  late final _SortRangeD _sortRangeFn;
  late final _FindAllD _findAllFn;
  late final _ReplaceAllD _replaceAllFn;
  late final _SheetNamesD _sheetNamesFn;
  late final _ActiveSheetD _activeSheetFn;
  late final _SetActiveSheetD _setActiveSheetFn;
  late final _AddSheetD _addSheetFn;
  late final _RenameSheetD _renameSheetFn;
  late final _DeleteSheetD _deleteSheetFn;
  late final _ClipD _copyFn;
  late final _ClipD _cutFn;
  late final _PasteD _pasteFn;
  late final _StructD _insertRowsFn;
  late final _StructD _deleteRowsFn;
  late final _StructD _insertColsFn;
  late final _StructD _deleteColsFn;
  // Save / load: sc_serialize(session) → char* (same shape as the no-arg reads);
  // sc_deserialize(session, data) → int (same shape as sc_paste: session + one
  // string → 1/0).
  late final _NoArgD _serializeFn;
  late final _PasteD _deserializeFn;
  late final _FlagD _undoFn;
  late final _FlagD _redoFn;
  late final _FlagD _canUndoFn;
  late final _FlagD _canRedoFn;
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
    _getDisplayWindow = _lib.lookupFunction<_WindowC, _WindowD>('sc_get_display_window');
    _setFormatFn = _lib.lookupFunction<_SetFormatC, _SetFormatD>('sc_set_format');
    _fillFn = _lib.lookupFunction<_FillC, _FillD>('sc_fill');
    _sortRangeFn = _lib.lookupFunction<_SortRangeC, _SortRangeD>('sc_sort_range');
    _findAllFn = _lib.lookupFunction<_FindAllC, _FindAllD>('sc_find_all');
    _replaceAllFn = _lib.lookupFunction<_ReplaceAllC, _ReplaceAllD>('sc_replace_all');
    _sheetNamesFn = _lib.lookupFunction<_SheetNamesC, _SheetNamesD>('sc_sheet_names');
    _activeSheetFn = _lib.lookupFunction<_ActiveSheetC, _ActiveSheetD>('sc_active_sheet');
    _setActiveSheetFn = _lib.lookupFunction<_SetActiveSheetC, _SetActiveSheetD>('sc_set_active_sheet');
    _addSheetFn = _lib.lookupFunction<_AddSheetC, _AddSheetD>('sc_add_sheet');
    _renameSheetFn = _lib.lookupFunction<_RenameSheetC, _RenameSheetD>('sc_rename_sheet');
    _deleteSheetFn = _lib.lookupFunction<_DeleteSheetC, _DeleteSheetD>('sc_delete_sheet');
    _copyFn = _lib.lookupFunction<_ClipC, _ClipD>('sc_copy');
    _cutFn = _lib.lookupFunction<_ClipC, _ClipD>('sc_cut');
    _pasteFn = _lib.lookupFunction<_PasteC, _PasteD>('sc_paste');
    _insertRowsFn = _lib.lookupFunction<_StructC, _StructD>('sc_insert_rows');
    _deleteRowsFn = _lib.lookupFunction<_StructC, _StructD>('sc_delete_rows');
    _insertColsFn = _lib.lookupFunction<_StructC, _StructD>('sc_insert_cols');
    _deleteColsFn = _lib.lookupFunction<_StructC, _StructD>('sc_delete_cols');
    _serializeFn = _lib.lookupFunction<_NoArgC, _NoArgD>('sc_serialize');
    _deserializeFn = _lib.lookupFunction<_PasteC, _PasteD>('sc_deserialize');
    _undoFn = _lib.lookupFunction<_FlagC, _FlagD>('sc_undo');
    _redoFn = _lib.lookupFunction<_FlagC, _FlagD>('sc_redo');
    _canUndoFn = _lib.lookupFunction<_FlagC, _FlagD>('sc_can_undo');
    _canRedoFn = _lib.lookupFunction<_FlagC, _FlagD>('sc_can_redo');
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

  /// Set a cell's display format code (an Excel-style code like `"#,##0.00"` or
  /// `"0%"`); an empty code clears it. Drives the engine's display path that
  /// [window] reads through `sc_get_display_window`.
  void setFormat(String a1, String code) {
    final a1Ptr = _toCString(a1);
    final codePtr = _toCString(code);
    try {
      _setFormatFn(_handle, a1Ptr, codePtr);
    } finally {
      _freeCString(a1Ptr);
      _freeCString(codePtr);
    }
  }

  /// Drag-fill: replicate the `src` cell across the inclusive A1 rectangle
  /// `dstStart`..`dstEnd`. Relative references shift per target (`=A1` filled
  /// down → `=A2`), absolute (`$`) refs pin, the source's format carries along,
  /// an empty source clears each target. A malformed address is a no-op.
  void fill(String src, String dstStart, String dstEnd) {
    final srcPtr = _toCString(src);
    final startPtr = _toCString(dstStart);
    final endPtr = _toCString(dstEnd);
    try {
      _fillFn(_handle, srcPtr, startPtr, endPtr);
    } finally {
      _freeCString(srcPtr);
      _freeCString(startPtr);
      _freeCString(endPtr);
    }
  }

  /// Sort the rows of the inclusive rectangle [start]..[end] by the computed
  /// values in [keyCol] (1-based, inside the rectangle), ascending/descending.
  /// Each row moves as a record; the engine shifts moved formulas' references
  /// with their row and carries formats. Returns true when a sort was applied
  /// (or the range was already sorted), false for a no-op (malformed address /
  /// out-of-range key / oversized range). [keyCol] is clamped into the u32 range
  /// before crossing the C ABI (Dart `int` is 64-bit; the C param is `Uint32`).
  bool sortRange(String start, String end, int keyCol, bool ascending) {
    final startPtr = _toCString(start);
    final endPtr = _toCString(end);
    try {
      return _sortRangeFn(_handle, startPtr, endPtr, _u32(keyCol), ascending ? 1 : 0) == 1;
    } finally {
      _freeCString(startPtr);
      _freeCString(endPtr);
    }
  }

  /// Find every cell whose text contains [query] — the A1 addresses, parsed from
  /// the engine's `{"matches":[...]}` JSON. [inFormulas] searches each cell's
  /// source when true, its computed display value when false; [matchCase]=false
  /// folds ASCII case. Empty query → empty list. Read-only.
  List<String> findAll(String query, bool inFormulas, bool matchCase) {
    final qPtr = _toCString(query);
    try {
      final json = _takeString(_findAllFn(_handle, qPtr, inFormulas ? 1 : 0, matchCase ? 1 : 0));
      final obj = jsonDecode(json) as Map<String, dynamic>;
      return (obj['matches'] as List).cast<String>();
    } finally {
      _freeCString(qPtr);
    }
  }

  /// Replace [query] with [replacement] in the source of every matching cell (the
  /// engine rewrites + recomputes; the facade keeps its source echo in step).
  /// [matchCase]=false folds ASCII case. Returns the count of cells changed; an
  /// empty query is a no-op (0).
  int replaceAll(String query, String replacement, bool matchCase) {
    final qPtr = _toCString(query);
    final rPtr = _toCString(replacement);
    try {
      return _replaceAllFn(_handle, qPtr, rPtr, matchCase ? 1 : 0);
    } finally {
      _freeCString(qPtr);
      _freeCString(rPtr);
    }
  }

  // ── Multi-sheet workbook ───────────────────────────────────────────
  // Bare-A1 ops address the ACTIVE sheet; a formula may reference another
  // (=Summary!A1). sheetNames() → { 'sheets': [names…], 'active': index }.

  // Defensive, like [window]/[usedRange]/[changedSince]: a malformed/empty
  // engine payload yields an empty workbook view rather than a thrown cast.
  Map<String, dynamic> sheetNames() {
    final json = _takeString(_sheetNamesFn(_handle));
    final Object? obj = json.isEmpty ? null : jsonDecode(json);
    return obj is Map
        ? obj.cast<String, dynamic>()
        : <String, dynamic>{'sheets': <String>[], 'active': 0};
  }

  int activeSheet() => _activeSheetFn(_handle);
  bool setActiveSheet(int index) => _setActiveSheetFn(_handle, _u32(index)) != 0;
  bool addSheet(String name) {
    final p = _toCString(name);
    try {
      return _addSheetFn(_handle, p) != 0;
    } finally {
      _freeCString(p);
    }
  }

  bool renameSheet(int index, String newName) {
    final p = _toCString(newName);
    try {
      return _renameSheetFn(_handle, _u32(index), p) != 0;
    } finally {
      _freeCString(p);
    }
  }

  bool deleteSheet(int index) => _deleteSheetFn(_handle, _u32(index)) != 0;

  /// Structural edits: insert / delete [count] rows or columns at the 1-based
  /// position [at]. The engine shifts every formula reference at or after the
  /// band (a reference whose whole band is deleted becomes `#REF!`), then
  /// recomputes. [at]/[count] are clamped into the u32 range before crossing the
  /// C ABI: clamping the LOW end stops a negative wrapping to a huge unsigned
  /// band, and clamping the HIGH end stops a 64-bit Dart int silently truncating
  /// to its low 32 bits (Dart `int` is 64-bit; the C param is `Uint32`).
  void insertRows(int at, int count) =>
      _insertRowsFn(_handle, _u32(at), _u32(count));
  void deleteRows(int at, int count) =>
      _deleteRowsFn(_handle, _u32(at), _u32(count));
  void insertCols(int at, int count) =>
      _insertColsFn(_handle, _u32(at), _u32(count));
  void deleteCols(int at, int count) =>
      _deleteColsFn(_handle, _u32(at), _u32(count));

  // Clamp a Dart (64-bit) int into the unsigned 32-bit range the C ABI expects.
  static int _u32(int v) => v < 0 ? 0 : (v > 0xFFFFFFFF ? 0xFFFFFFFF : v);

  /// Copy the inclusive rectangle [start]..[end] into the clipboard — a
  /// whole-block copy that pastes as a unit. The source is untouched; the buffer
  /// survives any number of pastes.
  void copy(String start, String end) {
    final startPtr = _toCString(start);
    final endPtr = _toCString(end);
    try {
      _copyFn(_handle, startPtr, endPtr);
    } finally {
      _freeCString(startPtr);
      _freeCString(endPtr);
    }
  }

  /// Cut the inclusive rectangle [start]..[end]. Like [copy] but a one-shot
  /// move: the [paste] that places it clears the source it didn't overwrite.
  void cut(String start, String end) {
    final startPtr = _toCString(start);
    final endPtr = _toCString(end);
    try {
      _cutFn(_handle, startPtr, endPtr);
    } finally {
      _freeCString(startPtr);
      _freeCString(endPtr);
    }
  }

  /// Paste the clipboard so its top-left lands at [dstStart]. Returns `true`
  /// when applied, `false` (a no-op) for an empty clipboard, malformed address,
  /// or off-grid destination. The block's references shift by the destination's
  /// offset; content and format ride along.
  bool paste(String dstStart) {
    final dstPtr = _toCString(dstStart);
    try {
      return _pasteFn(_handle, dstPtr) != 0;
    } finally {
      _freeCString(dstPtr);
    }
  }

  /// Serialize the whole workbook to a self-contained JSON document — the
  /// SOURCE (formula text + typed literals) + per-cell formats, not the computed
  /// values (those recompute on load, so the document is small and can't disagree
  /// with itself). The engine owns no I/O; the host persists the returned string
  /// wherever it likes. (`_takeString` frees the engine's char* allocation.)
  String serialize() => _takeString(_serializeFn(_handle));

  /// Replace the workbook from a document produced by [serialize]. Returns `true`
  /// on success, `false` for malformed / unsupported input (the workbook is left
  /// untouched — the engine validates before it mutates). Formulas reload live.
  bool deserialize(String data) {
    final dataPtr = _toCString(data);
    try {
      return _deserializeFn(_handle, dataPtr) != 0;
    } finally {
      _freeCString(dataPtr);
    }
  }

  /// Undo / redo: walk the engine's snapshot history. Each returns `true` if it
  /// changed the document (the host then re-reads the viewport), `false` if there
  /// was nothing to do. canUndo/canRedo gate a host's Undo/Redo controls.
  bool undo() => _undoFn(_handle) != 0;
  bool redo() => _redoFn(_handle) != 0;
  bool canUndo() => _canUndoFn(_handle) != 0;
  bool canRedo() => _canRedoFn(_handle) != 0;

  /// Dense display strings for the inclusive 1-based rectangle, row-major
  /// (empty cells become ''). Empty list on a bad/oversized request.
  ///
  /// Reads `sc_get_display_window`: each cell arrives already rendered through
  /// its format code as a display string, so the host paints it directly and
  /// never re-derives number formatting. The format-aware sibling of
  /// `sc_get_window`; the JSON is `{...,"cells":[["1,234.50",...],...]}`.
  List<List<String>> window(int row0, int col0, int row1, int col1) {
    final json = _takeString(_getDisplayWindow(_handle, row0, col0, row1, col1));
    final Object? obj = jsonDecode(json);
    if (obj is! Map || obj['cells'] is! List) return const [];
    return (obj['cells'] as List)
        .map<List<String>>((row) =>
            (row as List).map<String>((c) => (c as String?) ?? '').toList())
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

    // Attach Excel-style format codes so the engine's display path is visible in
    // the windowed view (which renders via sc_get_display_window): the cross-foot
    // totals read with thousands grouping + two decimals, and the far-flung Z1000
    // total as a percent. Values are unchanged — only how the display strings
    // render. Identical to the web/Qt demos' seeded formats.
    const formats = <List<String>>[
      ['E1', '#,##0.00'], ['E2', '#,##0.00'], ['E3', '#,##0.00'],
      ['E4', '#,##0.00'], ['E5', '#,##0.00'],
      ['A5', '#,##0.00'], ['B5', '#,##0.00'], ['C5', '#,##0.00'], ['D5', '#,##0.00'],
      ['Z1000', '0.0%'], // 39 → "3900.0%": proves the format applies far off-origin
    ];
    for (final f in formats) {
      _session.setFormat(f[0], f[1]);
    }

    // A second sheet, "Summary", proves cross-sheet references compute live:
    // its B3 sums two of its own cells, and back on the first sheet G1 reaches
    // ACROSS with a qualifier (`=Summary!B3`) — identical seed to the web/Qt
    // demos. The first sheet stays active (bare-A1 ops still address it).
    _session.addSheet('Summary');
    _session.setActiveSheet(1);
    _session.setCell('A1', '100');
    _session.setCell('A2', '200');
    _session.setCell('B3', '=A1+A2'); // 300, on the Summary sheet
    _session.setActiveSheet(0);
    _session.setCell('G1', '=Summary!B3'); // 300, pulled across the sheets
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

  /// The engine's per-edit revision clock — bumps on every mutation. The
  /// status footer shows it so the live recompute is visible while scrolling.
  int get revision => _session.currentRevision();

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

  /// Drag-fill: replicate the selected cell into the [rows] rows below it. The
  /// engine shifts each copy's relative references, pins absolute (`$`) refs,
  /// and carries the format. Regrows the extent if the fill reached new ground.
  void fillDown(int rows) {
    final col = columnLetters(selCol);
    final first = '$col${selRow + 1}';
    final last = '$col${selRow + rows}';
    _session.fill(infAddress, first, last);
    computeExtent();
  }

  /// Number formatting: attach an Excel-style format code to the selected cell
  /// (`#,##0.00`, `0.0%`, `$#,##0.00`, or `""` to clear). Display-only — the
  /// stored value is unchanged; the engine renders it through the code, so a
  /// fresh rowCells read shows the formatted string.
  void applyFormat(String code) => _session.setFormat(infAddress, code);

  /// Range sort: reorder the rows of the seeded budget block A1:E4 by the
  /// SELECTED column (clamped into the block's columns A..E = 1..5), ascending
  /// or descending. Each row moves as a record; the E-column SUM formulas travel
  /// with their row (the engine shifts their refs), so every total stays correct.
  /// Returns false for a no-op (already sorted / bad args). Regrows the extent.
  bool sortBlock(bool ascending) {
    final keyCol = selCol < 1 ? 1 : (selCol > 5 ? 5 : selCol);
    final ok = _session.sortRange('A1', 'E4', keyCol, ascending);
    computeExtent();
    return ok;
  }

  /// Find: the A1 addresses whose source contains [query] (case-insensitive).
  List<String> findAll(String query) => _session.findAll(query, true, false);

  /// Replace [query] → [replacement] in every matching cell's source (the engine
  /// recomputes); returns the count changed and regrows the extent.
  int replaceAll(String query, String replacement) {
    final n = _session.replaceAll(query, replacement, false);
    computeExtent();
    return n;
  }

  /// Select the cell at an A1 address (e.g. "B3"); a no-op for a malformed ref.
  void selectA1(String a1) {
    final m = RegExp(r'^([A-Za-z]+)(\d+)$').firstMatch(a1);
    if (m == null) return;
    final letters = m.group(1)!.toUpperCase();
    var col = 0;
    for (var i = 0; i < letters.length; i++) {
      col = col * 26 + (letters.codeUnitAt(i) - 64);
    }
    selectInf(int.parse(m.group(2)!), col);
  }

  /// Structural edits: insert / delete the selected cell's row or column. The
  /// engine shifts every formula reference at or after the band (a reference
  /// whose whole band is deleted becomes `#REF!`) and recomputes; regrow the
  /// extent so the view re-reads. Operate on a single row/column at the cursor.
  void insertRow() {
    _session.insertRows(selRow, 1);
    computeExtent();
  }

  void deleteRow() {
    _session.deleteRows(selRow, 1);
    computeExtent();
  }

  void insertCol() {
    _session.insertCols(selCol, 1);
    computeExtent();
  }

  void deleteCol() {
    _session.deleteCols(selCol, 1);
    computeExtent();
  }

  /// Clipboard: copy/cut the selected cell, then paste it at the selection. The
  /// engine shifts the pasted formula's relative references by the destination's
  /// offset, pins absolute (`$`) refs, carries the format; a cut clears the
  /// source on paste. [pasteCell] returns false (a no-op) when the clipboard is
  /// empty, and regrows the extent on success.
  void copyCell() => _session.copy(infAddress, infAddress);
  void cutCell() => _session.cut(infAddress, infAddress);
  bool pasteCell() {
    final ok = _session.paste(infAddress);
    if (ok) computeExtent();
    return ok;
  }

  /// Save / load: serialize the whole workbook to a JSON document, and restore
  /// it. The document stores only the source + formats — computed values
  /// recompute on load, so a loaded formula stays live. [loadBook] returns false
  /// (workbook untouched) for malformed input; on success it regrows the extent
  /// and refreshes the formula bar so the view re-reads.
  String saveBook() => _session.serialize();
  bool loadBook(String data) {
    final ok = _session.deserialize(data);
    if (ok) {
      computeExtent();
      formula = _session.getRaw(infAddress);
    }
    return ok;
  }

  /// Undo / redo: walk the engine's snapshot history. On success the extent
  /// regrows and the formula bar refreshes (any cell could have changed); a
  /// restored formula stays live. canUndo/canRedo gate the buttons.
  bool get canUndo => _session.canUndo();
  bool get canRedo => _session.canRedo();
  bool undoEdit() {
    final ok = _session.undo();
    if (ok) {
      computeExtent();
      formula = _session.getRaw(infAddress);
    }
    return ok;
  }
  bool redoEdit() {
    final ok = _session.redo();
    if (ok) {
      computeExtent();
      formula = _session.getRaw(infAddress);
    }
    return ok;
  }

  // ── Multi-sheet workbook ───────────────────────────────────────────
  // The workbook holds several sheets; bare-A1 ops address the ACTIVE one,
  // while a formula may reach across with a qualifier (`=Summary!A1`). The
  // tab bar reads [sheetNames]/[activeSheet] and drives the mutators below;
  // each refreshes the extent + formula bar so the windowed view re-reads.

  /// The sheet names in tab order.
  List<String> get sheetNames {
    final m = _session.sheetNames();
    return (m['sheets'] as List?)?.cast<String>() ?? const [];
  }

  /// The active sheet's 0-based index.
  int get activeSheet => _session.activeSheet();

  /// Switch the active sheet (bare-A1 ops now address it). Reselects A1's
  /// neighbourhood by re-priming the formula bar at the current cursor.
  void selectSheet(int index) {
    if (_session.setActiveSheet(index)) {
      computeExtent();
      selectInf(selRow, selCol);
    }
  }

  /// Add a new empty sheet and switch to it.
  void addSheet(String name) {
    if (_session.addSheet(name)) {
      _session.setActiveSheet(sheetNames.length - 1);
      computeExtent();
      selectInf(1, 1);
    }
  }

  /// Rename a sheet by index; cross-sheet references that named it are
  /// rewritten by the engine, so dependents stay live.
  void renameSheet(int index, String newName) {
    if (_session.renameSheet(index, newName)) {
      computeExtent();
      selectInf(selRow, selCol);
    }
  }

  /// Delete a sheet by index. References into it become `#REF!`; the engine
  /// keeps at least one sheet, so this is a no-op on the last one.
  void deleteSheet(int index) {
    if (_session.deleteSheet(index)) {
      computeExtent();
      selectInf(1, 1);
    }
  }
}
