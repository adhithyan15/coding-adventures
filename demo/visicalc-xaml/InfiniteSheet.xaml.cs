// InfiniteSheet.xaml.cs — code-behind for the virtualized infinite-sheet view.
//
// Renders an effectively-infinite (u32 × u32, sparse) sheet on the shared Rust
// engine through InfiniteSheetModel (Engine.cs), which reads the engine's
// viewport primitive over P/Invoke. The .NET sibling of the SwiftUI / Qt /
// Flutter / Compose infinite views.
//
// Virtualization: BodyList is a ListView whose ItemsSource is just the row
// numbers (1..TotalRows) — a lightweight int list. The ListView's ItemsStackPanel
// realizes a container only for on-screen rows; ContainerContentChanging fills
// each realized row's cells from one engine get_display_window over its
// 1×TotalCols strip (model.RowCells) — display strings, already rendered through
// each cell's format code. So building the UI costs only the visible rows, never the
// whole (possibly millions-tall) sheet. The gutter is a second virtualized
// ListView over the same row numbers, and the header is a one-time StackPanel of
// column letters; both are kept in sync with the body's scroll offsets.
//
// WinUI 3 is Windows-only — this builds and runs on Windows. The engine-backed
// logic it drives (InfiniteSheetModel) is proven cross-platform by the headless
// test (test/Program.cs via scripts/verify.sh).

using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.UI;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.System;
using Windows.UI;

namespace Mosaic.Generated;

public sealed partial class InfiniteSheet : UserControl
{
    private const double ColW = 90, RowH = 24, GutterW = 64, HeadH = 26;

    private static readonly SolidColorBrush Bg = New(0x1E, 0x1E, 0x1E);
    private static readonly SolidColorBrush Chrome = New(0x2D, 0x2D, 0x30);
    private static readonly SolidColorBrush BorderC = New(0x3F, 0x3F, 0x46);
    private static readonly SolidColorBrush Ink = New(0xCC, 0xCC, 0xCC);
    private static readonly SolidColorBrush Dim = New(0x9D, 0x9D, 0x9D);
    private static readonly SolidColorBrush Sel = New(0x09, 0x47, 0x71);
    private static readonly FontFamily Mono = new("Consolas");

    private readonly InfiniteSheetModel _model = new();
    private List<int> _rowNumbers = new();
    private ScrollViewer? _bodyInnerSv, _gutterInnerSv;

    public InfiniteSheet()
    {
        InitializeComponent();

        _rowNumbers = Enumerable.Range(1, _model.TotalRows).ToList();
        BodyList.ItemsSource = _rowNumbers;
        GutterList.ItemsSource = _rowNumbers;
        BodyList.Width = _model.TotalCols * ColW;

        BuildHeader();
        RefreshFormulaBar();

        Loaded += OnLoaded;
    }

