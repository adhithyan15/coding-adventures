import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_navigation_shell/NavigationShell.dart';

void main() {
  Widget app(double width) => MaterialApp(
    home: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: width,
        height: 600,
        child: NavigationShell(dispatch: (_) {}),
      ),
    ),
  );

  testWidgets('navigation split uses a side-by-side pane when regular', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(app(800));

    expect(find.byType(Row), findsOneWidget);
    expect(find.byType(Scaffold), findsNothing);
    expect(find.byType(Drawer), findsNothing);
    final projectsPane = find.bySemanticsLabel(RegExp(r'^Projects(\n|$)'));
    expect(projectsPane, findsOneWidget);
    expect(tester.getSize(projectsPane).width, 236);
    expect(find.text('Project detail'), findsOneWidget);
    semantics.dispose();
  });

  testWidgets('navigation split uses an accessible drawer when compact', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(app(500));

    expect(tester.getSize(find.byType(LayoutBuilder)).width, 500);
    expect(find.text('Project detail'), findsOneWidget);
    final scaffold = tester.state<ScaffoldState>(find.byType(Scaffold));
    scaffold.openDrawer();
    await tester.pumpAndSettle();
    expect(find.byType(Drawer), findsOneWidget);
    expect(
      find.bySemanticsLabel(RegExp(r'^Projects(\n|$)')),
      findsOneWidget,
    );
    expect(find.text('Project navigation'), findsOneWidget);
    semantics.dispose();
  });
}
