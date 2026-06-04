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
import 'generated/formula_bar.dart';
import 'generated/grid.dart';

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
  // Hard-coded 5×5 sample spreadsheet — same data as VC2-html /
  // VC2-webcomp so the demos look visually identical across backends.
  static const _sampleRows = [
    ['15', '3', '12', '8', '5'],
    ['8', '14', '7', '22', '11'],
    ['12', '9', '18', '6', '25'],
    ['4', '11', '3', '17', '9'],
    ['7', '5', '13', '10', '19'],
  ];

  String _formula = '=SUM(B1:B5)';
  // The generated Grid widget uses `double` for every numeric
  // coordinate (the Flutter emitter lowers `number` slots to
  // `double` so verbatim expressions like
  // `r == editRow && c == editCol` line up across backends).
  // The host mirrors that on its state so the dispatch wiring
  // is a clean pass-through.
  double _selectedRow = 0;
  double _selectedCol = 0;
  double _editRow = -1;
  double _editCol = -1;
  String _editContent = '';

  String get _cellAddress {
    final colLetter = String.fromCharCode(65 + _selectedCol.toInt());
    return '$colLetter${_selectedRow.toInt() + 1}';
  }

  void _onFormulaBarEvent(FormulaBarEvent event) {
    setState(() {
      switch (event) {
        case FormulaBarEventFormulaChange(:final value):
          _formula = value;
        case FormulaBarEventCommit():
          // No-op — the React demo would commit to cells[r][c]; the
          // Flutter demo just keeps the formula bar text.
          break;
        case FormulaBarEventCancel():
          // Reset to the cell's stored value.
          _formula = _sampleRows[_selectedRow.toInt()][_selectedCol.toInt()];
      }
    });
  }

  void _onGridEvent(GridEvent event) {
    setState(() {
      switch (event) {
        case GridEventNavigate(:final row, :final col):
          _selectedRow = row.toDouble();
          _selectedCol = col.toDouble();
          _formula = _sampleRows[row.toInt()][col.toInt()];
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
              const Padding(
                padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Text(
                  'VISICALC · MOSAIC FLUTTER DEMO',
                  style: TextStyle(
                    color: Color(0xFF9D9D9D),
                    fontSize: 11,
                    letterSpacing: 1.0,
                  ),
                ),
              ),
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
                    columnHeaders: const ['A', 'B', 'C', 'D', 'E'],
                    viewportRows: _sampleRows,
                    columnWidths: const [96, 96, 96, 96, 96],
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
          ),
        ),
      ),
    );
  }
}
