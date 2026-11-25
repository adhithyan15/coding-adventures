/// A pluggable, generic event loop — the heartbeat of any
/// interactive application.
///
/// ```text
/// while running:
///     collect events from all sources
///     for each event:
///         dispatch to handlers
///         if any handler says "exit" → stop
/// ```
///
/// `EventLoop<E>` is generic over the event type `E`: callers define
/// what events look like, inject sources emitting exactly those
/// events, and register handlers. All sources and handlers run on
/// the thread that calls [EventLoop.run]; Dart isolates needing to
/// inject events should buffer into a list drained by `poll`.
///
/// ## Port notes
///
/// `continue` is a Dart keyword, so the reference's
/// `ControlFlow::Continue` is [ControlFlow.continueLoop]. The
/// reference's idle-phase `yield_now` has no equivalent in
/// synchronous Dart code, so the idle phase is a deliberate no-op
/// here; everything else (collection order, immediate exit, stop
/// flag checked at the top of each iteration, flag reset on
/// [EventLoop.run]) mirrors the reference exactly.
library;

/// Signals whether the event loop should continue running or stop.
///
/// An enum instead of `bool` keeps call sites self-documenting and
/// leaves room for future variants without breaking handlers.
enum ControlFlow {
  /// Keep looping — there is more work to do.
  continueLoop,

  /// Stop the loop immediately after this event. Subsequent
  /// handlers for the same event are not called.
  exit,
}

/// Anything that can produce events for the loop to dispatch.
///
/// The critical contract: **`poll` must return immediately**.
/// Return an empty list if nothing is ready. Never block —
/// blocking is the loop's job.
abstract class EventSource<E> {
  /// Returns all events currently available. Must not block.
  List<E> poll();
}

/// Shared stop flag between a loop and its handles.
class _StopFlag {
  bool stopped = false;
}

/// A handle for stopping the event loop from outside a handler.
///
/// Obtain via [EventLoop.stopHandle]. Handles share the loop's
/// underlying flag.
class StopHandle {
  final _StopFlag _flag;

  StopHandle._(this._flag);

  /// Signals the associated event loop to stop on its next
  /// iteration.
  void stop() {
    _flag.stopped = true;
  }
}

/// A pluggable, generic event loop.
class EventLoop<E> {
  final List<EventSource<E>> _sources = [];
  final List<ControlFlow Function(E)> _handlers = [];
  final _StopFlag _stopped = _StopFlag();

  /// Creates a new, empty event loop.
  EventLoop();

  /// Registers an event source. Sources are polled in registration
  /// order.
  void addSource(EventSource<E> source) {
    _sources.add(source);
  }

  /// Registers an event handler.
  ///
  /// Handlers receive each event in registration order. If any
  /// handler returns [ControlFlow.exit], the loop stops
  /// immediately — subsequent handlers for the same event are not
  /// called.
  void onEvent(ControlFlow Function(E) handler) {
    _handlers.add(handler);
  }

  /// Returns a [StopHandle] that can stop this loop from outside a
  /// handler.
  StopHandle stopHandle() => StopHandle._(_stopped);

  /// Signals the loop to exit on the next iteration.
  void stop() {
    _stopped.stopped = true;
  }

  /// Starts the event loop. Blocks until a handler returns
  /// [ControlFlow.exit] or [stop] is called.
  ///
  /// Each iteration performs three phases:
  ///
  /// 1. **Collect** — poll every source; append results to a local
  ///    queue.
  /// 2. **Dispatch** — deliver each queued event to every handler
  ///    in order. Stop immediately if any handler returns [ControlFlow.exit].
  /// 3. **Idle** — when the queue was empty the reference yields
  ///    to the OS scheduler; synchronous Dart code cannot yield,
  ///    so this phase is intentionally a no-op here.
  void run() {
    _stopped.stopped = false;

    while (true) {
      // Check the stop flag at the top of each iteration so that a
      // call to stop() takes effect even when no events arrive.
      if (_stopped.stopped) {
        return;
      }

      // Phase 1: Collect.
      final queue = <E>[];
      for (final source in _sources) {
        queue.addAll(source.poll());
      }

      // Phase 2: Dispatch.
      var shouldExit = false;
      for (final event in queue) {
        for (final handler in _handlers) {
          if (handler(event) == ControlFlow.exit) {
            shouldExit = true;
            break;
          }
        }
        if (shouldExit) {
          break;
        }
      }
      if (shouldExit) {
        return;
      }

      // Phase 3: Idle (no-op; see the library docs).
    }
  }
}
