// infinite_grid.dart — a virtualized, effectively-infinite spreadsheet view for
// the Flutter demo, rendered on the shared Rust engine through the viewport
// primitive (the same get_display_window / used_range / changed_since the
// SwiftUI InfiniteGridView, the Qt InfiniteSheet.qml, and web infinite.html drive).
//
// The sheet is u32 × u32 and sparse; only the cells in the VISIBLE rows are ever
// built. The body is a `ListView.builder`, which natively virtualizes — it calls
// its `itemBuilder` only for rows near the viewport and recycles them as they
// scroll off. So a 1000-row-tall sheet costs the handful of rows you can see,
// and each built row makes ONE engine `get_display_window` over its 1×totalCols
// strip (InfiniteSheetModel.rowCells) — display strings, already rendered through
// each cell's format code. Per-frame engine work is proportional to *visible*
// rows, never to the sheet's height.
//
// Two-axis scroll with frozen chrome, all kept in sync by one-way controller
// links (the chrome is non-interactive and slaved to the body's scroll):
//
//   ┌────────┬──────────────────────────────┐
//   │ corner │  column-letter header  (A B…) │  ← frozen on top, follows ↔
//   ├────────┼──────────────────────────────┤
//   │  row   │                              │
//   │ number │   body: ListView of rows     │
//   │ gutter │   (each row = Row of         │  gutter frozen left, follows ↓
//   │  1 2 … │    totalCols cells)          │
//   └────────┴──────────────────────────────┘
//
//   • body vertical scroll  → gutter.jumpTo(offset)   (gutter follows ↕)
//   • body horizontal scroll → header.jumpTo(offset)  (header follows ↔)

import 'dart:io' show Directory, File;

import 'package:flutter/material.dart';
import 'engine.dart';

/// Cell geometry (logical pixels). Shared by the header, gutter, and body so
/// their scroll offsets line up. (Roomier, to match the web reference.)
const double _rowH = 26;
const double _colW = 92;
const double _gutterW = 64;
const double _headH = 28;

// ── Design tokens ──────────────────────────────────────────────────────────
// Mirror code/programs/typescript/visicalc-html/infinite.html's palette so every VisiCalc backend
// reads as one considered surface (dark modern spreadsheet). Same token set as
// the Qt InfiniteSheet.qml port.
const _cBg = Color(0xFF16181D); // app / base cell
const _cPanel = Color(0xFF1B1E24); // toolbar + zebra band
const _cSurface = Color(0xFF21252C); // buttons, pill
const _cSurfaceHover = Color(0xFF2B313A);
const _cSurfaceDown = Color(0xFF14171C);
const _cField = Color(0xFF0F1115); // formula input well
const _cLine = Color(0xFF2C313A); // hairline borders
const _cLineStrong = Color(0xFF3A404B); // control borders
const _cHead = Color(0xFF20242B); // row/col headers
const _cHeadSel = Color(0xFF2B3340); // header of selected row/col
const _cInk = Color(0xFFE8EAED); // primary text
const _cMuted = Color(0xFF9AA3B2); // labels, headers
const _cAccent = Color(0xFF4AA3FF); // selection + focus
const _cSel = Color(0xFF21344A); // selected-cell fill
const _mono = 'monospace';

/// The infinite-sheet view. Owns its [InfiniteSheetModel] and four scroll
/// controllers (body H/V + the slaved header H and gutter V).
class InfiniteGrid extends StatefulWidget {
  const InfiniteGrid({super.key});

  @override
  State<InfiniteGrid> createState() => _InfiniteGridState();
}

class _InfiniteGridState extends State<InfiniteGrid> {
  late final InfiniteSheetModel _model = InfiniteSheetModel();

  // The body scrolls on both axes; the chrome controllers are driven from it.
  final _bodyV = ScrollController();
  final _bodyH = ScrollController();
  final _gutterV = ScrollController();
  final _headerH = ScrollController();

