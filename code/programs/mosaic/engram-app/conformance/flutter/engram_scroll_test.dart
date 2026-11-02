// Engram's screens scroll on Flutter, so a control below the fold can be
// reached (EN-6). The Flutter counterpart of compose/EngramScrollUiTest.kt.
//
// A native window scrolls nothing unless the layout says so. Until the screens
// were wrapped in `HostScroll [ content-scroll ]` (an `Expanded`
// `SingleChildScrollView` here), Flutter laid each screen out in a `Column`
// that simply ran past the window: in an 800x600 window the Decks screen
// overflowed by 344px, so the collection's action buttons (Import, Export,
// Add note, Delete note type, ...) were drawn off screen with no way to bring
// them up, and the framework reported the overflow as a layout error.
//
// The test:
//
//   1. starts on Decks in an 800x600 window (the size Compose Desktop opens
//      Engram at, since Engram declares none), and requires that it lays out
//      without an exception;
//   2. requires that the last control on the screen, "Delete note type",
//      starts BELOW the window, so the test really crosses the fold;
//   3. drags the screen's scrollable up, a bounded number of steps, until the
//      button is inside the window;
//   4. requires that the button is now hit-testable: a tap at its centre
//      reaches it, and nothing covers it;
//   5. requires that the screen switcher is still inside the window. On the
//      desktop layout the header sits outside the scroll on purpose.
//
// Why Decks and not Options, which the Compose test uses: on Flutter, Options
// throws for reasons unrelated to scrolling. Its sibling-bury checkbox row
// overflows to the right (Flutter does not lower `flex-wrap`), and before
// #18467 its text fields had an unbounded width. Decks is free of both, so
// this test fails only if the screen does not scroll.
//
// Run it where the Flutter project is emitted (CI's Flutter lane does this):
//
//   mosaic-compile pkg code/programs/mosaic/engram-app --backend flutter \
//     --output OUT --emit-project --profile native-complete \
//     --runtime-library <libengram_mosaic_app>
//   cp engram_scroll_test.dart OUT/flutter/test/
//   cd OUT/flutter && MOSAIC_APP_STATE_PATH=<fresh file> flutter test test/engram_scroll_test.dart
//
// On macOS, in a long output path, the runtime library must be linked with
// `-headerpad_max_install_names` (see lessons.d).

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_engram_app/EngramApp.dart';
import 'package:mosaic_engram_app/main.dart';
import 'package:mosaic_engram_app/mosaic_host.dart';

const _window = Size(800, 600);
const _step = 200.0;
const _maxSteps = 40;

/// The runtime snapshots the collection, so the test must never run against a
/// real one: without MOSAIC_APP_STATE_PATH the host falls back to the app's
/// per-user data file. Require a fresh, isolated state file, which also makes
/// the test start on Decks with an empty collection.
void _requireIsolatedState() {
  final statePath = Platform.environment['MOSAIC_APP_STATE_PATH'];
  if (statePath == null || statePath.trim().isEmpty) {
    fail('set MOSAIC_APP_STATE_PATH to a fresh temporary file; the runtime writes its snapshot there');
  }
  final file = File(statePath);
  if (file.existsSync() && file.lengthSync() > 0) {
    fail('MOSAIC_APP_STATE_PATH ($statePath) already holds state; point it at a fresh file');
  }
}

void main() {
  testWidgets('a control below the fold can be scrolled to', (tester) async {
    _requireIsolatedState();
    tester.view.physicalSize = _window;
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final host = MosaicHost.loadRequired();
    await tester.pumpWidget(MosaicApp(mosaicHostLoader: () => host));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull, reason: 'Decks must lay out cleanly');
    // The desktop layout, whose header stays put. (The touch layout scrolls
    // its header too, and is chosen only for a coarse pointer.)
    expect(find.byType(EngramApp), findsOneWidget);

    // 2. The screen's last control exists, but starts below the window.
    final last = find.widgetWithText(ElevatedButton, 'Delete note type');
    expect(last, findsOneWidget);
    expect(
      tester.getRect(last).top,
      greaterThanOrEqualTo(_window.height),
      reason: '"Delete note type" should start below the window, or this test proves nothing',
    );

    // 3. Scroll the screen until the button is inside the window.
    final screen = find.ancestor(of: last, matching: find.byType(Scrollable)).first;
    var steps = 0;
    while (tester.getRect(last).bottom > _window.height) {
      expect(steps++, lessThan(_maxSteps), reason: 'the screen does not scroll "Delete note type" into view');
      await tester.drag(screen, const Offset(0, -_step));
      await tester.pumpAndSettle();
    }

    // 4. It is really there: a tap at its centre would reach it.
    expect(last.hitTestable(), findsOneWidget);

    // 5. The switcher stayed put. (Its "Options" segment: "Decks" is also the
    //    deck list's label on this screen.)
    final optionsTab = find.text('Options');
    expect(optionsTab, findsOneWidget);
    expect(tester.getRect(optionsTab).top, greaterThanOrEqualTo(0));
    expect(optionsTab.hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
