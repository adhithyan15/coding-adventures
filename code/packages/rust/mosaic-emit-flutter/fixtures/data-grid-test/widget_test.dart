import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_data_grid/DataGrid.dart';

void collectSemantics(SemanticsNode node, List<SemanticsNode> nodes) {
  nodes.add(node);
  node.visitChildren((child) {
    collectSemantics(child, nodes);
    return true;
  });
}

void main() {
  testWidgets('dynamic Mosaic grid exposes native table semantics', (tester) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(
      MaterialApp(
        home: DataGrid(
          headers: const <String>['Name', 'Status'],
          rows: const <List<String>>[
            <String>['Ada', 'Ready'],
          ],
          dispatch: (_) {},
        ),
      ),
    );

    expect(find.byType(DataTable), findsOneWidget);
    final nodes = <SemanticsNode>[];
    collectSemantics(tester.getSemantics(find.byType(Table)), nodes);
    final roles = nodes.map((node) => node.getSemanticsData().role).toList();
    expect(roles.where((role) => role == SemanticsRole.table), hasLength(1));
    expect(roles.where((role) => role == SemanticsRole.row), hasLength(2));
    expect(
      roles.where((role) => role == SemanticsRole.columnHeader),
      hasLength(2),
    );
    expect(roles.where((role) => role == SemanticsRole.cell), hasLength(2));
    semantics.dispose();
  });

  testWidgets('an empty header set stays render-safe', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: DataGrid(
          headers: const <String>[],
          rows: const <List<String>>[],
          dispatch: (_) {},
        ),
      ),
    );

    expect(find.byType(DataTable), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