    private static SolidColorBrush New(byte r, byte g, byte b) =>
        new(Color.FromArgb(0xFF, r, g, b));

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        // The body's own (inner) vertical ScrollViewer drives the gutter; the
        // outer horizontal ScrollViewer (BodyHScroll) drives the header.
        _bodyInnerSv = FindScrollViewer(BodyList);
        _gutterInnerSv = FindScrollViewer(GutterList);
        if (_bodyInnerSv is not null)
            _bodyInnerSv.ViewChanged += (_, _) =>
                _gutterInnerSv?.ChangeView(null, _bodyInnerSv.VerticalOffset, null, true);
    }

    // ── Header (built once) ──────────────────────────────────────────
    private void BuildHeader()
    {
        for (int c = 1; c <= _model.TotalCols; c++)
            HeaderPanel.Children.Add(ChromeCell(ColW, HeadH, _model.ColumnLetters(c)));
    }

    // ── Virtualized body + gutter population ─────────────────────────
    private void BodyList_ContainerContentChanging(ListViewBase sender, ContainerContentChangingEventArgs args)
    {
        if (args.InRecycleQueue) return;
        int rowNum = (int)args.Item;
        args.ItemContainer.Content = BuildRow(rowNum);
        args.Handled = true;
    }

    private void GutterList_ContainerContentChanging(ListViewBase sender, ContainerContentChangingEventArgs args)
    {
        if (args.InRecycleQueue) return;
        int rowNum = (int)args.Item;
        args.ItemContainer.Content = ChromeCell(GutterW, RowH, rowNum.ToString());
        args.Handled = true;
    }

    /// One body row: a horizontal strip of tappable cells. A single engine read
    /// (RowCells) fills the whole row.
    private StackPanel BuildRow(int rowNum)
    {
        var sp = new StackPanel { Orientation = Orientation.Horizontal };
        IReadOnlyList<string> cells = _model.RowCells(rowNum);
        for (int c = 1; c <= _model.TotalCols; c++)
        {
            string text = (c - 1) < cells.Count ? cells[c - 1] : string.Empty;
            bool selected = _model.SelRow == rowNum && _model.SelCol == c;
            int col = c; // capture for the handler
            var cell = new Border
            {
                Width = ColW,
                Height = RowH,
                Background = selected ? Sel : Bg,
                BorderBrush = BorderC,
                BorderThickness = new Thickness(0.5),
                Child = new TextBlock
                {
                    Text = text,
                    Foreground = Ink,
                    FontSize = 12,
                    FontFamily = Mono,
                    HorizontalAlignment = HorizontalAlignment.Right,
                    VerticalAlignment = VerticalAlignment.Center,
                    Margin = new Thickness(0, 0, 4, 0),
                    TextTrimming = TextTrimming.CharacterEllipsis,
                },
            };
            cell.Tapped += (_, _) => Select(rowNum, col);
            sp.Children.Add(cell);
        }
        return sp;
    }

    private Border ChromeCell(double w, double h, string text) => new()
    {
        Width = w,
        Height = h,
        Background = Chrome,
        BorderBrush = BorderC,
        BorderThickness = new Thickness(0.5),
        Child = new TextBlock
        {
            Text = text,
            Foreground = Dim,
            FontSize = 12,
            FontFamily = Mono,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
        },
    };

    // ── Selection + edit ─────────────────────────────────────────────
    private void Select(int rowNum, int col)
    {
        _model.SelectInf(rowNum, col);
        RefreshFormulaBar();
        RepaintRealizedRows();
    }

    private void FormulaBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter) return;
        _model.CommitInf(FormulaBox.Text);
        RefreshFormulaBar();
        RepaintRealizedRows();
        e.Handled = true;
    }

    private void RefreshFormulaBar()
    {
        AddressText.Text = _model.InfAddress;
        FormulaBox.Text = _model.Formula;
    }

    /// Rebuild only the currently-realized body rows so selection highlight and
    /// recomputed values show. ContainerFromItem returns null for virtualized
    /// (off-screen) rows, so this touches just what's on screen.
    private void RepaintRealizedRows()
    {
        foreach (int rowNum in _rowNumbers)
        {
            if (BodyList.ContainerFromItem(rowNum) is ListViewItem { Content: StackPanel } item)
                item.Content = BuildRow(rowNum);
        }
    }

    // ── Scroll sync ──────────────────────────────────────────────────
    private void BodyHScroll_ViewChanged(object sender, ScrollViewerViewChangedEventArgs e) =>
        HeaderScroll.ChangeView(BodyHScroll.HorizontalOffset, null, null, true);

    /// Walk the visual tree to find a control's first ScrollViewer (a ListView's
    /// internal scroller is part of its template).
    private static ScrollViewer? FindScrollViewer(DependencyObject root)
    {
        if (root is ScrollViewer sv) return sv;
        int n = VisualTreeHelper.GetChildrenCount(root);
        for (int i = 0; i < n; i++)
        {
            var found = FindScrollViewer(VisualTreeHelper.GetChild(root, i));
            if (found is not null) return found;
        }
        return null;
    }
}
