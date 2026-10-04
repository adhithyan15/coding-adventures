// The Flutter platform library's "Replace it?" question (UI87 §7.7), driven
// through a real Material dialog by the widget tester.
//
// GTK's file chooser, as `file_selector_linux` opens it, saves over an
// existing file without asking. The library asks instead, in a dialog on the
// app's root navigator (`mosaicAskToReplace`). This test proves that dialog
// appears, names the file, and answers as the person chose -- and that with no
// app on screen the question throws, so the save fails rather than replacing
// a file unasked.
//
// Copied into a generated project's `test/` and run with
//   flutter test test/mosaic_replace_dialog_test.dart
// (CI does this in the Flutter TaskApp lane). It needs the Flutter engine,
// which is why it is not part of the plain-Dart harness in
// `../flutter-platform-effects/`.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

// Relative, because the project's package name differs per app.
// ignore: avoid_relative_lib_imports
import '../lib/mosaic_platform_effects.dart';

void main() {
  Future<Future<bool>> ask(WidgetTester tester, String name) async {
    await tester.pumpWidget(const MaterialApp(home: Text('app')));
    final answer = mosaicAskToReplace(name);
    await tester.pumpAndSettle();
    expect(find.text('Replace "$name"?'), findsOneWidget);
    return answer;
  }

  testWidgets('Replace answers true', (tester) async {
    final answer = await ask(tester, 'journal.json');
    await tester.tap(find.text('Replace'));
    await tester.pumpAndSettle();
    expect(await answer, isTrue);
    expect(find.text('Replace "journal.json"?'), findsNothing);
  });

  testWidgets('Cancel keeps the file', (tester) async {
    final answer = await ask(tester, 'journal.json');
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(await answer, isFalse);
  });

  testWidgets('dismissing the question keeps the file', (tester) async {
    final answer = await ask(tester, 'notes.md');
    await tester.tapAt(const Offset(4, 4)); // the barrier, outside the dialog
    await tester.pumpAndSettle();
    expect(await answer, isFalse);
  });

  testWidgets('the root navigator is found without a key', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: Text('app')));
    expect(mosaicRootNavigator(), isNotNull);
  });

  testWidgets('with no navigator the question throws', (tester) async {
    await tester.pumpWidget(
      const Directionality(textDirection: TextDirection.ltr, child: Text('no app')),
    );
    expect(mosaicRootNavigator(), isNull);
    await expectLater(mosaicAskToReplace('journal.json'), throwsStateError);
  });
}
