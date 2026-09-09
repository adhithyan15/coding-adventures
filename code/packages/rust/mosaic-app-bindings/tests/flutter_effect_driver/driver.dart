// Drives the emitted Flutter host against the conformance runtime.
//
// This crate's other tests assert on the TEXT of the emitted host. That cannot
// tell you it compiles, let alone that it behaves -- the Compose port of this
// same change shipped three missing imports past every text assertion. The
// defect this exists for is behavioural: before the host completed effects, an
// `await` was dropped and the app waited forever with nothing reporting it.
//
// One scenario per process, chosen by MOSAIC_PROBE_CASE, because the host reads
// MOSAIC_APP_STATE_PATH once at load and a shared state file would let one
// scenario restore another's count.
import 'dart:async';
import 'dart:io';

import 'package:mosaic_flutter_effect_driver/mosaic_host.dart';

var _failures = 0;

void check(bool ok, String what) {
  stdout.writeln(what.padRight(60) + (ok ? 'ok' : 'FAIL'));
  if (!ok) _failures += 1;
}

Map<String, Object?> propsOf(Map<String, Object?>? update) {
  final props = update?['props'];
  return props is Map ? Map<String, Object?>.from(props) : <String, Object?>{};
}

// A missing key must not read as zero: comparing against 0 would pass when the
// props are empty, which is how the Qt version of this file first passed.
int awaited(Map<String, Object?> props, String context) {
  final value = props['awaitedEffects'];
  if (value is! num) {
    stdout.writeln('VACUOUS: $context carries no awaitedEffects key');
    _failures += 1;
    return -1;
  }
  return value.toInt();
}

// Likewise: a missing count must not compare equal to the value under test.
int count(Map<String, Object?> props, String context) {
  final value = props['count'];
  if (value is! num) {
    stdout.writeln('VACUOUS: $context carries no count key');
    _failures += 1;
    return -1 << 62;
  }
  return value.toInt();
}

String status(Map<String, Object?> props) {
  final value = props['status'];
  return value is String ? value : '';
}

/// Whether a snapshot was actually produced, and why not when it was not.
///
/// A refusal can arrive either as a thrown `MosaicRuntimeException` or as an
/// `error` key, and which one a pending effect takes is the runtime's business,
/// not this driver's. Both count as refused; only a real snapshot counts.
(bool, String) snapshotMade(MosaicHost host) {
  try {
    final snap = host.snapshot();
    if (snap == null) return (false, 'the app produced no snapshot');
    final error = snap['error'];
    if (error != null) return (false, '$error');
    return (true, '');
  } on Object catch (error) {
    return (false, '$error');
  }
}

String suffix(bool ok, String why) => ok ? '' : ' [$why]';

Future<Map<String, Object?>> request(
  MosaicHost host, {
  bool batch = false,
}) async => propsOf(
  await host.handleEvent(<String, Object?>{
    'event': batch ? 'requestEffectBatch' : 'requestEffect',
    'notify': false,
  }),
);

MosaicHost loadHost() {
  final host = MosaicHost.load();
  if (host == null) {
    throw StateError('the emitted Flutter host did not load the runtime');
  }
  return host;
}

bool isAwait(String delivery) => delivery.toLowerCase() == 'await';

/// A fresh app awaits nothing, and an await nobody answers is failed.
Future<void> caseUnhandled() async {
  final host = loadHost();
  check(
    awaited(propsOf(await host.props()), 'startup') == 0,
    'a fresh app awaits nothing',
  );

  // No handler connected -- what every native host did before completion
  // existed. The effect must come back failed, not sit pending forever.
  final unhandled = await request(host);
  check(
    awaited(unhandled, 'unhandled') == 0,
    'an unanswered await is failed, not dropped',
  );
  check(
    status(unhandled).contains('no host handler'),
    'the app is told why, rather than just waiting',
  );
  check(
    count(unhandled, 'unhandled') == 0,
    'a failed completion does not advance the app',
  );
}

