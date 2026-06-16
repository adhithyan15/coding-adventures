// infinite_grid_test.dart — a WIDGET test that pumps the real InfiniteGrid tree
// headlessly and drives it the way a user would: it renders the virtualized
// sheet on the live Rust engine (loaded via dart:ffi), taps a cell to select
// it, edits the formula bar, and asserts every dependent recomputes on screen.
//
// This is the Flutter analog of running the GUI by hand — the widget tree, the
// engine, and the FFI bridge are all exercised end-to-end, just without pixels.
//
// Finder note: a data cell's value and a row-number in the gutter can be the
// same string (e.g. "15" is both A1 and the row-15 label), and a virtualized
// ListView keeps cached-but-offscreen rows in the tree. So we (a) assert on
// values that can't also be a low row number (38, 169, 138…) and (b) target a
// cell for tapping via its GestureDetector ancestor — the chrome cells have
// none, so the match is unambiguous.
//
// Run (after `bash scripts/build.sh` vendors native/libspreadsheet_capi.*):
//   flutter test test/infinite_grid_test.dart

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:visicalc_flutter/infinite_grid.dart';

void main() {
  Future<void> pumpGrid(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: InfiniteGrid())),
    );
    await tester.pumpAndSettle();
  }

  // The unique data cell (GestureDetector) whose value is [text]. The gutter's
  // identically-numbered label is a plain chrome cell with no GestureDetector,
  // so this resolves to exactly the data cell.
  Finder cell(String text) =>
      find.ancestor(of: find.text(text), matching: find.byType(GestureDetector));

  testWidgets('renders the seeded budget with frozen chrome', (tester) async {
    await pumpGrid(tester);

    // Column-letter header (frozen chrome) — single letters, unambiguous.
    expect(find.text('A'), findsOneWidget);
    expect(find.text('B'), findsOneWidget);
    expect(find.text('E'), findsOneWidget);

    // Engine-computed seeded values that can't also be a built row number.
    expect(find.text('38'), findsOneWidget); // E1 = SUM(A1:D1)
    expect(find.text('169'), findsOneWidget); // E5 grand total

    // The formula bar starts on A1 showing its source.
    expect(find.text('A1'), findsOneWidget); // address label
  });

  testWidgets('tap selects a cell and loads its source into the bar',
      (tester) async {
    await pumpGrid(tester);

    // Tap the A1 data cell (value "15"); the bar's field shows the source.
    await tester.tap(cell('15'));
    await tester.pumpAndSettle();

    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.controller!.text, '15');
    expect(find.text('A1'), findsOneWidget);
  });

  testWidgets('editing a cell recomputes every dependent on screen',
      (tester) async {
    await pumpGrid(tester);

    // Select A1 and change 15 -> 115 through the formula bar.
    await tester.tap(cell('15'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '115');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();

    // Dependents recompute live: E1 = 115+3+12+8 = 138, A5 = 115+8+12+4 = 139,
    // E5 = 138+51+45+35 = 269 (none of which is a built row number).
    expect(find.text('138'), findsOneWidget); // E1
    expect(find.text('139'), findsOneWidget); // A5
    expect(find.text('269'), findsOneWidget); // E5
  });
}
