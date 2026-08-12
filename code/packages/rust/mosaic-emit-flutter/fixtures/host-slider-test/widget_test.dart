import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_volume/Volume.dart';

void main() {
  testWidgets('native slider preserves range and dispatches change plus commit', (
    tester,
  ) async {
    final events = <VolumeEvent>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Volume(value: 25, disabled: false, dispatch: events.add),
        ),
      ),
    );

    final slider = tester.widget<Slider>(find.byType(Slider));
    expect(slider.value, 25);
    expect(slider.min, 0);
    expect(slider.max, 100);
    expect(slider.divisions, 20);

    slider.onChanged!(40);
    slider.onChangeEnd!(45);
    expect(events.whereType<VolumeEventChange>().single.value, 40);
    expect(events.whereType<VolumeEventCommit>().single.value, 45);
  });

  testWidgets('disabled slider exposes no interactive callbacks', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Volume(value: 25, disabled: true, dispatch: (_) {}),
        ),
      ),
    );

    final slider = tester.widget<Slider>(find.byType(Slider));
    expect(slider.onChanged, isNull);
    expect(slider.onChangeEnd, isNull);
  });
}
