import 'dart:async';
import 'dart:io';

import 'package:mosaic_flutter_runtime_conformance/mosaic_host.dart';

Never _fail(String assertion) =>
    throw StateError('Failed assertion: $assertion');

void _require(bool condition, String assertion) {
  if (!condition) _fail(assertion);
}

Map<String, Object?> _object(Object? value, String assertion) {
  if (value is! Map) _fail('$assertion was not an object');
  final object = Map<String, Object?>.from(value);
  _require(!object.containsKey('error'), '$assertion returned an error');
  return object;
}

Map<String, Object?> _props(Map<String, Object?> update, String assertion) =>
    _object(update['props'], '$assertion props');

int _integer(Object? value, String assertion) {
  if (value is! num) _fail('$assertion was not numeric');
  return value.toInt();
}

String _expectedPlatform() {
  if (Platform.isMacOS || Platform.isIOS) return 'apple';
  if (Platform.isWindows) return 'windows';
  return 'linux';
}

/// UI48 ENV4 (spec §7.8) against the conformance app, which does not react to
/// `environmentChanged`: the runtime answers at the revision showing (3, after
/// the restore above) with no props.
Future<void> _checkEnvironmentReport(MosaicHost host, int count) async {
  // The six values and the 600 / 1024 thresholds.
  final compact = MosaicHost.environmentReport(599, 800, false);
  _require(compact.length == 6, 'report has the six UI48 values');
  _require(compact['sizeClass'] == 'compact', '599 wide is compact');
  _require(
    compact['orientation'] == 'portrait',
    'taller than wide is portrait',
  );
  _require(compact['colorScheme'] == 'light', 'light scheme');
  _require(
    compact['pointer'] == 'fine' && compact['hover'] == 'hover',
    'desktop pointer',
  );
  _require(
    compact['reducedMotion'] == 'no-preference',
    'reduced motion default',
  );
  final regular = MosaicHost.environmentReport(
    600,
    600,
    true,
    reduceMotion: true,
  );
  _require(regular['sizeClass'] == 'regular', '600 wide is regular');
  _require(regular['orientation'] == 'landscape', 'a square is landscape');
  _require(regular['colorScheme'] == 'dark', 'dark scheme');
  _require(regular['reducedMotion'] == 'reduce', 'reduced motion observed');
  _require(
    MosaicHost.environmentReport(1023, 700, false)['sizeClass'] == 'regular',
    '1023 wide is regular',
  );
  _require(
    MosaicHost.environmentReport(1024, 700, false)['sizeClass'] ==
        'expanded',
    '1024 wide is expanded',
  );

  Future<void> requireProps(String assertion) async {
    final showing = _object(await host.props(), assertion);
    _require(_integer(showing['revision'], assertion) == 3, assertion);
    final props = _props(showing, assertion);
    _require(_integer(props['count'], assertion) == count, assertion);
    _require(props['status'] == 'restored', assertion);
  }

  // Ignored: nothing new to show, and the props showing are kept.
  _require(
    host.reportEnvironment(compact) == null,
    'an ignored report has nothing to show',
  );
  await requireProps('an ignored report keeps the props and revision');

  // Invalid: refused as an `error` answer, never thrown; the props stay.
  final invalid = <String, String>{...compact, 'sizeClass': 'enormous'};
  final refusal = host.reportEnvironment(invalid);
  final refusalError = refusal == null ? null : refusal['error'];
  _require(
    refusalError is String &&
        refusalError.startsWith('Mosaic environment report failed'),
    'an invalid report is refused',
  );
  await requireProps('a refusal keeps the props');
  // The identical refused report is held back, not refused again.
  _require(
    host.reportEnvironment(invalid) == null,
    'a refused report is not re-sent',
  );

  // Sent or not. While failEnvironment is on, every report that reaches the
  // app is an app error -- a failure that is not the report's fault -- so an
  // `error` answer proves a report was sent and null that it was held back.
  Future<Map<String, Object?>> failEnvironment(
    bool fail,
    String assertion,
  ) async => _object(
    await host.handleEvent(<String, Object?>{
      'name': 'failEnvironment',
      'payload': <String, Object?>{'fail': fail},
    }),
    assertion,
  );
  String? failure(Map<String, Object?>? answer) {
    final error = answer == null ? null : answer['error'];
    return error is String && error.startsWith('Mosaic environment report failed')
        ? error
        : null;
  }

  await failEnvironment(true, 'the app is told to fail environment changes');
  _require(
    host.reportEnvironment(invalid) == null,
    'the refused report is not re-sent',
  );
  // The refusal did not replace the last report taken: that one is still
  // unchanged, so still not sent.
  _require(
    host.reportEnvironment(compact) == null,
    'an unchanged report is not sent',
  );
  final appFailure = failure(host.reportEnvironment(regular));
  _require(
    appFailure != null && appFailure.contains('Mosaic application error'),
    "a changed report is sent, and the app's failure reported",
  );
  _require(
    failure(host.reportEnvironment(regular)) != null,
    'a report that failed transiently is sent again',
  );
  await failEnvironment(false, 'the app is told to take environment changes again');

  // An ignored report writes no state: nothing the app saves changed.
  final statePath = Platform.environment['MOSAIC_APP_STATE_PATH'];
  if (statePath == null || statePath.trim().isEmpty) {
    stdout.writeln(
      'MOSAIC_APP_STATE_PATH unset: skipped the checks that an ignored report '
      'writes no state',
    );
    return;
  }
  final state = File(statePath);
  _require(state.existsSync(), 'state persisted');
  state.deleteSync();
  _require(
    host.reportEnvironment(regular) == null,
    'once the failure passes, the same report is taken',
  );
  _require(
    !state.existsSync(),
    'an ignored report does not rewrite the state file',
  );
  // With the state path a directory, any write fails and says so. An ignored
  // report attempts none, so it raises no warning to surface; an event still
  // does, on its own answer.
  final blocked = Directory(statePath)..createSync();
  _require(
    host.reportEnvironment(compact) == null,
    'a changed, ignored report has nothing to show',
  );
  _require(
    !_object(await host.props(), 'props after an ignored report')
        .containsKey('persistenceWarning'),
    'an ignored report raises no persistence warning',
  );
  _require(
    (await failEnvironment(true, 'an event while saving fails'))
        .containsKey('persistenceWarning'),
    'an event that cannot persist surfaces the warning at once',
  );
  blocked.deleteSync();
  final cleared = await failEnvironment(false, 'the state path writable again');
  _require(
    state.existsSync() && !cleared.containsKey('persistenceWarning'),
    'the next event persists and clears the warning',
  );
  await failEnvironment(true, 'the app is told to fail environment changes once more');
  _require(
    host.reportEnvironment(compact) == null,
    'the report taken while saving failed is held back',
  );
  await failEnvironment(false, 'the app is left taking environment changes');
}

