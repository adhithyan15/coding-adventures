import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_nested_row_conditional/NestedRowConditional.dart';

void main() {
  for (final innerFocused in <bool>[true, false]) {
    testWidgets(
      'nested conditional input lays out without unbounded flex ($innerFocused)',
      (tester) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: NestedRowConditional(
                outerVisible: true,
                innerFocused: innerFocused,
                value: 'Draft release',
                dispatch: (_) {},
              ),
            ),
          ),
        );

        await tester.pump();
        expect(tester.takeException(), isNull);
        expect(find.text('Task'), findsOneWidget);
        expect(find.byType(TextField), findsOneWidget);
      },
    );
  }
}
