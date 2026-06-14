// MainWindow.xaml.cs — host shell code-behind for VC2-xaml.
//
// Wires BOTH generated controls:
//   - FormulaBar : slot properties + Dispatch event.
//   - Grid       : ColumnHeaders / ViewportRows / ColumnWidths /
//                  Selected*/Edit*/EditContent dependency properties
//                  + a Dispatch handler for Navigate / FormulaChange /
//                  EditCommit / EditCancel.
//
// There is no hand-written grid anymore — `<gen:Grid>` is the
// auto-generated control mounted in MainWindow.xaml. This file only
// FEEDS it host state and reacts to its events.
//
// Same hard-coded 5x5 sample data as VC2-html / VC2-webcomp /
// VC2-flutter / VC2-qt / VC2-swiftui.

using System;
using System.Collections.Generic;
using Microsoft.UI.Xaml;

namespace Mosaic.Generated;

public partial class MainWindow : Window
{
    // 5x5 sample spreadsheet — same data as the other VC2-* demos.
    private static readonly string[][] SampleRows = new[]
    {
        new[] { "15", "3",  "12", "8",  "5"  },
        new[] { "8",  "14", "7",  "22", "11" },
        new[] { "12", "9",  "18", "6",  "25" },
        new[] { "4",  "11", "3",  "17", "9"  },
        new[] { "7",  "5",  "13", "10", "19" },
    };

    // Column headers: a blank gutter ("") + the five data columns.
    private static readonly string[] Headers = { "", "A", "B", "C", "D", "E" };

    // Fixed per-column pixel widths: 48 px gutter + 96 px data columns.
    // These feed BOTH the Grid's ColumnWidths DP and the per-cell
    // Grid_VVm.Width field the generated cell binds.
    private static readonly double[] Widths = { 48, 96, 96, 96, 96, 96 };

    private int _selectedRow;
    private int _selectedCol;
    private string _formula = "=SUM(B1:B5)";

    // Edit state mirrors the Grid.mil contract: editRow == -1 means
    // "not editing". The live edit buffer is _editContent.
    private int _editRow = -1;
    private int _editCol = -1;
    private string _editContent = string.Empty;

    private string CellAddress => $"{(char)('A' + _selectedCol)}{_selectedRow + 1}";

    public MainWindow()
    {
        InitializeComponent();
        WireFormulaBar();
        WireGrid();
    }

    // ── FormulaBar ────────────────────────────────────────────────

    private void WireFormulaBar()
    {
        FormulaBarControl.CellAddress = CellAddress;
        FormulaBarControl.Formula = _formula;
        FormulaBarControl.ReadOnly = false;
        FormulaBarControl.Dispatch = HandleFormulaBarEvent;
    }

    private void HandleFormulaBarEvent(FormulaBarEvent evt)
    {
        switch (evt)
        {
            case FormulaBarEvent.FormulaChange c:
                _formula = c.Value;
                break;
            case FormulaBarEvent.Commit:
                // no-op for v1
                break;
            case FormulaBarEvent.Cancel:
                _formula = SampleRows[_selectedRow][_selectedCol];
                FormulaBarControl.Formula = _formula;
                break;
        }
    }

    // ── Grid ──────────────────────────────────────────────────────

    private void WireGrid()
    {
        // Simple (scalar / flat-list) dependency properties feed
        // straight through.
        SheetGrid.ColumnHeaders = Headers;
        SheetGrid.ColumnWidths = Widths;
        SheetGrid.SelectedRow = _selectedRow;
        SheetGrid.SelectedCol = _selectedCol;
        SheetGrid.EditRow = _editRow;
        SheetGrid.EditCol = _editCol;
        SheetGrid.EditContent = _editContent;

        // The generated Grid raises one GridEvent per user action.
        SheetGrid.Dispatch += HandleGridEvent;

        // ViewportRows: each row is prefixed with its 1-based label so
        // the gutter column shows "1".."5" alongside the five data
        // cells. The generated Grid's ViewportRows DP is typed
        // IReadOnlyList<IReadOnlyList<string>>, so this is the shape it
        // declares.
        SheetGrid.ViewportRows = BuildViewportRows();
    }

