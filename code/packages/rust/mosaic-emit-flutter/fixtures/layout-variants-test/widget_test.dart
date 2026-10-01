// UI48 §7.9 (ENV2/ENV3 on Flutter): the gate §7 asks for -- it CHANGES the
// environment and asserts the root swaps. A test that only rendered at one
// size would pass against a shell frozen on one layout.
//
// The fixture has a default layout (`LayoutProbe`, a Row) and a compact one
// (`LayoutProbeCompact`, a Column), selected by the conventional rule
// `compact` <- `sizeClass == compact`, i.e. a window narrower than 600
// logical pixels. The generated sample shell has no runtime (the standard
// binding finds no library under `flutter test`), so its props are the
// generated samples, kept in the shell's State across the swap.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_layout_probe/LayoutProbe.compact.dart';
import 'package:mosaic_layout_probe/LayoutProbe.dart';
import 'package:mosaic_layout_probe/main.dart';

void main() {
  test('the generated rules are the convention, under wire names', () {
    expect(mosaicLayoutRules, hasLength(1));
    expect(mosaicLayoutRules.single.$1, 'compact');
    expect(mosaicLayoutRules.single.$2, <String, String>{
      'sizeClass': 'compact',
    });
  });

  test('the selector returns the first match, else null', () {
    expect(
      mosaicLayoutVariant(<String, String>{
        'sizeClass': 'compact',
        'pointer': 'fine',
      }),
      'compact',
    );
    expect(mosaicLayoutVariant(<String, String>{'sizeClass': 'regular'}), isNull);
    expect(mosaicLayoutVariant(<String, String>{'sizeClass': 'expanded'}), isNull);
    // A report without the axis never matches a rule that tests it.
    expect(mosaicLayoutVariant(<String, String>{}), isNull);
  });

  testWidgets('resizing across the compact threshold swaps the root', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1.0;
    tester.view.physicalSize = const Size(1200, 800);
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const MosaicApp());
    await tester.pump();
    expect(find.byType(LayoutProbe), findsOneWidget);
    expect(find.byType(LayoutProbeCompact), findsNothing);
    expect(find.text('Sample Title'), findsOneWidget);

    // Narrower than 600: compact. The swap happens on this frame -- nothing
    // but the window changed, so nothing else could have rebuilt the root.
    tester.view.physicalSize = const Size(400, 800);
    await tester.pump();
    expect(find.byType(LayoutProbeCompact), findsOneWidget);
    expect(find.byType(LayoutProbe), findsNothing);
    // The same props reach the other root.
    expect(find.text('Sample Title'), findsOneWidget);
    expect(find.text('Pick'), findsOneWidget);

    // Just under and at the threshold: the bucket, not the pixel, decides.
    tester.view.physicalSize = const Size(599, 800);
    await tester.pump();
    expect(find.byType(LayoutProbeCompact), findsOneWidget);
    tester.view.physicalSize = const Size(600, 800);
    await tester.pump();
    expect(find.byType(LayoutProbe), findsOneWidget);
    expect(find.byType(LayoutProbeCompact), findsNothing);

    // And back again.
    tester.view.physicalSize = const Size(400, 800);
    await tester.pump();
    expect(find.byType(LayoutProbeCompact), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
