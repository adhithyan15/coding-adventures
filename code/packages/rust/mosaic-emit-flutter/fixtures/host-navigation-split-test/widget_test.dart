import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
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

  // #15834 — this test used to open the drawer by reaching into the widget's
  // own state:
  //
  //     tester.state<ScaffoldState>(find.byType(Scaffold)).openDrawer();
  //
  // which proved the drawer's CONTENTS were right and nothing about whether
  // anyone could get to them. No user can call `ScaffoldState.openDrawer()`.
  // Underneath it the compact branch was `Scaffold(drawer:, body:)` with no
  // `AppBar`, and Flutter draws the hamburger only from an `AppBar` — so the
  // pane opened on an edge-drag gesture and nothing else. Measured on this
  // very fixture: 0 buttons and ZERO actionable semantics nodes at 500px.
  //
  // It now opens the pane the way a person does, so the assertion is about
  // reachability rather than about `Drawer`'s contents.
  testWidgets('navigation split pane is reachable when compact', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(app(500));

    expect(tester.getSize(find.byType(LayoutBuilder)).width, 500);
    expect(find.text('Project detail'), findsOneWidget);
    // Closed to begin with.
    expect(find.text('Project navigation'), findsNothing);

    // There is a control, it is named after the pane, and a screen reader
    // can both find and activate it. Flutter's own drawer button sets label
    // and tooltip alike; so does this one.
    final opener = find.byTooltip('Projects');
    expect(
      opener,
      findsOneWidget,
      reason: 'the pane needs a control a user can actually press',
    );
    expect(find.bySemanticsLabel('Projects'), findsWidgets);

    final node = tester.getSemantics(opener);
    expect(
      node.getSemanticsData().hasAction(SemanticsAction.tap),
      isTrue,
      reason: 'a screen reader must be able to activate it',
    );
    expect(
      node.getSemanticsData().hasAction(SemanticsAction.focus),
      isTrue,
      reason: 'a keyboard user must be able to reach it',
    );

    // Press it. No privileged access to ScaffoldState.
    await tester.tap(opener);
    await tester.pumpAndSettle();

    expect(find.byType(Drawer), findsOneWidget);
    // Two carry the pane's name once it is open — the opener and the pane
    // itself — so this asks that the name is on the open pane specifically,
    // rather than counting.
    expect(
      find.descendant(
        of: find.byType(Drawer),
        matching: find.bySemanticsLabel(RegExp(r'^Projects(\n|$)')),
      ),
      findsOneWidget,
    );
    expect(find.text('Project navigation'), findsOneWidget);
    semantics.dispose();
  });
}
