## 0.1.0

- Initial Dart lane of the event-loop identity: generic
  `EventLoop`/`EventSource`/`StopHandle` with collect-dispatch
  semantics, `ControlFlow.continueLoop`/`exit` (`continue` is a
  Dart keyword), and a stop flag checked per iteration and reset
  on run; the idle yield is a documented no-op (impossible from
  synchronous Dart code).
- Tests mirror the reference suite (8 unit tests), add 5 extras,
  6 seeded property tests over the in-lane `pbt` package, and a
  2,000-iteration differential fuzz campaign over the in-lane
  `fuzz` package; 100% line coverage.