    // Build the viewport rows: prefix each data row with its label.
    //   row 0 -> [ "1", "15", "3", "12", "8", "5" ]
    private static IReadOnlyList<IReadOnlyList<string>> BuildViewportRows()
    {
        var rows = new List<IReadOnlyList<string>>(SampleRows.Length);
        for (int r = 0; r < SampleRows.Length; r++)
        {
            var row = new List<string> { (r + 1).ToString() };
            row.AddRange(SampleRows[r]);
            rows.Add(row);
        }
        return rows;
    }

    // ───────────────────────────────────────────────────────────────
    // WINDOWS-DEV TODO — per-cell VM projection (Group C population).
    // ───────────────────────────────────────────────────────────────
    //
    // The generated Grid's nested `ItemsRepeater`s bind:
    //   - the outer repeater to `ViewportRows`           (one Grid_RowVm
    //     per row, exposing `IReadOnlyList<string> Row`), and
    //   - the inner repeater to `Row`                    (one Grid_VVm
    //     per cell, exposing `string V`, `int Index`,
    //     `double Width`).
    //
    // The emitter generates the Grid_RowVm / Grid_VVm RECORD TYPES and
    // the cell binds `Width="{x:Bind Width}"`, but it does NOT generate
    // the code that PROJECTS the raw `IReadOnlyList<IReadOnlyList<string>>`
    // into those VM instances (see the <remarks> on Generated/Grid_VVm.cs).
    //
    // On Windows you have two options:
    //
    //   (a) Change the `ViewportRows` DP feed to a precomputed
    //       `List<Grid_RowVm>` where each `Grid_RowVm.Row` is a
    //       `List<Grid_VVm>` you build by zipping cell value + column
    //       index -> Widths[col]:
    //
    //         var vmRows = new List<Grid_RowVm>();
    //         foreach (var labelled in BuildViewportRows())
    //         {
    //             var cells = new List<Grid_VVm>();
    //             for (int col = 0; col < labelled.Count; col++)
    //                 cells.Add(new Grid_VVm(labelled[col], col, Widths[col]));
    //             vmRows.Add(new Grid_RowVm(cells, vmRows.Count));
    //         }
    //
    //       …and widen the Grid's `ViewportRows`/`Row` types to the VM
    //       records (an emitter follow-up: have the DP generator type
    //       the slot from the resolved VM rather than the raw .mil slot
    //       type). Until that lands the DP is the raw string-list type,
    //       so the inner `x:Bind V/Width` cannot resolve at runtime.
    //
    //   (b) Wait for the emitter to thread the VM-projection itself
    //       (the per-cell `R`/`C` predicate threading — see the
    //       deferred state-highlight item — lands in the same change).
    //
    // This macOS checkout cannot run `dotnet build`, so the projection
    // is left for the Windows dev to wire per the recipe above.

    private void HandleGridEvent(object? sender, GridEvent evt)
    {
        switch (evt)
        {
            case GridEvent.Navigate n:
                SelectCell((int)n.Row, (int)n.Col);
                break;

            case GridEvent.FormulaChange c:
                // Each keystroke inside the inline cell editor updates
                // the live edit buffer the Grid reflects back.
                _editContent = c.Value;
                SheetGrid.EditContent = _editContent;
                break;

            case GridEvent.EditCommit:
                // Enter — persist the buffered edit into the cell and
                // leave edit mode.
                if (_editRow >= 0 && _editCol >= 0)
                {
                    SampleRows[_editRow][_editCol] = _editContent;
                }
                EndEdit();
                break;

            case GridEvent.EditCancel:
                // Escape — discard the buffer, leave edit mode.
                EndEdit();
                break;
        }
    }

    private void SelectCell(int r, int c)
    {
        _selectedRow = r;
        _selectedCol = c;
        // The data cell at (r, c) is SampleRows[r][c]; the Grid's first
        // column is the row-label gutter, so column 0 of the viewport
        // is the label and data columns are 1..5.
        _formula = SampleRows[r][c];

        SheetGrid.SelectedRow = r;
        SheetGrid.SelectedCol = c;

        // Refresh FormulaBar bindings.
        FormulaBarControl.CellAddress = CellAddress;
        FormulaBarControl.Formula = _formula;
    }

    private void EndEdit()
    {
        _editRow = -1;
        _editCol = -1;
        _editContent = string.Empty;
        SheetGrid.EditRow = _editRow;
        SheetGrid.EditCol = _editCol;
        SheetGrid.EditContent = _editContent;
        // Re-feed the (possibly mutated) viewport so committed edits show.
        SheetGrid.ViewportRows = BuildViewportRows();
    }
}