/// A handler that answers properly settles the effect and moves the app.
Future<void> caseAnswered() async {
  final host = loadHost();
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery)) {
      host.completeEffect(id, <String, Object?>{
        'ok': <String, Object?>{'amount': 5},
      });
    }
  };
  final answered = await request(host);
  check(awaited(answered, 'handled') == 0, 'an answered await is settled');
  check(count(answered, 'handled') == 5, "the handler's value reached the app");
}

/// A batch where the handler answers BOTH, each chaining.
///
/// One slot holding "the last update" drops the first answer's minted effect:
/// it exists in no collection the host kept, so it is never emitted, never
/// failed, and permanently pending -- which kills snapshot and restore.
/// Answering a single effect cannot reach it.
Future<void> caseBatchBothAnswered() async {
  final host = loadHost();
  var answers = 0;
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery)) {
      final chain = answers < 2; // chain only the first two, or this never ends
      answers += 1;
      host.completeEffect(id, <String, Object?>{
        'ok': <String, Object?>{'amount': 1, 'chain': chain},
      });
    }
  };
  final batch = await request(host, batch: true);
  check(answers >= 2, 'the handler answered both effects of the batch');
  check(
    awaited(batch, 'both-answered batch') == 0,
    'a fully-answered chaining batch leaves nothing outstanding',
  );
  final (made, why) = snapshotMade(host);
  check(
    made,
    'snapshot still works after a fully-answered chaining batch${suffix(made, why)}',
  );
}

/// A batch where the handler answers one and ignores the other.
Future<void> caseBatchPartlyAnswered() async {
  final host = loadHost();
  var answeredOne = false;
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery) && !answeredOne) {
      answeredOne = true;
      host.completeEffect(id, <String, Object?>{
        'ok': <String, Object?>{'amount': 3, 'chain': true},
      });
    }
  };
  final batch = await request(host, batch: true);
  check(
    awaited(batch, 'mixed batch') == 0,
    'a partly-answered batch leaves nothing outstanding',
  );
  final (made, why) = snapshotMade(host);
  check(
    made,
    'snapshot still works after a partly-answered batch${suffix(made, why)}',
  );
}

/// A handler that throws.
///
/// The handler runs inside the settle loop, so an escaping exception leaves the
/// id in `_awaiting` with nothing left to discharge it -- and the runtime
/// refuses to snapshot or restore while anything is pending, so one throwing
/// handler costs the process its persistence for good.
Future<void> caseThrowingHandler() async {
  final host = loadHost();
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery)) throw StateError('the dialog exploded');
  };
  final thrown = await request(host);
  check(
    awaited(thrown, 'throwing handler') == 0,
    'a handler that throws does not leave the effect pending',
  );
  check(
    status(thrown).contains('the dialog exploded'),
    'the app is told the handler failed, and why',
  );
  final (made, why) = snapshotMade(host);
  check(made, 'snapshot survives a handler that threw${suffix(made, why)}');
}

/// The shape a real file-dialog handler writes first: hand the chosen path back
/// as the `File` the dialog returned.
///
/// `jsonEncode` has no encoder for it and throws, from inside the handler, from
/// inside the settle -- the same wedge as above by a route an app reaches on
/// its first run rather than only when something is wrong.
Future<void> caseUnconvertibleResult() async {
  final host = loadHost();
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery)) {
      host.completeEffect(id, <String, Object?>{
        'ok': <String, Object?>{'path': File('/tmp/deck.apkg')},
      });
    }
  };
  final bad = await request(host);
  check(
    awaited(bad, 'unconvertible result') == 0,
    'an unconvertible effect result does not wedge persistence',
  );
  // Dart's own wording: `JsonUnsupportedObjectError` stringifies to
  // "Converting object to an encodable object failed: Instance of '_File'".
  // Matching "encodable" rather than the class name keeps this pinned to the
  // part that names the cause.
  check(
    status(bad).contains('encodable'),
    'the app is told the result value could not be converted',
  );
  final (made, why) = snapshotMade(host);
  check(
    made,
    'snapshot survives an unconvertible effect result${suffix(made, why)}',
  );
}

