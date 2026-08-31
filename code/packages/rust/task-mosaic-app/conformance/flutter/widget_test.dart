import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_task_app/main.dart';
import 'package:mosaic_task_app/mosaic_host.dart';

const _taskName = 'Native acceptance task';
const _editedTaskName = 'Edited native task';
const _editedDue = '2026-01-12';
const _persistedTaskName = 'Persisted native task';
const _due = '2026-01-09';
const _schedule = '2026-01-05 → 2026-01-05';

Finder _input(String hint) => find.byWidgetPredicate(
  (widget) => widget is TextField && widget.decoration?.hintText == hint,
  description: 'TextField with hint "$hint"',
);

Future<void> _settle(WidgetTester tester) async {
  await tester.pump();
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('generated controls drive the Rust scheduling lifecycle', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(2400, 1600);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final restoredOnLaunch =
        Platform.environment['MOSAIC_EXPECT_RESTORED'] == '1';
    final host = MosaicHost.loadRequired();
    await tester.pumpWidget(MosaicApp(mosaicHost: host));
    await _settle(tester);

    if (restoredOnLaunch) {
      expect(find.text(_persistedTaskName), findsOneWidget);
      expect(find.text('due $_due'), findsOneWidget);
      expect(find.text(_schedule), findsOneWidget);
      await tester.tap(find.widgetWithText(ElevatedButton, 'Delete'));
      await _settle(tester);
      expect(find.text(_persistedTaskName), findsNothing);
      return;
    }

    final before = host.snapshot();
    expect(
      () => host.handleEvent(<String, Object?>{
        'name': 'onNewTaskNameChange',
        'payload': <String, Object?>{'value': 7},
      }),
      throwsA(anything),
    );
    expect(host.snapshot().toString(), before.toString());
    expect(find.text(_taskName), findsNothing);

    await tester.enterText(_input('What needs doing?'), _taskName);
    await _settle(tester);
    await tester.enterText(_input('Due (optional)'), _due);
    await _settle(tester);
    await tester.tap(find.widgetWithText(ElevatedButton, 'Add task'));
    await _settle(tester);

    expect(find.text(_taskName), findsOneWidget);
    expect(find.text('due $_due'), findsOneWidget);
    expect(find.text(_schedule), findsNothing);
    await tester.tap(find.widgetWithText(ElevatedButton, 'Board').first);
    await _settle(tester);
    expect(find.text(_schedule), findsOneWidget);

    await tester.tap(find.widgetWithText(ElevatedButton, 'Edit'));
    await _settle(tester);
    await tester.enterText(_input('Task name'), _editedTaskName);
    await tester.enterText(_input('Due (optional)').last, _editedDue);
    await tester.tap(find.widgetWithText(ElevatedButton, 'Save'));
    await _settle(tester);
    expect(find.text(_editedTaskName), findsOneWidget);
    expect(find.text('due $_editedDue'), findsOneWidget);

    await tester.tap(find.widgetWithText(ElevatedButton, '○'));
    await _settle(tester);
    expect(find.widgetWithText(ElevatedButton, '✓'), findsOneWidget);
    expect(find.text('100%'), findsWidgets);
    await tester.tap(find.widgetWithText(ElevatedButton, '✓'));
    await _settle(tester);
    expect(find.widgetWithText(ElevatedButton, '○'), findsOneWidget);

    await tester.tap(find.widgetWithText(ElevatedButton, 'Delete'));
    await _settle(tester);
    expect(find.text(_editedTaskName), findsNothing);

    await tester.enterText(_input('What needs doing?'), _persistedTaskName);
    await _settle(tester);
    await tester.enterText(_input('Due (optional)'), _due);
    await _settle(tester);
    await tester.tap(find.widgetWithText(ElevatedButton, 'Add task'));
    await _settle(tester);
    expect(find.text(_persistedTaskName), findsOneWidget);
    expect(find.text(_schedule), findsOneWidget);
  });
}