  final _formulaCtrl = TextEditingController();
  // Drives the formula field's accent focus ring (rebuild on focus changes).
  final _formulaFocus = FocusNode();

  // Find / replace inputs.
  final _findCtrl = TextEditingController();
  final _replaceCtrl = TextEditingController();

  // In-memory "saved file" slot for the Save / Load buttons: Save stows the
  // serialized workbook here, Load restores from it. (A real app would write
  // this string to a file; the demo keeps the round trip self-contained.)
  String _savedSnapshot = '';

  // The most recent File → Save / Open result, echoed in the status bar (e.g.
  // "saved 4.2 KB → visicalc-demo.xlsx" / "opened visicalc-demo.csv"). Empty
  // until the first file action.
  String _fileMsg = '';

  // A PRIVATE per-session scratch directory for the File → Save / Open demo,
  // created lazily on first use with `createTemp` (mode 0700 on POSIX). Writing
  // into our own freshly-made directory — rather than a fixed name in the shared,
  // world-writable system temp root (/tmp, mode 1777) — avoids a local
  // symlink-clobber on save and keeps the file from being world-readable, while
  // still letting Open read back what Save wrote (it's a stable per-session dir).
  Directory? _fileDir;
  Directory get _scratchDir =>
      _fileDir ??= Directory.systemTemp.createTempSync('visicalc-');

  @override
  void initState() {
    super.initState();
    // One-way links: the body drives the chrome. `hasClients`/offset guards
    // keep the jumpTo cheap and loop-free (the chrome is non-interactive).
    _bodyV.addListener(() {
      if (_gutterV.hasClients && _gutterV.offset != _bodyV.offset) {
        _gutterV.jumpTo(_bodyV.offset);
      }
    });
    _bodyH.addListener(() {
      if (_headerH.hasClients && _headerH.offset != _bodyH.offset) {
        _headerH.jumpTo(_bodyH.offset);
      }
    });
    _formulaCtrl.text = _model.formula;
    // Repaint the accent focus ring when the formula field gains/loses focus.
    _formulaFocus.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _bodyV.dispose();
    _bodyH.dispose();
    _gutterV.dispose();
    _headerH.dispose();
    _formulaCtrl.dispose();
    _formulaFocus.dispose();
    _findCtrl.dispose();
    _replaceCtrl.dispose();
    // Best-effort cleanup of the private scratch dir (the OS also reclaims temp).
    try {
      _fileDir?.deleteSync(recursive: true);
    } catch (_) {/* already gone / racing OS cleanup — ignore */}
    _model.dispose();
    super.dispose();
  }

  void _select(int row, int col) {
    setState(() {
      _model.selectInf(row, col);
      _formulaCtrl.text = _model.formula;
    });
  }

  void _commit() {
    setState(() {
      _model.commitInf(_formulaCtrl.text);
      _formulaCtrl.text = _model.formula;
    });
  }

  // Drag-fill: replicate the selected cell into the 10 rows below it. The engine
  // shifts each copy's relative refs, pins absolute ($) refs, carries the format.
  void _fillDown() {
    setState(() => _model.fillDown(10));
  }

  // Clipboard: copy/cut the selected cell, then paste at the selection. The
  // engine shifts the pasted formula's relative refs by the destination's
  // offset, pins absolute ($) refs, carries the format; a cut clears on paste.
  void _copy() => _model.copyCell();
  void _cut() => _model.cutCell();
  void _paste() => setState(() => _model.pasteCell());

  // Save / load: serialize the whole workbook (formulas + formats) to a JSON
  // document held in memory, and restore it. Computed values recompute on load,
  // so a loaded formula stays live; the formula bar re-reads after a load.
  void _save() => setState(() => _savedSnapshot = _model.saveBook());
  void _load() {
    if (_savedSnapshot.isEmpty) return;
    setState(() {
      _model.loadBook(_savedSnapshot);
      _formulaCtrl.text = _model.formula;
    });
  }