/// A handler that answers every effect by minting another one, forever.
///
/// The settle loop is bounded at 64 rounds precisely so this terminates. The
/// bound has to both stop AND report: giving up quietly would leave the app
/// looking settled while the runtime still waits.
Future<void> caseRunawayChaining() async {
  final host = loadHost();
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery)) {
      host.completeEffect(id, <String, Object?>{
        'ok': <String, Object?>{'amount': 0, 'chain': true},
      });
    }
  };
  final update = await host.handleEvent(<String, Object?>{
    'event': 'requestEffect',
    'notify': false,
  });
  check(update != null, 'a runaway chain returns instead of spinning forever');
  final error = update?['error'];
  check(
    error is String && error.contains('did not settle'),
    'a runaway chain is reported rather than abandoned quietly',
  );
}

/// Take ownership, answer LATER.
///
/// Unlike the Qt, SwiftUI and Compose drivers beside this one, "later" here is
/// a later turn of the event loop rather than another thread: a Dart isolate is
/// single-threaded, so there is no lock to deadlock against and nothing to
/// marshal the answer back onto. What must still hold is that a deferred effect
/// stays outstanding, and that the late answer reaches the UI on its own.
Future<void> caseDeferred() async {
  final host = loadHost();
  int? deferredId;
  host.effectHandler = (id, kind, payload, delivery) {
    if (isAwait(delivery)) {
      deferredId = id;
      host.deferEffect(id); // "the dialog is open"
    }
  };
  var pushes = 0;
  host.setPropsChangedHandler(() => pushes += 1);

  final afterRequest = await request(host);
  final pushesBeforeAnswer = pushes;
  // Deferring something the runtime is not waiting on must be refused, or the
  // fail sweep is switched off for an effect nothing will ever answer.
  check(!host.deferEffect(99999), 'deferring an effect nothing awaits is refused');
  check(deferredId != null, 'the handler was offered the effect');
  check(
    awaited(afterRequest, 'deferred') == 1,
    'a deferred effect stays outstanding rather than being failed',
  );
  final (madeWhilePending, _) = snapshotMade(host);
  check(
    !madeWhilePending,
    'snapshot is refused while a deferred effect is outstanding',
  );

  // ...the dialog closes, on a later turn of the event loop.
  await Future<void>.delayed(const Duration(milliseconds: 20));
  host.completeEffect(deferredId!, <String, Object?>{
    'ok': <String, Object?>{'amount': 9},
  });

  // `>= 1` would have been satisfied by the push from the request above -- the
  // assertion has to be that THIS answer pushed.
  check(
    pushes > pushesBeforeAnswer,
    'the late answer reached the UI as a props change',
  );
  final settled = propsOf(await host.props());
  check(awaited(settled, 'answered-late') == 0, 'answering a deferred effect settles it');
  check(
    count(settled, 'answered-late') == 9,
    "the deferred answer's value reached the app",
  );
  final (made, why) = snapshotMade(host);
  check(
    made,
    'snapshot works again once the deferred effect is answered${suffix(made, why)}',
  );
}

Future<void> main() async {
  final probeCase = Platform.environment['MOSAIC_PROBE_CASE'] ?? '';
  switch (probeCase) {
    case 'unhandled':
      await caseUnhandled();
    case 'answered':
      await caseAnswered();
    case 'batch-both':
      await caseBatchBothAnswered();
    case 'batch-mixed':
      await caseBatchPartlyAnswered();
    case 'throwing':
      await caseThrowingHandler();
    case 'unconvertible':
      await caseUnconvertibleResult();
    case 'runaway':
      await caseRunawayChaining();
    case 'deferred':
      await caseDeferred();
    default:
      stdout.writeln('unknown MOSAIC_PROBE_CASE `$probeCase`');
      exit(2);
  }
  stdout.writeln(_failures == 0 ? 'case passed' : '$_failures check(s) failed');
  exit(_failures == 0 ? 0 : 1);
}
