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

import 'package:flutter/material.dart';
import 'engine.dart';

/// Cell geometry (logical pixels). Shared by the header, gutter, and body so
/// their scroll offsets line up.
const double _rowH = 24;
const double _colW = 90;
const double _gutterW = 64;
const double _headH = 26;

const _bg = Color(0xFF1E1E1E);
const _chrome = Color(0xFF2D2D30);
const _border = Color(0xFF3F3F46);
const _ink = Color(0xFFCCCCCC);
const _dim = Color(0xFF9D9D9D);
const _sel = Color(0xFF094771);
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
  }

  @override
  void dispose() {
    _bodyV.dispose();
    _bodyH.dispose();
    _gutterV.dispose();
    _headerH.dispose();
    _formulaCtrl.dispose();
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

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _formulaBar(),
        _header(),
        Expanded(child: _bodyRow()),
      ],
    );
  }

  // ── Formula bar: the selected cell's address + an editable source line ──
  Widget _formulaBar() {
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 8, 8, 6),
      child: Row(
        children: [
          SizedBox(
            width: _gutterW,
            child: Text(
              _model.infAddress,
              style: const TextStyle(color: _dim, fontSize: 12, fontFamily: _mono),
            ),
          ),
          Expanded(
            child: Container(
              height: 28,
              padding: const EdgeInsets.symmetric(horizontal: 6),
              decoration: BoxDecoration(color: _chrome, border: Border.all(color: _border)),
              alignment: Alignment.centerLeft,
              child: TextField(
                controller: _formulaCtrl,
                onSubmitted: (_) => _commit(),
                style: const TextStyle(color: _ink, fontSize: 13, fontFamily: _mono),
                cursorColor: _ink,
                decoration: const InputDecoration(
                  isCollapsed: true,
                  border: InputBorder.none,
                ),
              ),
            ),
          ),
          const SizedBox(width: 8),
          // Drag-fill: replicate the selected cell into the 10 rows below it.
          Tooltip(
            message: 'Replicate the selected cell into the 10 rows below it',
            child: OutlinedButton(
              onPressed: _fillDown,
              style: OutlinedButton.styleFrom(
                foregroundColor: _ink,
                side: const BorderSide(color: _border),
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
              ),
              child: const Text('Fill ↓ 10', style: TextStyle(fontFamily: _mono, fontSize: 12)),
            ),
          ),
        ],
      ),
    );
  }

  // ── Column-letter header (frozen vertically, follows horizontal scroll) ──
  Widget _header() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
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
                itemBuilder: (_, i) =>
                    _chromeCell(_colW, _headH, _model.columnLetters(i + 1)),
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
      padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Gutter: its own virtualized ListView, non-interactive, slaved to
          // the body's vertical scroll.
          SizedBox(
            width: _gutterW,
            child: ListView.builder(
              controller: _gutterV,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: _model.totalRows,
              itemExtent: _rowH,
              itemBuilder: (_, i) => _chromeCell(_gutterW, _rowH, '${i + 1}'),
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
            padding: const EdgeInsets.only(right: 4),
            decoration: BoxDecoration(
              color: selected ? _sel : _bg,
              border: Border.all(color: _border, width: 0.5),
            ),
            child: Text(
              text,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(color: _ink, fontSize: 12, fontFamily: _mono),
            ),
          ),
        );
      }),
    );
  }

  /// A frozen header/gutter cell (column letter, row number, or the corner).
  Widget _chromeCell(double w, double h, String text) {
    return Container(
      width: w,
      height: h,
      alignment: Alignment.center,
      decoration: BoxDecoration(color: _chrome, border: Border.all(color: _border, width: 0.5)),
      child: Text(
        text,
        style: const TextStyle(color: _dim, fontSize: 12, fontFamily: _mono),
      ),
    );
  }
}