  // ── File → Save / Open a REAL spreadsheet file on disk ────────────────
  // Unlike the in-memory Save / Load above, these write and read an actual file
  // — an .xlsx (a ZIP, keeping live formulas) or a .csv (text, values only) —
  // proving the engine's file codecs cross dart:ffi as raw bytes. The demo uses
  // a fixed path in the system temp dir (no plugin file picker); a production
  // host would swap in an OS Save/Open dialog and hand the same bytes across.
  String _demoPath(String ext) => '${_scratchDir.path}/visicalc-demo.$ext';

  // Serialize the workbook to `format` bytes and write them to the demo path.
  void _exportFile(String format, String ext) {
    final bytes = _model.exportBytes(format);
    final path = _demoPath(ext);
    File(path).writeAsBytesSync(bytes);
    final kb = (bytes.length / 1024).toStringAsFixed(1);
    setState(() => _fileMsg = 'saved $kb KB → ${_baseName(path)}');
  }

  // Read the demo path's bytes back and open them as `format`, replacing the
  // workbook. Reports "no <file> yet" if it was never saved, or "not a valid
  // <ext>" if the engine rejects the bytes (the workbook is left untouched).
  void _openFile(String format, String ext) {
    final file = File(_demoPath(ext));
    if (!file.existsSync()) {
      setState(() => _fileMsg = 'no ${_baseName(file.path)} yet — Save first');
      return;
    }
    final ok = _model.importBytes(format, file.readAsBytesSync());
    setState(() {
      _formulaCtrl.text = _model.formula;
      _fileMsg = ok
          ? 'opened ${_baseName(file.path)}'
          : 'not a valid $ext file';
    });
  }

  static String _baseName(String path) => path.split('/').last;

  // Undo / redo: walk the engine's snapshot history. On success the whole grid
  // re-reads and the formula bar re-syncs; the buttons disable at the history
  // ends via the model's canUndo/canRedo.
  void _undo() {
    if (!_model.undoEdit()) return;
    setState(() => _formulaCtrl.text = _model.formula);
  }
  void _redo() {
    if (!_model.redoEdit()) return;
    setState(() => _formulaCtrl.text = _model.formula);
  }

  // Structural edits: insert / delete the selected cell's row or column. The
  // engine shifts every formula reference across the band and recomputes; a
  // reference whose whole band is deleted becomes #REF!. The grid re-reads and
  // the formula bar re-syncs (the moved/destroyed source may differ).
  void _insertRow() => setState(() {
        _model.insertRow();
        _formulaCtrl.text = _model.formula;
      });
  void _deleteRow() => setState(() {
        _model.deleteRow();
        _formulaCtrl.text = _model.formula;
      });
  void _insertCol() => setState(() {
        _model.insertCol();
        _formulaCtrl.text = _model.formula;
      });
  void _deleteCol() => setState(() {
        _model.deleteCol();
        _formulaCtrl.text = _model.formula;
      });

  // Number formatting: apply an Excel-style format code to the selected cell.
  // Display-only — the engine renders the stored value through the code, so a
  // setState rebuild re-reads the row and shows the formatted string.
  void _applyFormat(String code) => setState(() => _model.applyFormat(code));

  // Range sort: reorder the budget block A1:E4 by the selected column,
  // ascending/descending. A setState rebuild re-reads the reordered rows.
  void _sortBlock(bool ascending) => setState(() => _model.sortBlock(ascending));

  // Find: select the first cell whose source contains the find text.
  void _find() {
    final q = _findCtrl.text;
    if (q.isEmpty) return;
    final hits = _model.findAll(q);
    if (hits.isEmpty) return;
    setState(() {
      _model.selectA1(hits.first);
      _formulaCtrl.text = _model.formula;
    });
  }

