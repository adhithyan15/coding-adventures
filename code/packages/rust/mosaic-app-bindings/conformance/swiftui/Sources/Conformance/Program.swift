import Foundation

@objc protocol MosaicHostBridgeObject {
  func applyProps() -> NSDictionary?
  func handleEvent(_ envelope: NSDictionary, name: NSString) -> NSDictionary?
  @objc optional func node(named name: NSString) -> NSObject?
  @objc optional func setPropsChangedHandler(_ handler: @escaping () -> Void)
}

private func require(_ condition: @autoclosure () -> Bool, _ assertion: String) {
  guard condition() else { fatalError("Failed assertion: \(assertion)") }
}

private func object(_ value: NSDictionary?, _ assertion: String) -> NSDictionary {
  guard let value else { fatalError("Failed assertion: \(assertion) was nil") }
  require(value["error"] == nil, "\(assertion) returned an error")
  return value
}

private func integer(_ value: Any?, _ assertion: String) -> Int64 {
  guard let number = value as? NSNumber else {
    fatalError("Failed assertion: \(assertion) was not numeric")
  }
  return number.int64Value
}

private func props(_ update: NSDictionary, _ assertion: String) -> NSDictionary {
  guard let value = update["props"] as? NSDictionary else {
    fatalError("Failed assertion: \(assertion) props were missing")
  }
  return value
}

private func run(libraryPath: String?) {
  guard let host = MosaicRuntimeHost.load(libraryPath: libraryPath) else {
    fatalError("standard SwiftUI binding did not load the Rust app")
  }
  defer { host.close() }

  let started = object(host.applyProps(), "startup update")
  let startedProps = props(started, "startup update")
  require(integer(started["revision"], "startup revision") == 1, "startup revision")
  require(integer(startedProps["count"], "initial count") == 0, "initial count")
  require(startedProps["platform"] as? String == "apple", "startup platform")
  require(startedProps["status"] as? String == "started", "startup status")

  var notificationCount = 0
  host.setPropsChangedHandler { notificationCount += 1 }

  let event: NSDictionary = ["payload": ["amount": 4]]
  let dispatched = object(
    host.handleEvent(event, name: "increment" as NSString),
    "dispatch update"
  )
  let dispatchedProps = props(dispatched, "dispatch update")
  require(integer(dispatched["revision"], "dispatch revision") == 2, "dispatch revision")
  require(integer(dispatchedProps["count"], "dispatched count") == 4, "dispatched count")
  require(dispatchedProps["status"] as? String == "dispatched", "dispatch status")
  require(notificationCount == 1, "dispatch props-change notification")

  let snapshot = object(host.snapshot(), "snapshot")
  require(snapshot["schema"] as? String == "mosaic-app-conformance/counter", "snapshot schema")
  require(integer(snapshot["version"], "snapshot version") == 1, "snapshot version")
  require((snapshot["bytes"] as? NSArray)?.count == 8, "snapshot bytes")

  let restored = object(host.restore(snapshot), "restore update")
  let restoredProps = props(restored, "restore update")
  require(integer(restored["revision"], "restore revision") == 3, "restore revision")
  require(integer(restoredProps["count"], "restored count") == 4, "restored count")
  require(restoredProps["status"] as? String == "restored", "restore status")
  require(notificationCount == 2, "restore props-change notification")

  print("Mosaic SwiftUI Rust runtime conformance passed")
}

@main
private enum ConformanceMain {
  static func main() {
    let arguments = Array(CommandLine.arguments.dropFirst())
    let libraryPath: String?
    if arguments.isEmpty {
      libraryPath = nil
    } else if arguments.count == 2, arguments[0] == "--library" {
      libraryPath = arguments[1]
    } else {
      fatalError("usage: Conformance [--library <path>]")
    }
    run(libraryPath: libraryPath)
  }
}
