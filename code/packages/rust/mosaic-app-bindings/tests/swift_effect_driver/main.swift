// Drives the emitted SwiftUI host against the conformance runtime.
//
// This crate's other tests assert on the TEXT of the emitted host. That cannot
// tell you it compiles, let alone that it behaves — and the defect this exists
// for is behavioural: before the host completed effects, an `await` was dropped
// and the app waited forever with nothing reporting it.
import Foundation

// The protocol the emitted app declares; the host conforms to it.
@objc protocol MosaicHostBridgeObject {
  func applyProps() -> NSDictionary?
  func handleEvent(_ envelope: NSDictionary, name: NSString) -> NSDictionary?
  @objc optional func node(named name: NSString) -> NSObject?
  @objc optional func setPropsChangedHandler(_ handler: @escaping () -> Void)
  @objc optional func runInteractionAcceptance()
}

var failures = 0

func check(_ ok: Bool, _ what: String) {
  let label = what.count >= 56 ? what : what + String(repeating: " ", count: 56 - what.count)
  print("\(label) \(ok ? "ok" : "FAIL")")
  if !ok { failures += 1 }
}

// A missing key must not read as zero: comparing against 0 would pass when the
// props are empty, which is how the Qt version of this file first passed.
func awaited(_ props: [String: Any], _ context: String) -> Int {
  guard let value = props["awaitedEffects"] as? NSNumber else {
    print("VACUOUS: \(context) carries no awaitedEffects key")
    failures += 1
    return -1
  }
  return value.intValue
}

func props(_ update: NSDictionary?) -> [String: Any] {
  ((update as? [String: Any])?["props"] as? [String: Any]) ?? [:]
}

func request(_ host: MosaicRuntimeHost, batch: Bool = false) -> [String: Any] {
  let envelope: NSDictionary = ["payload": ["notify": false] as NSDictionary]
  return props(host.handleEvent(envelope, name: batch ? "requestEffectBatch" : "requestEffect"))
}

func makeHost(_ stateKey: String) -> MosaicRuntimeHost? {
  // Its own state file: hosts persist, and a shared path lets one restore
  // another's count, so an assertion reads a number its host never produced.
  if let path = ProcessInfo.processInfo.environment[stateKey] {
    setenv("MOSAIC_APP_STATE_PATH", path, 1)
  }
  return MosaicRuntimeHost.load()
}

guard let first = makeHost("MOSAIC_PROBE_STATE_A") else {
  print("host failed to load")
  exit(2)
}
check(awaited(props(first.applyProps()), "startup") == 0, "a fresh app awaits nothing")

// No handler connected — what every native host did before completion existed.
let unhandled = request(first)
check(awaited(unhandled, "unhandled") == 0, "an unanswered await is failed, not dropped")
check((unhandled["status"] as? String ?? "").contains("no host handler"),
      "the app is told why, rather than just waiting")
check((unhandled["count"] as? NSNumber)?.intValue == 0,
      "a failed completion does not advance the app")

// A handler that answers properly.
if let host = makeHost("MOSAIC_PROBE_STATE_B") {
  host.effectHandler = { [weak host] id, _, _, delivery in
    guard delivery.lowercased() == "await" else { return }
    host?.completeEffect(id, ["ok": ["amount": 5]])
  }
  let answered = request(host)
  check(awaited(answered, "handled") == 0, "an answered await is settled")
  check((answered["count"] as? NSNumber)?.intValue == 5, "the handler's value reached the app")
}

// A batch where the handler answers BOTH, each chaining.
//
// One slot holding "the last update" drops the first answer's minted effect:
// it exists in no map the host kept, so it is never emitted, never failed, and
// permanently pending — which kills snapshot and restore. Answering a single
// effect cannot reach it.
if let host = makeHost("MOSAIC_PROBE_STATE_C") {
  var answers = 0
  host.effectHandler = { [weak host] id, _, _, delivery in
    guard delivery.lowercased() == "await" else { return }
    let chain = answers < 2   // chain only the first two, or this never ends
    answers += 1
    host?.completeEffect(id, ["ok": ["amount": 1, "chain": chain]])
  }
  let batch = request(host, batch: true)
  check(answers >= 2, "the handler answered both effects of the batch")
  check(awaited(batch, "both-answered batch") == 0,
        "a fully-answered chaining batch leaves nothing outstanding")
  let snapshot = host.snapshot() as? [String: Any]
  check(snapshot != nil && snapshot?["error"] == nil,
        "snapshot still works after a fully-answered chaining batch")
}

// A batch where the handler answers one and ignores the other.
if let host = makeHost("MOSAIC_PROBE_STATE_D") {
  var answeredOne = false
  host.effectHandler = { [weak host] id, _, _, delivery in
    guard delivery.lowercased() == "await", !answeredOne else { return }
    answeredOne = true
    host?.completeEffect(id, ["ok": ["amount": 3, "chain": true]])
  }
  let batch = request(host, batch: true)
  check(awaited(batch, "mixed batch") == 0,
        "a partly-answered batch leaves nothing outstanding")
  let snapshot = host.snapshot() as? [String: Any]
  check(snapshot != nil && snapshot?["error"] == nil,
        "snapshot still works after a partly-answered batch")
}

// The shape a real file-dialog handler writes first.
//
// `JSONSerialization` raises an ObjC NSInvalidArgumentException for a URL --
// not a Swift error, so the host's do/catch cannot see it and the PROCESS
// ABORTS with the lock held. Without the guard this check does not fail, it
// terminates the driver.
if let host = makeHost("MOSAIC_PROBE_STATE_E") {
  let refused = host.completeEffect(1, ["ok": ["url": URL(fileURLWithPath: "/tmp/x.apkg")]])
    as? [String: Any]
  check((refused?["error"] as? String ?? "").contains("JSON-serialisable"),
        "a non-serialisable effect result is refused, not aborted on")
  let nan = host.completeEffect(1, ["ok": ["amount": Double.nan]]) as? [String: Any]
  check((nan?["error"] as? String ?? "").contains("JSON-serialisable"),
        "a non-finite number is refused, not aborted on")
}

// No id-validation check here, deliberately, and the reason is worth recording.
//
// Qt needed one: `completeEffect` is `Q_INVOKABLE` and takes a `QVariant`, so
// QML could hand it 3.5 and `toULongLong` would truncate that to 4 — answering
// a DIFFERENT outstanding effect. Swift's signature is `UInt64`, so the same
// mistake does not compile. The validation that remains is `effectId`, which
// parses ids out of the runtime's own updates, and that is exercised by every
// check above.

print(failures == 0 ? "\nall checks passed" : "\n\(failures) check(s) failed")
exit(failures == 0 ? 0 : 1)
