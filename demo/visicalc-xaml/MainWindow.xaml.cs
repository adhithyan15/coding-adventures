// MainWindow.xaml.cs — host shell code-behind for VC2-xaml.
//
// Wires the FormulaBar's slot properties + event dispatch, and
// builds the hand-written 5x5 sample grid programmatically so the
// XAML markup stays small. State held in private fields; setters
// re-trigger property change on the FormulaBar.
//
// Same hard-coded 5x5 sample data as VC2-html / VC2-webcomp /
// VC2-flutter / VC2-qt / VC2-swiftui.

using System;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

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

    private int _selectedRow = 0;
    private int _selectedCol = 0;
    private string _formula = "=SUM(B1:B5)";

    private string CellAddress => $"{(char)('A' + _selectedCol)}{_selectedRow + 1}";

    public MainWindow()
    {
        InitializeComponent();
        WireFormulaBar();
        BuildSampleGrid();
    }

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

    private void BuildSampleGrid()
    {
        // Add column + row definitions: 1 row-label col + 5 data cols,
        // 1 header row + 5 data rows.
        for (int c = 0; c < 6; c++)
        {
            SheetGrid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(96) });
        }
        for (int r = 0; r < 6; r++)
        {
            SheetGrid.RowDefinitions.Add(new RowDefinition { Height = new GridLength(r == 0 ? 24 : 22) });
        }

        // Header row: empty corner cell + "A".."E".
        AddCell(SheetGrid, 0, 0, "",  isHeader: true);
        for (int c = 0; c < 5; c++)
        {
            AddCell(SheetGrid, 0, c + 1, ((char)('A' + c)).ToString(), isHeader: true);
        }

        // Data rows: row label "1".."5" + cells.
        for (int r = 0; r < 5; r++)
        {
            AddCell(SheetGrid, r + 1, 0, (r + 1).ToString(), isHeader: true);
            for (int c = 0; c < 5; c++)
            {
                AddCell(SheetGrid, r + 1, c + 1, SampleRows[r][c],
                        isHeader: false,
                        onTap: (capR, capC) => SelectCell(capR, capC),
                        capturedRow: r, capturedCol: c);
            }
        }
    }

    private void AddCell(
        Grid parent,
        int row,
        int col,
        string text,
        bool isHeader,
        Action<int, int>? onTap = null,
        int capturedRow = -1,
        int capturedCol = -1)
    {
        var border = new Border
        {
            BorderBrush = new SolidColorBrush(Color.FromArgb(0xFF, 0x3F, 0x3F, 0x46)),
            BorderThickness = new Thickness(1),
            Background = new SolidColorBrush(isHeader
                ? Color.FromArgb(0xFF, 0x2D, 0x2D, 0x30)
                : Color.FromArgb(0xFF, 0x1E, 0x1E, 0x1E)),
        };

        var tb = new TextBlock
        {
            Text = text,
            FontFamily = new FontFamily("Consolas"),
            FontSize = 12,
            Foreground = new SolidColorBrush(isHeader
                ? Color.FromArgb(0xFF, 0x9D, 0x9D, 0x9D)
                : Color.FromArgb(0xFF, 0xCC, 0xCC, 0xCC)),
            HorizontalAlignment = isHeader ? HorizontalAlignment.Center : HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(4, 0, 4, 0),
        };
        border.Child = tb;

        if (onTap != null)
        {
            int r = capturedRow, c = capturedCol;
            border.PointerPressed += (s, e) => onTap(r, c);
        }

        Grid.SetRow(border, row);
        Grid.SetColumn(border, col);
        parent.Children.Add(border);
    }

    private void SelectCell(int r, int c)
    {
        _selectedRow = r;
        _selectedCol = c;
        _formula = SampleRows[r][c];
        // Refresh FormulaBar bindings.
        FormulaBarControl.CellAddress = CellAddress;
        FormulaBarControl.Formula = _formula;
        // (Cell-highlight repaint is a follow-up — would walk
        // SheetGrid.Children to find the selected cell and update
        // its Background + outline.)
    }
}
