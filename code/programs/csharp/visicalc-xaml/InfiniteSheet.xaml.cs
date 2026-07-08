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
    // Roomier geometry, to match the web reference.
    private const double ColW = 92, RowH = 26, GutterW = 64, HeadH = 28;

    // ── Design tokens ──────────────────────────────────────────────────
    // Mirror code/programs/typescript/visicalc-html/infinite.html's palette so every VisiCalc backend
    // reads as one considered surface (dark modern spreadsheet). Same token set as
    // the Qt / Flutter / Compose ports.
    private static readonly SolidColorBrush Bg = New(0x16, 0x18, 0x1D); // base cell
    private static readonly SolidColorBrush Panel = New(0x1B, 0x1E, 0x24); // zebra band
    private static readonly SolidColorBrush Line = New(0x2C, 0x31, 0x3A); // hairline borders
    private static readonly SolidColorBrush LineStrong = New(0x3A, 0x40, 0x4B); // control borders
    private static readonly SolidColorBrush Head = New(0x20, 0x24, 0x2B); // row/col headers
    private static readonly SolidColorBrush HeadSel = New(0x2B, 0x33, 0x40); // header of selected row/col
    private static readonly SolidColorBrush Ink = New(0xE8, 0xEA, 0xED); // primary text
    private static readonly SolidColorBrush Muted = New(0x9A, 0xA3, 0xB2); // labels, headers
    private static readonly SolidColorBrush Accent = New(0x4A, 0xA3, 0xFF); // selection + focus
    private static readonly SolidColorBrush White = New(0xFF, 0xFF, 0xFF);
    private static readonly SolidColorBrush Sel = New(0x21, 0x34, 0x4A); // selected-cell fill
    private static readonly FontFamily Mono = new("Consolas");

    private readonly InfiniteSheetModel _model = new();
    private List<int> _rowNumbers = new();

    // In-memory "saved file" slot for the Save / Load buttons: Save stows the
    // serialized workbook here, Load restores from it. (A real app would write it
    // to a file; the demo keeps the round trip self-contained.)
    private string _savedSnapshot = string.Empty;

    /// Short find/replace status (match/replace count) echoed in the footer.
    private string _findStatus = string.Empty;

    /// The most recent File → Save / Open result, echoed in the footer.
    private string _fileStatus = string.Empty;

    /// A PRIVATE per-session scratch directory for the File → Save / Open demo,
    /// created lazily with CreateTempSubdirectory (mode 0700 on Unix). Writing
    /// into our own freshly-made directory — rather than a fixed name in the
    /// shared, world-writable system temp root — avoids a local symlink-clobber
    /// on save and keeps the file from being world-readable, while still letting
    /// Open read back what Save wrote (a stable per-session dir).
    private string? _fileDir;
    private string ScratchDir => _fileDir ??=
        System.IO.Directory.CreateTempSubdirectory("visicalc-").FullName;
    private string DemoPath(string ext) =>
        System.IO.Path.Combine(ScratchDir, $"visicalc-demo.{ext}");

    private ScrollViewer? _bodyInnerSv, _gutterInnerSv;

    public InfiniteSheet()
    {
        InitializeComponent();

        _rowNumbers = Enumerable.Range(1, _model.TotalRows).ToList();
        BodyList.ItemsSource = _rowNumbers;
        GutterList.ItemsSource = _rowNumbers;
        BodyList.Width = _model.TotalCols * ColW;

        BuildHeader();
        BuildSheetTabs();
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

    // ── Header (rebuilt on selection so the selected column tints to accent) ──
    private void BuildHeader()
    {
        HeaderPanel.Children.Clear();
        for (int c = 1; c <= _model.TotalCols; c++)
            HeaderPanel.Children.Add(ChromeCell(ColW, HeadH, _model.ColumnLetters(c), _model.SelCol == c));
    }

    // ── Sheet tab bar (multi-sheet workbook) ─────────────────────────
    // One chip per sheet; the active one tints to the accent. Click a tab to
    // switch (bare-A1 ops then address it); the active tab carries inline ✎ rename
    // and ✕ delete affordances. A formula like =Summary!B3 reaches across the
    // tabs, so switching + editing updates every cross-sheet dependent live.
    private void BuildSheetTabs()
    {
        SheetTabsPanel.Children.Clear();
        IReadOnlyList<string> names = _model.SheetNames();
        int active = _model.ActiveSheet();
        for (int i = 0; i < names.Count; i++)
            SheetTabsPanel.Children.Add(SheetTab(i, names[i], i == active));
    }

    /// One sheet tab chip: the name, plus (when active) inline ✎ rename and ✕
    /// delete glyphs. The whole chip is tap-to-switch; the glyphs handle their own
    /// tap (and mark it handled so they don't also switch).
    private Border SheetTab(int index, string name, bool selected)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
        row.Children.Add(new TextBlock
        {
            Text = name,
            Foreground = selected ? White : Ink,
            FontSize = 12,
            FontFamily = Mono,
            FontWeight = selected ? FontWeights.SemiBold : FontWeights.Normal,
            VerticalAlignment = VerticalAlignment.Center,
        });
        if (selected)
        {
            row.Children.Add(Glyph("✎", () => RenameSheetAsync(index)));
            row.Children.Add(Glyph("✕", () => DeleteSheet(index)));
        }

        var chip = new Border
        {
            Background = selected ? Sel : New(0x21, 0x25, 0x2C),
            BorderBrush = selected ? Accent : LineStrong,
            BorderThickness = new Thickness(selected ? 2 : 1),
            CornerRadius = new CornerRadius(6),
            Padding = new Thickness(10, 5, 10, 5),
            Margin = new Thickness(0, 0, 4, 0),
            Child = row,
        };
        chip.Tapped += (_, _) => SwitchSheet(index);
        return chip;
    }

    /// A small inline action glyph (✎ / ✕) inside the active sheet tab. Tapping it
    /// runs `action` and marks the event handled so the parent chip's switch-tap
    /// doesn't also fire.
    private TextBlock Glyph(string text, Action action)
    {
        var tb = new TextBlock
        {
            Text = text,
            Foreground = Muted,
            FontSize = 12,
            FontFamily = Mono,
            Margin = new Thickness(8, 0, 0, 0),
            VerticalAlignment = VerticalAlignment.Center,
        };
        tb.Tapped += (_, e) => { e.Handled = true; action(); };
        return tb;
    }

    private void SwitchSheet(int index) { _model.SelectSheet(index); AfterSheetOp(); }
    private void DeleteSheet(int index) { _model.DeleteSheet(index); AfterSheetOp(); }

    private void AddSheetButton_Click(object sender, RoutedEventArgs e)
    {
        // Name the new sheet "SheetN" where N avoids a clash with existing names.
        var existing = new HashSet<string>(_model.SheetNames());
        int n = existing.Count + 1;
        while (existing.Contains($"Sheet{n}")) n++;
        _model.AddSheet($"Sheet{n}");
        AfterSheetOp();
    }

    /// Rename the sheet at `index` via a small text dialog; the engine rewrites
    /// every referencing formula's qualifier on commit.
    private async void RenameSheetAsync(int index)
    {
        IReadOnlyList<string> names = _model.SheetNames();
        if (index < 0 || index >= names.Count) return;
        var input = new TextBox
        {
            Text = names[index],
            FontFamily = Mono,
            SelectionStart = 0,
        };
        var dialog = new ContentDialog
        {
            Title = "Rename sheet",
            Content = input,
            PrimaryButtonText = "Rename",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = XamlRoot,
        };
        var result = await dialog.ShowAsync();
        if (result != ContentDialogResult.Primary) return;
        string newName = input.Text.Trim();
        if (newName.Length == 0) return;
        _model.RenameSheet(index, newName);
        AfterSheetOp();
    }

    /// After any sheet op (switch / add / rename / delete) the active sheet — and
    /// thus the extent, the cells, and the selection — may have changed wholesale.
    /// Rebuild the row-number source (the new sheet's extent), re-tile the header,
    /// re-sync the formula bar, and rebuild the tab bar.
    private void AfterSheetOp()
    {
        _rowNumbers = Enumerable.Range(1, _model.TotalRows).ToList();
        BodyList.ItemsSource = _rowNumbers;
        GutterList.ItemsSource = _rowNumbers;
        BodyList.Width = _model.TotalCols * ColW;
        BuildHeader();
        BuildSheetTabs();
        RefreshFormulaBar();
        RepaintRealizedRows();
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
        // The selected row's gutter label tints to the accent.
        args.ItemContainer.Content = ChromeCell(GutterW, RowH, rowNum.ToString(), _model.SelRow == rowNum);
        args.Handled = true;
    }

    /// One body row: a horizontal strip of tappable cells. A single engine read
    /// (RowCells) fills the whole row. Selected → accent fill + 2px accent ring;
    /// otherwise a zebra band (even rows take the panel tint).
    private StackPanel BuildRow(int rowNum)
    {
        var sp = new StackPanel { Orientation = Orientation.Horizontal };
        IReadOnlyList<string> cells = _model.RowCells(rowNum);
        SolidColorBrush band = rowNum % 2 == 0 ? Panel : Bg;
        for (int c = 1; c <= _model.TotalCols; c++)
        {
            string text = (c - 1) < cells.Count ? cells[c - 1] : string.Empty;
            bool selected = _model.SelRow == rowNum && _model.SelCol == c;
            int col = c; // capture for the handler
            var cell = new Border
            {
                Width = ColW,
                Height = RowH,
                Background = selected ? Sel : band,
                BorderBrush = selected ? Accent : Line,
                BorderThickness = new Thickness(selected ? 2 : 0.5),
                Child = new TextBlock
                {
                    Text = text,
                    Foreground = selected ? White : Ink,
                    FontSize = 12,
                    FontFamily = Mono,
                    FontWeight = selected ? FontWeights.SemiBold : FontWeights.Normal,
                    HorizontalAlignment = HorizontalAlignment.Right,
                    VerticalAlignment = VerticalAlignment.Center,
                    Margin = new Thickness(0, 0, 6, 0),
                    TextTrimming = TextTrimming.CharacterEllipsis,
                },
            };
            cell.Tapped += (_, _) => Select(rowNum, col);
            sp.Children.Add(cell);
        }
        return sp;
    }

    /// A frozen header/gutter cell. When [selected] (its row/column holds the
    /// cursor) it tints to the accent.
    private Border ChromeCell(double w, double h, string text, bool selected = false) => new()
    {
        Width = w,
        Height = h,
        Background = selected ? HeadSel : Head,
        BorderBrush = Line,
        BorderThickness = new Thickness(0.5),
        Child = new TextBlock
        {
            Text = text,
            Foreground = selected ? Accent : Muted,
            FontSize = 11,
            FontFamily = Mono,
            FontWeight = FontWeights.SemiBold,
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

    // The formula field's accent focus ring: thicken + tint the wrapping border
    // while the box has focus (the WinUI analog of the web demo's :focus ring).
    private void FormulaBox_GotFocus(object sender, RoutedEventArgs e)
    {
        FormulaFieldBorder.BorderBrush = Accent;
        FormulaFieldBorder.BorderThickness = new Thickness(2);
    }

    private void FormulaBox_LostFocus(object sender, RoutedEventArgs e)
    {
        FormulaFieldBorder.BorderBrush = LineStrong;
        FormulaFieldBorder.BorderThickness = new Thickness(1);
    }

    /// Drag-fill: replicate the selected cell into the 10 rows below it. The
    /// engine shifts each copy's relative refs, pins absolute ($) refs, carries
    /// the format, and recomputes every dependent; then repaint the visible rows.
    private void FillButton_Click(object sender, RoutedEventArgs e)
    {
        _model.FillDown(10);
        RepaintRealizedRows();
    }

    /// Structural edits: insert / delete the selected cell's row or column. The
    /// engine shifts every formula reference across the band and recomputes; a
    /// reference whose whole band is deleted becomes #REF!. Re-sync the formula
    /// bar (the selected cell's content may have moved) and repaint the rows.
    private void InsRowButton_Click(object sender, RoutedEventArgs e)
    {
        _model.InsertRow();
        RefreshFormulaBar();
        RepaintRealizedRows();
    }

    private void DelRowButton_Click(object sender, RoutedEventArgs e)
    {
        _model.DeleteRow();
        RefreshFormulaBar();
        RepaintRealizedRows();
    }

    private void InsColButton_Click(object sender, RoutedEventArgs e)
    {
        _model.InsertCol();
        RefreshFormulaBar();
        RepaintRealizedRows();
    }

    private void DelColButton_Click(object sender, RoutedEventArgs e)
    {
        _model.DeleteCol();
        RefreshFormulaBar();
        RepaintRealizedRows();
    }

    /// Number formatting: apply an Excel-style code to the selected cell. The
    /// format is display-only — the stored value is unchanged; repaint the
    /// realized rows so the formatted string shows.
    private void FmtDecimalButton_Click(object sender, RoutedEventArgs e) { _model.ApplyFormat("#,##0.00"); RepaintRealizedRows(); }
    private void FmtPercentButton_Click(object sender, RoutedEventArgs e) { _model.ApplyFormat("0.0%"); RepaintRealizedRows(); }
    private void FmtCurrencyButton_Click(object sender, RoutedEventArgs e) { _model.ApplyFormat("$#,##0.00"); RepaintRealizedRows(); }
    private void FmtGeneralButton_Click(object sender, RoutedEventArgs e) { _model.ApplyFormat(""); RepaintRealizedRows(); }

    /// Range sort: reorder the budget block A1:E4 by the selected column,
    /// ascending/descending. Each row moves as a record — the E-column SUM
    /// formulas travel with their row and the engine shifts their refs, so every
    /// total stays correct. Repaint the realized rows to show the new order.
    private void SortAscButton_Click(object sender, RoutedEventArgs e) { _model.SortBlock(true); RepaintRealizedRows(); }
    private void SortDescButton_Click(object sender, RoutedEventArgs e) { _model.SortBlock(false); RepaintRealizedRows(); }

    /// Clipboard: copy/cut the selected cell, then paste it at the selection. The
    /// engine shifts the pasted formula's relative refs by the destination's
    /// offset, pins absolute ($) refs, carries the format; a cut clears the source
    /// on paste. Paste repaints the realized rows when it actually applied.
    private void CopyButton_Click(object sender, RoutedEventArgs e) => _model.CopyCell();
    private void CutButton_Click(object sender, RoutedEventArgs e) => _model.CutCell();
    private void PasteButton_Click(object sender, RoutedEventArgs e)
    {
        if (_model.PasteCell()) RepaintRealizedRows();
    }

    /// Save / load: serialize the whole workbook (formulas + formats) to a JSON
    /// document held in memory, and restore it. Computed values recompute on load,
    /// so a loaded formula stays live; the formula bar and rows re-read after Load.
    private void SaveButton_Click(object sender, RoutedEventArgs e) =>
        _savedSnapshot = _model.SaveBook();

    private void LoadButton_Click(object sender, RoutedEventArgs e)
    {
        if (_savedSnapshot.Length == 0) return;
        if (_model.LoadBook(_savedSnapshot))
        {
            RefreshFormulaBar();
            RepaintRealizedRows();
        }
    }

    // ── File → Save / Open a REAL spreadsheet file on disk ────────────
    // Unlike the in-memory Save / Load above, these write and read an actual file
    // — an .xlsx (a ZIP, keeping live formulas) or a .csv (text, values only) —
    // over the engine's byte codecs (InfiniteSheetModel.ExportBytes/ImportBytes
    // → sc_save_*/sc_load_*), proving file bytes cross P/Invoke intact. The demo
    // uses a fixed name in a private per-session temp dir (no OS picker); a
    // production host would swap in a FileSavePicker / FileOpenPicker and hand the
    // same bytes across.
    private void SaveXlsxButton_Click(object sender, RoutedEventArgs e) => ExportFile("xlsx");
    private void OpenXlsxButton_Click(object sender, RoutedEventArgs e) => OpenFile("xlsx");
    private void SaveCsvButton_Click(object sender, RoutedEventArgs e) => ExportFile("csv");
    private void OpenCsvButton_Click(object sender, RoutedEventArgs e) => OpenFile("csv");

    private void ExportFile(string format)
    {
        byte[] bytes = _model.ExportBytes(format);
        string path = DemoPath(format);
        System.IO.File.WriteAllBytes(path, bytes);
        _fileStatus = $"saved {bytes.Length / 1024.0:0.0} KB → {System.IO.Path.GetFileName(path)}";
        UpdateStatus();
    }

    private void OpenFile(string format)
    {
        string path = DemoPath(format);
        if (!System.IO.File.Exists(path))
        {
            _fileStatus = $"no {System.IO.Path.GetFileName(path)} yet — Save first";
            UpdateStatus();
            return;
        }
        bool ok = _model.ImportBytes(format, System.IO.File.ReadAllBytes(path));
        _fileStatus = ok ? $"opened {System.IO.Path.GetFileName(path)}" : $"not a valid {format} file";
        if (ok)
        {
            RefreshFormulaBar();
            RepaintRealizedRows();
        }
        UpdateStatus();
    }

    /// Undo / redo: walk the engine's snapshot history. On success the formula
    /// bar re-reads and the realized rows repaint (any cell could have changed);
    /// the buttons enable/disable off CanUndo/CanRedo via RefreshHistoryButtons.
    private void UndoButton_Click(object sender, RoutedEventArgs e)
    {
        if (_model.UndoEdit())
        {
            RefreshFormulaBar();
            RepaintRealizedRows();
        }
    }

    private void RedoButton_Click(object sender, RoutedEventArgs e)
    {
        if (_model.RedoEdit())
        {
            RefreshFormulaBar();
            RepaintRealizedRows();
        }
    }

    /// Keep the Undo/Redo buttons' enabled state in step with the engine's
    /// history ends. Called from RefreshFormulaBar (so every edit/select/load
    /// refreshes it) and RepaintRealizedRows (so fill/paste do too).
    private void RefreshHistoryButtons()
    {
        UndoButton.IsEnabled = _model.CanUndo;
        RedoButton.IsEnabled = _model.CanRedo;
    }

    private void RefreshFormulaBar()
    {
        AddressText.Text = _model.InfAddress;
        FormulaBox.Text = _model.Formula;
        RefreshHistoryButtons();
        UpdateStatus();
    }

    /// Find: locate every cell whose source contains the query (case-insensitive)
    /// and jump the selection to the first hit; the footer shows the match count.
    /// The .NET sibling of the web demo's Find button and the Qt/Flutter/Compose
    /// ports — it searches formula text, so "=SUM" or a literal like "15" both hit.
    private void FindButton_Click(object sender, RoutedEventArgs e) => RunFind();
    private void FindBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter) return;
        RunFind();
        e.Handled = true;
    }

    private void RunFind()
    {
        string query = FindBox.Text;
        var hits = _model.FindAll(query);
        if (hits.Count > 0)
        {
            _model.SelectA1(hits[0]);
            RefreshFormulaBar();
            RepaintRealizedRows();
        }
        _findStatus = query.Length == 0 ? string.Empty
            : hits.Count == 0 ? "no match"
            : $"{hits.Count} match{(hits.Count == 1 ? "" : "es")}";
        UpdateStatus();
    }

    /// Replace: rewrite the query → replacement in every cell's source and
    /// recompute; the footer shows how many cells changed. The engine re-parses
    /// each rewrite (so a formula stays live, a literal stays typed).
    private void ReplaceButton_Click(object sender, RoutedEventArgs e) => RunReplace();
    private void ReplaceBox_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter) return;
        RunReplace();
        e.Handled = true;
    }

    private void RunReplace()
    {
        int n = _model.ReplaceAll(FindBox.Text, ReplaceBox.Text);
        _findStatus = $"{n} replaced";
        RefreshFormulaBar();
        RepaintRealizedRows();
    }

    /// The hairline footer: the live virtual-grid size + the per-edit revision
    /// clock (mirrors the web/Qt/Flutter/Compose status lines), with the
    /// find/replace count appended when present.
    private void UpdateStatus() =>
        StatusText.Text =
            $"Virtual grid: {_model.TotalRows} rows × {_model.TotalCols} cols  ·  revision {_model.Revision}"
            + (_findStatus.Length > 0 ? $"  ·  {_findStatus}" : string.Empty)
            + (_fileStatus.Length > 0 ? $"  ·  {_fileStatus}" : string.Empty);

    /// Rebuild only the currently-realized rows (body + gutter) and the header so
    /// the selection highlight, accent-tinted row/column headers, and recomputed
    /// values show. ContainerFromItem returns null for virtualized (off-screen)
    /// rows, so this touches just what's on screen.
    private void RepaintRealizedRows()
    {
        foreach (int rowNum in _rowNumbers)
        {
            if (BodyList.ContainerFromItem(rowNum) is ListViewItem { Content: StackPanel } bodyItem)
                bodyItem.Content = BuildRow(rowNum);
            if (GutterList.ContainerFromItem(rowNum) is ListViewItem { Content: Border } gutterItem)
                gutterItem.Content = ChromeCell(GutterW, RowH, rowNum.ToString(), _model.SelRow == rowNum);
        }
        BuildHeader(); // re-tint the selected column's header
        RefreshHistoryButtons(); // fill/paste mutate the doc → re-gate Undo/Redo
        UpdateStatus();
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
