import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_pinned_navigation_shell/PinnedNavigationShell.dart';

void main() {
  testWidgets('collapse never retains the side-by-side split when compact', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(
      MaterialApp(
        home: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 500,
            height: 600,
            child: PinnedNavigationShell(dispatch: (_) {}),
          ),
        ),
      ),
    );

    expect(find.byType(Row), findsOneWidget);
    expect(find.byType(Drawer), findsNothing);
    expect(
      find.bySemanticsLabel(RegExp(r'^Workbench(\n|$)')),
      findsOneWidget,
    );
    expect(find.text('Pinned navigation'), findsOneWidget);
    expect(find.text('Workbench detail'), findsOneWidget);
    semantics.dispose();
  });
}
