// Grid.dart — hand-written placeholder (Flutter / VC2-flutter).
//
// The mosaic-emit-flutter pipeline emits a placeholder
// `SizedBox.shrink()` for the `Grid` built-in primitive — only the
// React emitter knows how to lower it into a real <table>. Until the
// Flutter Grid emitter lands, this file is a HAND-WRITTEN
// approximation of what the eventual auto-generated widget should
// look like. The shape mirrors the React emitter's output:
//
//   - 5-column DataTable / Container-of-Rows
//   - sticky-header semantics (Flutter approximates with a fixed
//     header row above a scrollable body)
//   - excel-blue (#264f78 on dark, #cce5ff on light) selected cell
//   - alternating zebra-stripe body rows
//
// Slots are passed in by the host but the data is hard-coded to a
// 5×5 sample spreadsheet — same data as VC2-html and VC2-webcomp so
// all three demos look visually identical.
//
// When the Flutter Grid emitter lands, replace this file with the
// auto-generated output (build.sh will overwrite it).

import 'package:flutter/material.dart';

sealed class GridEvent {
  const GridEvent();
}

class GridEventNavigate extends GridEvent {
  final int row;
  final int col;
  const GridEventNavigate({required this.row, required this.col});
}

/// Visual approximation of the Mosaic Grid component for VC2-flutter.
/// Same slot interface as the future auto-generated Grid widget —
/// host code passes in column-headers, viewport-rows, selected-row/
/// col, and dispatches navigate events. Hard-coded styling matches
/// Grid.dark.msl's palette so the look is identical to VC2-html.
class Grid extends StatelessWidget {
  final List<String> columnHeaders;
  final List<List<String>> viewportRows;
  final List<double> columnWidths;
  final double totalHeight;
  final int selectedRow;
  final int selectedCol;
  final int editRow;
  final int editCol;
  final void Function(GridEvent) dispatch;

  const Grid({
    super.key,
    required this.columnHeaders,
    required this.viewportRows,
    required this.columnWidths,
    required this.totalHeight,
    required this.selectedRow,
    required this.selectedCol,
    required this.editRow,
    required this.editCol,
    required this.dispatch,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: const BoxDecoration(
        color: Color(0xFF1E1E1E),
        border: Border(
          top: BorderSide(color: Color(0xFF3F3F46)),
        ),
      ),
      constraints: BoxConstraints(maxHeight: totalHeight),
      child: SingleChildScrollView(
        child: Column(
          children: [
            // Header row — pinned (would-be-sticky in a fuller impl).
            _HeaderRow(headers: columnHeaders),
            // Body rows.
            for (int r = 0; r < viewportRows.length; r++)
              _DataRow(
                rowIndex: r,
                cells: viewportRows[r],
                isEven: r % 2 == 0,
                selectedCol: r == selectedRow ? selectedCol : -1,
                editingCol: r == editRow ? editCol : -1,
                onTap: (c) => dispatch(GridEventNavigate(row: r, col: c)),
              ),
          ],
        ),
      ),
    );
  }
}

class _HeaderRow extends StatelessWidget {
  final List<String> headers;
  const _HeaderRow({required this.headers});

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 24,
      decoration: const BoxDecoration(
        color: Color(0xFF2D2D30),
      ),
      child: Row(
        children: [
          // Leading row-label cell (empty).
          _HeaderCell(label: ''),
          for (final h in headers) _HeaderCell(label: h),
        ],
      ),
    );
  }
}

class _HeaderCell extends StatelessWidget {
  final String label;
  const _HeaderCell({required this.label});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 96,
      height: 24,
      alignment: Alignment.center,
      decoration: const BoxDecoration(
        color: Color(0xFF2D2D30),
        border: Border(
          right: BorderSide(color: Color(0xFF3F3F46)),
          bottom: BorderSide(color: Color(0xFF3F3F46)),
        ),
      ),
      child: Text(
        label,
        style: const TextStyle(
          color: Color(0xFF9D9D9D),
          fontSize: 12,
          fontFamily: 'monospace',
        ),
      ),
    );
  }
}

class _DataRow extends StatelessWidget {
  final int rowIndex;
  final List<String> cells;
  final bool isEven;
  final int selectedCol;
  final int editingCol;
  final void Function(int) onTap;

  const _DataRow({
    required this.rowIndex,
    required this.cells,
    required this.isEven,
    required this.selectedCol,
    required this.editingCol,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final bgColor = isEven ? const Color(0xFF1E1E1E) : const Color(0xFF252526);
    return Container(
      height: 22,
      color: bgColor,
      child: Row(
        children: [
          // Leading row-label cell ("1", "2", …).
          _HeaderCell(label: '${rowIndex + 1}'),
          for (int c = 0; c < cells.length; c++)
            _DataCell(
              text: cells[c],
              isSelected: c == selectedCol,
              isEditing: c == editingCol,
              onTap: () => onTap(c),
            ),
        ],
      ),
    );
  }
}

class _DataCell extends StatelessWidget {
  final String text;
  final bool isSelected;
  final bool isEditing;
  final VoidCallback onTap;

  const _DataCell({
    required this.text,
    required this.isSelected,
    required this.isEditing,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    Color? cellBg;
    Color textColor = const Color(0xFFCCCCCC);
    if (isSelected) {
      cellBg = const Color(0xFF264F78);
      textColor = Colors.white;
    } else if (isEditing) {
      cellBg = const Color(0xFF1F4F3F);
    }

    return InkWell(
      onTap: onTap,
      child: Container(
        width: 96,
        height: 22,
        padding: const EdgeInsets.symmetric(horizontal: 2),
        alignment: Alignment.centerRight,
        decoration: BoxDecoration(
          color: cellBg,
          border: Border(
            right: const BorderSide(color: Color(0xFF3F3F46)),
            bottom: const BorderSide(color: Color(0xFF3F3F46)),
            // Selected cell gets an accent-coloured outline.
            top: isSelected
                ? const BorderSide(color: Color(0xFF007ACC))
                : BorderSide.none,
            left: isSelected
                ? const BorderSide(color: Color(0xFF007ACC))
                : BorderSide.none,
          ),
        ),
        child: Text(
          text,
          style: TextStyle(
            color: textColor,
            fontSize: 12,
            fontFamily: 'monospace',
          ),
        ),
      ),
    );
  }
}
