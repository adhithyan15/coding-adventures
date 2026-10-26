// Journal's pane scrolls on Flutter, so an entry below the fold can be
// reached (JO-5). The Flutter twin of compose/JournalScrollUiTest.kt.
//
// A native window scrolls nothing unless the layout says so. Until the pane
// was wrapped in `HostScroll [ timeline-scroll ]` (a `SingleChildScrollView`
// here), Journal's declared 1100x760 window showed about two timeline rows,
// and every older entry was laid out below the window's bottom edge with no
// way to bring it up.
//
// The test:
//
//   1. writes ENTRIES entries straight through the real Rust runtime (the
//      same events the editor sends), so it measures the pane and not the
//      editor;
//   2. takes the bottom-most timeline row and requires that it starts BELOW
//      the window, so the test really crosses the fold;
//   3. drags the pane's scrollable up, a bounded number of steps, until the
//      row is inside the window;
//   4. taps it, and requires that the editor's title field holds that entry.
//
// Journal has no Flutter desktop lane in CI yet (Mosaic backlog item 13), so
// this runs where the Flutter project is emitted:
//
//   mosaic-compile pkg code/programs/mosaic/journal-app --backend flutter \
//     --output OUT --emit-project --profile native-complete \
//     --runtime-library <libjournal_mosaic_app>
//   cp journal_scroll_test.dart OUT/flutter/test/
//   cd OUT/flutter && MOSAIC_APP_STATE_PATH=<fresh file> flutter test test/journal_scroll_test.dart
//
// When that lane exists it should run this file too.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_journal_app/main.dart';
import 'package:mosaic_journal_app/mosaic_host.dart';

const _window = Size(1100, 760); // journal-app's [app] window
const _entries = 8;
const _step = 200.0;
const _maxSteps = 40;

Future<void> _send(MosaicHost host, String name, [Map<String, Object?> payload = const {}]) async {
  await host.handleEvent({'name': name, 'payload': payload});
}

void main() {
  testWidgets('an entry below the fold can be scrolled to and opened', (tester) async {
    tester.view.physicalSize = _window;
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final host = MosaicHost.loadRequired();
    for (var n = 1; n <= _entries; n++) {
      await _send(host, 'onNewEntry');
      await _send(host, 'onTitleChange', {'value': 'Reach $n'});
      await _send(host, 'onBodyChange', {'value': 'Entry $n of $_entries.'});
      await _send(host, 'onSaveEntry');
    }

    await tester.pumpWidget(MosaicApp(mosaicHostLoader: () => host));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull, reason: 'the pane must lay out cleanly');

    // 2. The bottom-most row, by where it is drawn.
    final rows = [
      for (var n = 1; n <= _entries; n++) find.widgetWithText(ElevatedButton, 'Reach $n'),
    ];
    for (final row in rows) {
      expect(row, findsOneWidget);
    }
    final bottom = rows.reduce((a, b) => tester.getRect(a).top >= tester.getRect(b).top ? a : b);
    final title = (tester.widget<Text>(find.descendant(of: bottom, matching: find.byType(Text))).data)!;
    expect(
      tester.getRect(bottom).top,
      greaterThanOrEqualTo(_window.height),
      reason: '"$title" should start below the window, or this test proves nothing',
    );

    // 3. Scroll the pane until the row is inside the window.
    final pane = find.ancestor(of: bottom, matching: find.byType(Scrollable)).first;
    var steps = 0;
    while (tester.getRect(bottom).bottom > _window.height) {
      expect(steps++, lessThan(_maxSteps), reason: 'the pane does not scroll "$title" into view');
      await tester.drag(pane, const Offset(0, -_step));
      await tester.pumpAndSettle();
    }

    // 4. Open it: the editor holds that entry.
    await tester.tap(bottom);
    await tester.pumpAndSettle();
    expect(
      find.byWidgetPredicate((w) => w is EditableText && w.controller.text == title),
      findsOneWidget,
      reason: 'opening "$title" puts it in the editor',
    );
  });
}