  // Replace all: rewrite the find text → replacement in every matching cell's
  // source (the engine recomputes); the grid re-reads.
  void _replaceAll() {
    final q = _findCtrl.text;
    if (q.isEmpty) return;
    setState(() {
      _model.replaceAll(q, _replaceCtrl.text);
      _formulaCtrl.text = _model.formula;
    });
  }

  // ── Multi-sheet tab handlers ──
  // Switch the active sheet, add one, rename via a small dialog, or delete the
  // active sheet. Each re-syncs the formula bar to the (re-primed) selection.
  void _selectSheet(int index) => setState(() {
        _model.selectSheet(index);
        _formulaCtrl.text = _model.formula;
      });

  void _addSheet() {
    // Name the new sheet "SheetN" where N avoids a clash with existing names.
    final names = _model.sheetNames.toSet();
    var n = names.length + 1;
    while (names.contains('Sheet$n')) {
      n++;
    }
    setState(() {
      _model.addSheet('Sheet$n');
      _formulaCtrl.text = _model.formula;
    });
  }

  Future<void> _renameSheet(int index) async {
    final ctrl = TextEditingController(text: _model.sheetNames[index]);
    final newName = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: _cPanel,
        title: const Text('Rename sheet', style: TextStyle(color: _cInk, fontSize: 14)),
        content: TextField(
          controller: ctrl,
          autofocus: true,
          style: const TextStyle(color: _cInk, fontFamily: _mono),
          cursorColor: _cAccent,
          onSubmitted: (v) => Navigator.of(ctx).pop(v),
          decoration: const InputDecoration(border: OutlineInputBorder()),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(ctx).pop(), child: const Text('Cancel')),
          TextButton(
              onPressed: () => Navigator.of(ctx).pop(ctrl.text),
              child: const Text('Rename')),
        ],
      ),
    );
    ctrl.dispose();
    if (newName == null || newName.trim().isEmpty) return;
    setState(() {
      _model.renameSheet(index, newName.trim());
      _formulaCtrl.text = _model.formula;
    });
  }

  void _deleteSheet(int index) => setState(() {
        _model.deleteSheet(index);
        _formulaCtrl.text = _model.formula;
      });

  // A compact, modern toolbar button (rounded chip with hover/down/disabled
  // states) — the Flutter analog of the web demo's segmented controls and the
  // Qt port's `component ToolButton`. `enabled: null` disables it (Undo/Redo/
  // Load gate on model/snapshot state).
  Widget _toolButton(String label, String tip, VoidCallback onPressed,
      {bool enabled = true}) {
    return _ToolButton(
      label: label,
      tip: tip,
      enabled: enabled,
      onPressed: onPressed,
    );
  }

  // A thin vertical rule between toolbar button groups.
  Widget _toolSep() => Container(
        width: 1,
        height: 22,
        margin: const EdgeInsets.symmetric(horizontal: 6),
        color: _cLine,
      );

  // A compact toolbar text field for the find / replace inputs.
  Widget _findField(TextEditingController ctrl, String hint) => SizedBox(
        width: 96,
        child: Container(
          height: 30,
          padding: const EdgeInsets.symmetric(horizontal: 8),
          decoration: BoxDecoration(
            color: _cField,
            borderRadius: BorderRadius.circular(5),
            border: Border.all(color: _cLineStrong),
          ),
          alignment: Alignment.centerLeft,
          child: TextField(
            controller: ctrl,
            style: const TextStyle(color: _cInk, fontSize: 12, fontFamily: _mono),
            cursorColor: _cAccent,
            decoration: InputDecoration(
              isCollapsed: true,
              border: InputBorder.none,
              hintText: hint,
              hintStyle: const TextStyle(color: Color(0xFF5F6672), fontSize: 12, fontFamily: _mono),
            ),
          ),
        ),
      );

  @override
  Widget build(BuildContext context) {
    return Container(
      color: _cBg,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _formulaBar(),
          _header(),
          Expanded(child: _bodyRow()),
          _sheetTabs(),
          _statusBar(),
        ],
      ),
    );
  }

  // ── Formula bar: a panel holding the address pill, an `fx` marker, the
  // editable source line (with an accent focus ring), and segmented button
  // groups (drag-fill · clipboard · file · history) divided by thin rules. ──
  Widget _formulaBar() {
    final focused = _formulaFocus.hasFocus;
    return Container(
      margin: const EdgeInsets.fromLTRB(10, 10, 10, 6),
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: _cPanel,
        border: Border.all(color: _cLine),
        borderRadius: BorderRadius.circular(8),
      ),
      // The toolbar holds many controls; let it scroll horizontally so it never
      // overflows on a narrow window.
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
        children: [
          // Address pill.
          Container(
            width: 46,
            height: 30,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              color: _cSurface,
              border: Border.all(color: _cLineStrong),
              borderRadius: BorderRadius.circular(5),
            ),
            child: Text(
              _model.infAddress,
              style: const TextStyle(
                  color: _cInk,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                  fontFamily: _mono),
            ),
          ),
          const SizedBox(width: 6),
          const Text('fx',
              style: TextStyle(
                  color: _cMuted,
                  fontSize: 12,
                  fontStyle: FontStyle.italic,
                  fontFamily: _mono)),
          const SizedBox(width: 6),
          // Formula field — accent focus ring on edit. A fixed width (rather than
          // Expanded) so it composes inside the horizontally-scrolling toolbar.
          SizedBox(
            width: 280,
            child: Container(
              height: 30,
              padding: const EdgeInsets.symmetric(horizontal: 8),
              decoration: BoxDecoration(
                color: _cField,
                borderRadius: BorderRadius.circular(5),
                border: Border.all(
                  color: focused ? _cAccent : _cLineStrong,
                  width: focused ? 2 : 1,
                ),
              ),
              alignment: Alignment.centerLeft,
              child: TextField(
                key: const Key('formulaField'),
                controller: _formulaCtrl,
                focusNode: _formulaFocus,
                onSubmitted: (_) => _commit(),
                style: const TextStyle(color: _cInk, fontSize: 13, fontFamily: _mono),
                cursorColor: _cAccent,
                decoration: const InputDecoration(
                  isCollapsed: true,
                  border: InputBorder.none,
                ),
              ),
            ),
          ),
          const SizedBox(width: 6),
          // ── Drag-fill ──
          _toolButton('↓ Fill 10',
              'Replicate the selected cell into the 10 rows below it', _fillDown),
          _toolSep(),
          // ── Clipboard ──
          _toolButton('Copy', 'Copy the selected cell to the clipboard', _copy),
          const SizedBox(width: 6),
          _toolButton('Cut', 'Cut the selected cell (cleared when you paste)', _cut),
          const SizedBox(width: 6),
          _toolButton('Paste',
              'Paste the clipboard at the selected cell, shifting relative references',
              _paste),
          _toolSep(),
          // ── File (save / load to memory) ──
          _toolButton('Save', 'Serialize the whole workbook to memory', _save),
          const SizedBox(width: 6),
          _toolButton('Load', 'Restore the workbook from the last save', _load,
              enabled: _savedSnapshot.isNotEmpty),
          _toolSep(),
          // ── File → open / save a REAL file on disk (over the engine's byte
          //    codecs, across dart:ffi): an .xlsx (ZIP, live formulas) and a
          //    .csv (text, values only). ──
          _toolButton('Save .xlsx',
              'Write the workbook to visicalc-demo.xlsx (a real ZIP; keeps live formulas)',
              () => _exportFile('xlsx', 'xlsx')),
          const SizedBox(width: 6),
          _toolButton('Open .xlsx',
              'Reopen visicalc-demo.xlsx over the engine (reloads live formulas)',
              () => _openFile('xlsx', 'xlsx')),
          const SizedBox(width: 6),
          _toolButton('Save .csv',
              'Write the workbook to visicalc-demo.csv (values only)',
              () => _exportFile('csv', 'csv')),
          const SizedBox(width: 6),
          _toolButton('Open .csv',
              'Reopen visicalc-demo.csv over the engine (values only)',
              () => _openFile('csv', 'csv')),
          _toolSep(),
          // ── Structure (insert / delete the selected row or column) ──
          _toolButton('+ Row',
              'Insert a row above the selected cell (references shift down)', _insertRow),
          const SizedBox(width: 6),
          _toolButton('− Row',
              "Delete the selected cell's row (references shift up; refs into it become #REF!)",
              _deleteRow),
          const SizedBox(width: 6),
          _toolButton('+ Col',
              'Insert a column left of the selected cell (references shift right)', _insertCol),
          const SizedBox(width: 6),
          _toolButton('− Col',
              "Delete the selected cell's column (references shift left; refs into it become #REF!)",
              _deleteCol),
          _toolSep(),
          // ── Format (apply a number format to the selected cell) ──
          _toolButton('.00',
              'Format the selected cell with thousands separators + 2 decimals (#,##0.00)',
              () => _applyFormat('#,##0.00')),
          const SizedBox(width: 6),
          _toolButton('%', 'Format the selected cell as a percent (0.0%)',
              () => _applyFormat('0.0%')),
          const SizedBox(width: 6),
          _toolButton('\$', 'Format the selected cell as currency (\$#,##0.00)',
              () => _applyFormat('\$#,##0.00')),
          const SizedBox(width: 6),
          _toolButton('Gen', "Clear the selected cell's format (General)",
              () => _applyFormat('')),
          _toolSep(),
          // ── Sort (reorder the budget block A1:E4 by the selected column) ──
          _toolButton('▲ Sort',
              'Sort the budget block A1:E4 by the selected column, ascending (rows move as records; formulas track)',
              () => _sortBlock(true)),
          const SizedBox(width: 6),
          _toolButton('▼ Sort',
              'Sort the budget block A1:E4 by the selected column, descending',
              () => _sortBlock(false)),
          _toolSep(),
          // ── Find / replace (locate cells by text; bulk-edit their source) ──
          _findField(_findCtrl, 'find'),
          const SizedBox(width: 6),
          _findField(_replaceCtrl, 'replace'),
          const SizedBox(width: 6),
          _toolButton('Find',
              'Select the first cell whose source contains the find text', _find),
          const SizedBox(width: 6),
          _toolButton('Replace all',
              "Replace the find text with the replacement in every matching cell's source",
              _replaceAll),
          _toolSep(),
          // ── History (undo / redo) ──
          _toolButton('↶ Undo', 'Undo the last edit', _undo,
              enabled: _model.canUndo),
          const SizedBox(width: 6),
          _toolButton('↷ Redo', 'Redo the last undone edit', _redo,
              enabled: _model.canRedo),
        ],
        ),
      ),
    );
  }

  // ── Sheet tab bar (Excel-style, along the bottom) ──
  // One chip per sheet; the active one tints to the accent. Tap to switch,
  // double-tap to rename, right-click (or long-press) to delete; the trailing
  // "+" adds a sheet. A formula like `=Summary!B3` reaches across the tabs, so
  // switching sheets and editing here updates every cross-sheet dependent live.
  Widget _sheetTabs() {
    final names = _model.sheetNames;
    final active = _model.activeSheet;
    return Container(
      height: 34,
      margin: const EdgeInsets.fromLTRB(10, 4, 10, 0),
      decoration: const BoxDecoration(
        border: Border(top: BorderSide(color: _cLine)),
      ),
      child: Row(
        children: [
          Expanded(
            child: ListView.builder(
              scrollDirection: Axis.horizontal,
              itemCount: names.length,
              itemBuilder: (_, i) {
                final selected = i == active;
                return Padding(
                  padding: const EdgeInsets.only(right: 4, top: 4),
                  child: Tooltip(
                    message: 'Tap to switch · double-tap to rename · right-click to delete',
                    child: GestureDetector(
                      onTap: () => _selectSheet(i),
                      onDoubleTap: () => _renameSheet(i),
                      onSecondaryTap: () => _deleteSheet(i),
                      onLongPress: () => _deleteSheet(i),
                      child: Container(
                        key: Key('sheetTab_$i'),
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        alignment: Alignment.center,
                        decoration: BoxDecoration(
                          color: selected ? _cSel : _cSurface,
                          borderRadius: const BorderRadius.vertical(top: Radius.circular(6)),
                          border: Border.all(
                            color: selected ? _cAccent : _cLineStrong,
                            width: selected ? 2 : 1,
                          ),
                        ),
                        child: Text(
                          names[i],
                          style: TextStyle(
                            color: selected ? Colors.white : _cInk,
                            fontSize: 12,
                            fontWeight: selected ? FontWeight.bold : FontWeight.normal,
                            fontFamily: _mono,
                          ),
                        ),
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
          const SizedBox(width: 6),
          _toolButton('+ Sheet', 'Add a new sheet', _addSheet),
        ],
      ),
    );
  }

  // ── Status line: a hairline-separated footer echoing the live virtual-grid
  // size and the per-edit revision clock (mirrors the web/Qt demos). ──
  Widget _statusBar() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(height: 1, margin: const EdgeInsets.symmetric(horizontal: 10), color: _cLine),
        Padding(
          padding: const EdgeInsets.fromLTRB(10, 6, 10, 10),
          child: Text(
            'Virtual grid: ${_model.totalRows} rows × ${_model.totalCols} cols'
            '  ·  revision ${_model.revision}'
            '${_fileMsg.isEmpty ? '' : '  ·  $_fileMsg'}',
            style: const TextStyle(color: _cMuted, fontSize: 12, fontFamily: _mono),
          ),
        ),
      ],
    );
  }

  // ── Column-letter header (frozen vertically, follows horizontal scroll) ──
  // The selected column's header tints to the accent so the cursor's column
  // reads at a glance.
  Widget _header() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: Row(
        children: [
          _chromeCell(_gutterW, _headH, ''), // corner
          Expanded(
            child: SizedBox(
              height: _headH,
              child: ListView.builder(
                controller: _headerH,
                scrollDirection: Axis.horizontal,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: _model.totalCols,
                itemBuilder: (_, i) => _chromeCell(
                    _colW, _headH, _model.columnLetters(i + 1),
                    selected: _model.selCol == i + 1),
              ),
            ),
          ),
        ],
      ),
    );
  }

  // ── Body: row-number gutter + virtualized cell grid ──
  Widget _bodyRow() {
    return Padding(
      padding: const EdgeInsets.fromLTRB(10, 0, 10, 0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Gutter: its own virtualized ListView, non-interactive, slaved to
          // the body's vertical scroll. The selected row's label tints to accent.
          SizedBox(
            width: _gutterW,
            child: ListView.builder(
              controller: _gutterV,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: _model.totalRows,
              itemExtent: _rowH,
              itemBuilder: (_, i) => _chromeCell(_gutterW, _rowH, '${i + 1}',
                  selected: _model.selRow == i + 1),
            ),
          ),
          // Cells: a horizontal scroll view supplies left/right pan; the
          // vertical ListView.builder inside it supplies up/down scroll + row
          // virtualization. The inner list is as wide as the whole column span.
          Expanded(
            child: SingleChildScrollView(
              controller: _bodyH,
              scrollDirection: Axis.horizontal,
              child: SizedBox(
                width: _model.totalCols * _colW,
                child: ListView.builder(
                  controller: _bodyV,
                  itemCount: _model.totalRows,
                  itemExtent: _rowH,
                  itemBuilder: (_, i) => _dataRow(i + 1),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// One body row (1-based). A single engine read for the whole row via
  /// [InfiniteSheetModel.rowCells]; each cell is tap-to-select.
  Widget _dataRow(int rowNum) {
    final cells = _model.rowCells(rowNum);
    // Zebra: even rows take the panel tint, odd rows the base cell color.
    final band = rowNum.isEven ? _cPanel : _cBg;
    return Row(
      children: List.generate(_model.totalCols, (i) {
        final colNum = i + 1;
        final text = i < cells.length ? cells[i] : '';
        final selected = _model.selRow == rowNum && _model.selCol == colNum;
        return GestureDetector(
          onTap: () => _select(rowNum, colNum),
          child: Container(
            width: _colW,
            height: _rowH,
            alignment: Alignment.centerRight,
            padding: const EdgeInsets.only(right: 6),
            decoration: BoxDecoration(
              // Selected → accent fill + 2px accent ring; else zebra band.
              color: selected ? _cSel : band,
              border: Border.all(
                color: selected ? _cAccent : _cLine,
                width: selected ? 2 : 0.5,
              ),
            ),
            child: Text(
              text,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: selected ? Colors.white : _cInk,
                fontSize: 12,
                fontWeight: selected ? FontWeight.bold : FontWeight.normal,
                fontFamily: _mono,
              ),
            ),
          ),
        );
      }),
    );
  }

  /// A frozen header/gutter cell (column letter, row number, or the corner).
  /// When [selected] (its row/column holds the cursor) it tints to the accent.
  Widget _chromeCell(double w, double h, String text, {bool selected = false}) {
    return Container(
      width: w,
      height: h,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: selected ? _cHeadSel : _cHead,
        border: Border.all(color: _cLine, width: 0.5),
      ),
      child: Text(
        text,
        style: TextStyle(
          color: selected ? _cAccent : _cMuted,
          fontSize: 11,
          fontWeight: FontWeight.bold,
          fontFamily: _mono,
        ),
      ),
    );
  }
}

/// A compact, modern toolbar button — a rounded chip with hover / pressed /
/// disabled states, the Flutter analog of the web demo's segmented controls and
/// the Qt port's `component ToolButton`. Stateful only to track hover/press so
/// the chip can lift on hover and sink on press, like the other backends.
class _ToolButton extends StatefulWidget {
  const _ToolButton({
    required this.label,
    required this.tip,
    required this.enabled,
    required this.onPressed,
  });

  final String label;
  final String tip;
  final bool enabled;
  final VoidCallback onPressed;

  @override
  State<_ToolButton> createState() => _ToolButtonState();
}

class _ToolButtonState extends State<_ToolButton> {
  bool _hover = false;
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final enabled = widget.enabled;
    final bg = !enabled
        ? _cSurface
        : _down
            ? _cSurfaceDown
            : _hover
                ? _cSurfaceHover
                : _cSurface;
    // Disabled chips read dimmer; the wrapping Opacity adds the rest.
    final fg = enabled ? (_hover ? Colors.white : _cInk) : _cMuted;

    return Tooltip(
      message: widget.tip,
      child: MouseRegion(
        cursor: enabled ? SystemMouseCursors.click : SystemMouseCursors.basic,
        onEnter: (_) => setState(() => _hover = true),
        onExit: (_) => setState(() {
          _hover = false;
          _down = false;
        }),
        child: GestureDetector(
          onTapDown: enabled ? (_) => setState(() => _down = true) : null,
          onTapUp: enabled ? (_) => setState(() => _down = false) : null,
          onTapCancel: enabled ? () => setState(() => _down = false) : null,
          onTap: enabled ? widget.onPressed : null,
          child: Opacity(
            opacity: enabled ? 1.0 : 0.6,
            child: Container(
              height: 30,
              padding: const EdgeInsets.symmetric(horizontal: 11),
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: bg,
                borderRadius: BorderRadius.circular(5),
                border: Border.all(color: _cLineStrong),
              ),
              child: Text(
                widget.label,
                style: TextStyle(color: fg, fontSize: 12, fontFamily: _mono),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