Future<void> main() async {
  final restoredOnLaunch = Platform.environment['MOSAIC_EXPECT_RESTORED'] == '1';
  final expectWarning =
      Platform.environment['MOSAIC_EXPECT_PERSISTENCE_WARNING'] == '1';
  final initialCount = restoredOnLaunch ? 4 : 0;
  final host = MosaicHost.load();
  if (host == null) _fail('standard Flutter binding did not load the Rust app');

  try {
    final started = _object(await host.props(), 'startup update');
    final startedProps = _props(started, 'startup update');
    _require(
      started.containsKey('persistenceWarning') == expectWarning,
      'startup persistence warning',
    );
    _require(
      _integer(started['revision'], 'startup revision') == 1,
      'startup revision',
    );
    _require(
      _integer(startedProps['count'], 'initial count') == initialCount,
      'initial count',
    );
    _require(
      startedProps['platform'] == _expectedPlatform(),
      'startup platform',
    );
    _require(
      startedProps['status'] == (restoredOnLaunch ? 'restored' : 'started'),
      'startup status',
    );

    var notificationCount = 0;
    host.setPropsChangedHandler(() => notificationCount += 1);

    final dispatched = _object(
      await host.handleEvent(<String, Object?>{
        'name': 'increment',
        'payload': <String, Object?>{'amount': 4},
      }),
      'dispatch update',
    );
    final dispatchedProps = _props(dispatched, 'dispatch update');
    _require(
      _integer(dispatched['revision'], 'dispatch revision') == 2,
      'dispatch revision',
    );
    _require(
      _integer(dispatchedProps['count'], 'dispatched count') == initialCount + 4,
      'dispatched count',
    );
    _require(dispatchedProps['status'] == 'dispatched', 'dispatch status');

    final snapshot = _object(host.snapshot(), 'snapshot');
    _require(
      snapshot['schema'] == 'mosaic-app-conformance/counter',
      'snapshot schema',
    );
    _require(
      _integer(snapshot['version'], 'snapshot version') == 1,
      'snapshot version',
    );
    _require(
      (snapshot['bytes'] as List<Object?>).length == 8,
      'snapshot bytes',
    );

    final restored = _object(host.restore(snapshot), 'restore update');
    final restoredProps = _props(restored, 'restore update');
    _require(
      _integer(restored['revision'], 'restore revision') == 3,
      'restore revision',
    );
    _require(
      _integer(restoredProps['count'], 'restored count') == initialCount + 4,
      'restored count',
    );
    _require(restoredProps['status'] == 'restored', 'restore status');
    _require(notificationCount == 1, 'restore props-change notification');

    await _checkEnvironmentReport(host, initialCount + 4);
  } finally {
    host.dispose();
  }

  stdout.writeln('Mosaic Flutter Rust runtime conformance passed');
}
