import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_row_child_input/RowChildInput.dart';

// A TextField one container below a Row lays out. Before the emitter wrapped
// such a Row child in `Expanded`, every field here asserted "An
// InputDecorator, which is typically created by a TextField, cannot have an
// unbounded width" on every frame.
void main() {
  testWidgets('text fields below a Row child get a bounded width', (
    tester,
  ) async {
    final events = <RowChildInputEvent>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RowChildInput(
            fieldLabel: 'Front',
            fieldValue: '',
            fieldNames: const ['Front', 'Back'],
            dispatch: events.add,
          ),
        ),
      ),
    );

    await tester.pump();
    expect(tester.takeException(), isNull);
    expect(find.byType(TextField), findsNWidgets(5));

    // Every field received a finite width: the two in the Box share the
    // row with the list column, so each is narrower than the window but wider
    // than nothing.
    final window = tester.getSize(find.byType(Scaffold)).width;
    for (final field in tester.widgetList<TextField>(find.byType(TextField))) {
      final width = tester.getSize(find.byWidget(field)).width;
      expect(width, greaterThan(0));
      expect(width, lessThanOrEqualTo(window));
    }

    // And the field still edits.
    await tester.enterText(find.byType(TextField).first, 'hello');
    expect(events, hasLength(1));
  });
}
