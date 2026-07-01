// main.dart — VisiCalc Flutter host (VC2-flutter).
//
// Mounts the auto-generated `FormulaBar` AND `Grid` widgets, both
// produced by `mosaic-compile --backend flutter` from the shared
// `demo/visicalc/mosaic/{FormulaBar,Grid}.{mil,desktop.mll,dark.msl}`
// triples.  Grid.desktop.mll is a UI34
// `pkg::mosaic-pkg-grid::Grid` one-liner, so the widget's structure
// comes from the authoritative `mosaic-pkg-grid` composition.  No
// hand-written widgets in this file.
//
// State is held in a tiny `_AppState` ValueNotifier that pretends
// to be the host's reducer: it owns the formula text, the selected
// cell, the edit buffer, and a hard-coded 5×5 sample spreadsheet.
//
// Run:
//   flutter pub get
//   flutter run

import 'package:flutter/material.dart';
import 'engine.dart';
import 'generated/formula_bar.dart';
import 'generated/grid.dart';
import 'infinite_grid.dart';

void main() {
  runApp(const VisiCalcApp());
}

/// Top-level app — dark theme to match Grid.dark.msl / FormulaBar.dark.msl.
class VisiCalcApp extends StatelessWidget {
  const VisiCalcApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'VisiCalc — Mosaic Flutter demo',
      theme: ThemeData(
        brightness: Brightness.dark,
        scaffoldBackgroundColor: const Color(0xFF1E1E1E),
        textTheme: const TextTheme(
          bodyMedium: TextStyle(color: Color(0xFFCCCCCC)),
        ),
      ),
      home: const VisiCalcHome(),
    );
  }
}

/// The host shell — mounts FormulaBar above Grid, threads a tiny
/// state model that the components push events into. Mirrors the
/// React demo's reducer pattern at much smaller scale.
class VisiCalcHome extends StatefulWidget {
  const VisiCalcHome({super.key});

  @override
  State<VisiCalcHome> createState() => _VisiCalcHomeState();
}

class _VisiCalcHomeState extends State<VisiCalcHome> {
  // The 5×5 spreadsheet is no longer hard-coded: it's computed by the shared
  // Rust engine, reached through the C ABI via dart:ffi (see lib/engine.dart).
  // The model seeds the same cross-footing budget as every other VC2-* demo
  // (column E totals each row, row 5 totals each column, E5 = 169), and editing
  // the formula bar writes through to the engine and recomputes every cell.
  late final SpreadsheetModel _model = SpreadsheetModel();

  @override
  void initState() {
    super.initState();
    // Show the selected cell's source (A1 → "15") in the bar on launch.
    _formula = _model.rawAt(_selectedRow.toInt(), _selectedCol.toInt());
  }

  @override
  void dispose() {
    _model.dispose();
    super.dispose();
  }

  String _formula = '';
  // The generated Grid widget uses `double` for every numeric
  // coordinate (the Flutter emitter lowers `number` slots to
  // `double` so verbatim expressions like
  // `r == editRow && c == editCol` line up across backends).
  // The host mirrors that on its state so the dispatch wiring
  // is a clean pass-through.
  // Selection defaults to A1, which after the row-label gutter is
  // column index 1 (column 0 is the '1'..'5' row-number gutter).
  double _selectedRow = 0;
  double _selectedCol = 1;
  double _editRow = -1;
  double _editCol = -1;
  String _editContent = '';

  // Which view is showing: the classic 5×5 cross-foot budget (the auto-
  // generated Grid), or the virtualized infinite sheet (InfiniteGrid, rendered
  // on the same engine via the viewport primitive).
  bool _infinite = false;

  String get _cellAddress {
    // Column 0 is the row-label gutter; data columns A–E start at
    // index 1, so the letter is offset by one (`65 + col - 1`).
    final colLetter = String.fromCharCode(65 + _selectedCol.toInt() - 1);
    return '$colLetter${_selectedRow.toInt() + 1}';
  }

  void _onFormulaBarEvent(FormulaBarEvent event) {
    setState(() {
      switch (event) {
        case FormulaBarEventFormulaChange(:final value):
          _formula = value;
        case FormulaBarEventCommit():
          // Write the edited text to the engine and recompute every dependent
          // cell, then reflect the cell's (possibly canonicalised) source.
          _model.setCell(_selectedRow.toInt(), _selectedCol.toInt(), _formula);
          _formula = _model.rawAt(_selectedRow.toInt(), _selectedCol.toInt());
        case FormulaBarEventCancel():
          // Discard the edit: restore the cell's stored source from the engine.
          _formula = _model.rawAt(_selectedRow.toInt(), _selectedCol.toInt());
      }
    });
  }

  void _onGridEvent(GridEvent event) {
    setState(() {
      switch (event) {
        case GridEventNavigate(:final row, :final col):
          _selectedRow = row.toDouble();
          _selectedCol = col.toDouble();
          _formula = _model.rawAt(row.toInt(), col.toInt());
        case GridEventFormulaChange(:final value):
          _editContent = value;
        case GridEventEditCommit():
          _editRow = -1;
          _editCol = -1;
        case GridEventEditCancel():
          _editRow = -1;
          _editCol = -1;
          _editContent = '';
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 720),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Title bar + view toggle.
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        _infinite
                            ? 'VISICALC · INFINITE SHEET · RUST ENGINE'
                            : 'VISICALC · MOSAIC FLUTTER DEMO',
                        style: const TextStyle(
                          color: Color(0xFF9D9D9D),
                          fontSize: 11,
                          letterSpacing: 1.0,
                        ),
                      ),
                    ),
                    TextButton(
                      onPressed: () => setState(() => _infinite = !_infinite),
                      child: Text(_infinite ? 'Classic grid' : 'Infinite sheet'),
                    ),
                  ],
                ),
              ),
              // The infinite view owns its own model + chrome, so it replaces
              // the whole classic stack (formula bar + grid) when toggled on.
              if (_infinite)
                const Expanded(child: InfiniteGrid())
              else ...[
                FormulaBar(
                  cellAddress: _cellAddress,
                  formula: _formula,
                  readOnly: false,
                  dispatch: _onFormulaBarEvent,
                ),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    child: Grid(
                      // Leading '' is the row-label gutter header (above
                      // the '1'..'5' column); A–E label the data columns.
                      columnHeaders: const ['', 'A', 'B', 'C', 'D', 'E'],
                      viewportRows: _model.viewportRows,
                      // Narrow gutter (48) + five data columns (96 each).
                      columnWidths: const [48, 96, 96, 96, 96, 96],
                      totalHeight: 400,
                      selectedRow: _selectedRow,
                      selectedCol: _selectedCol,
                      editRow: _editRow,
                      editCol: _editCol,
                      editContent: _editContent,
                      dispatch: _onGridEvent,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
