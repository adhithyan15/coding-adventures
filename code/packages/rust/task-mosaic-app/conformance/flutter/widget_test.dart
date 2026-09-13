import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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

// #14857 — load a REAL font, because `flutter test`'s default measures
// one em per glyph.
//
// That default made every width measured here roughly double, and it is
// not a small distortion: Trestle's task row measured 1076px against
// Compose's 466 and looked like a 2.3x layout defect. With a real font
// the same row measures ~389px and is NARROWER than Compose on every
// text widget:
//
//   widget      test font    real font    Compose
//   Edit             68         48.0         64
//   Delete           97         53.1         77
//   task name       318        128.2        185
//   due <date>      200         96.1        133
//
// Any assertion about width taken without this is measuring the test
// environment. Best-effort: the font path is macOS-specific, so where it
// is absent the tests still run and only width claims lose their meaning.
Future<bool> loadRealFontIfAvailable() async {
  const candidates = [
    '/System/Library/Fonts/Supplemental/Arial.ttf',
    '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
  ];
  for (final path in candidates) {
    final file = File(path);
    if (!file.existsSync()) continue;
    final bytes = file.readAsBytesSync();
    // `Roboto` is the family Material resolves to by default, so loading
    // under that name replaces the test font everywhere rather than only
    // where a family is named explicitly.
    final loader = FontLoader('Roboto')
      ..addFont(Future.value(ByteData.view(bytes.buffer)));
    await loader.load();
    return true;
  }
  return false;
}

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
    final realFont = await loadRealFontIfAvailable();
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
    // The load has to be PROVEN, not assumed: the suite passes either way
    // because nothing else asserts a width, which is precisely how the
    // one-em-per-glyph default went unnoticed. `Delete` measures 53.1 with
    // Arial and 96.6 with the test font, so this threshold separates them
    // with room for a different real font.
    if (realFont) {
      final deleteWidth =
          tester.getSize(find.widgetWithText(ElevatedButton, 'Delete').first).width;
      expect(
        deleteWidth,
        lessThan(75),
        reason: 'the real font did not take effect — widths measured here are '
            'the test environment, not the app (#14857)',
      );
    }
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
