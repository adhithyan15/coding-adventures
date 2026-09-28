### Added -- an `Await` effect can be answered later (#14720)

Both hosts called the effect handler synchronously and failed anything still
unanswered when it returned. A handler therefore had exactly one option: answer
inline, on the settle thread, without blocking. Answering later was rejected as
already completed; blocking on another thread deadlocked against the lock.

That ruled out an asynchronous file dialog -- the motivating case for `await`
effects, and the flow Engram's Anki import needs. The shape could not express
the thing it was built for.

`deferEffect(id)` is the third outcome. A handler that takes ownership leaves
the effect pending instead of having it failed, and answers whenever the work
finishes, from any thread.

Two consequences worth stating:

- **A deferred answer reaches the UI on its own.** It is the return value of no
  call the UI made -- it arrives whenever the dialog closed -- so Qt gained an
  `updated(QVariantMap)` signal, and SwiftUI routes it through the
  `propsChangedHandler` it already had, **on the main thread and outside the
  lock**. Without that the app would advance while the screen kept showing the
  state from before.
- **Answering from another thread has a route on both hosts.** Qt's
  `completeEffect` touches unguarded members and refuses an off-thread call, so
  `answerDeferredEffect` queues onto the host's thread; without it the async
  flow this feature exists for had no working Qt path at all. SwiftUI's lock
  serialises, and the notification hops to main.
- **`snapshot` and `restore` stay refused while an effect is deferred.** That is
  correct rather than a defect: a half-answered import is not a state worth
  restoring. It does mean a handler that defers owes an answer -- abandoning one
  leaves the app waiting for good.

Blocking on another thread from inside the handler is still wrong, and both
templates now say so and say to defer instead -- the Qt header included, which
in the first version of this change still said the feature was impossible.

`deferEffect` refuses an id the runtime is not awaiting. Ids are sequential and
the call is reachable from QML, so an off-by-one would otherwise switch the fail
sweep off for an effect nothing will ever answer -- wedging `snapshot` and
`restore` for the life of the process, which is the failure the sweep exists to
prevent.

Settled before Compose, Flutter and XAML copy the shape, which was the point of
doing it now rather than after.

