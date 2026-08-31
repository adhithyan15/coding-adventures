import Foundation

@objc protocol MosaicHostBridgeObject {
  func applyProps() -> NSDictionary?
  func handleEvent(_ envelope: NSDictionary, name: NSString) -> NSDictionary?
  @objc optional func node(named name: NSString) -> NSObject?
  @objc optional func setPropsChangedHandler(_ handler: @escaping () -> Void)
}

private let taskName = "Native acceptance task"
private let persistedTaskName = "Persisted native task"
private let due = "2026-01-09"
private let schedule = "2026-01-05 → 2026-01-05"

private func require(_ condition: @autoclosure () -> Bool, _ assertion: String) {
  guard condition() else { fatalError("Failed assertion: \(assertion)") }
}

private func object(_ value: NSDictionary?, _ assertion: String) -> NSDictionary {
  guard let value else { fatalError("Failed assertion: \(assertion) was nil") }
  require(value["error"] == nil, "\(assertion) returned an error")
  return value
}

private func props(_ update: NSDictionary?, _ assertion: String) -> NSDictionary {
  let value = object(update, assertion)
  guard let result = value["props"] as? NSDictionary else {
    fatalError("Failed assertion: \(assertion) props were missing")
  }
  return result
}

private func rows(_ values: NSDictionary, _ assertion: String) -> [[Any]] {
  guard let rows = values["task-rows"] as? [[Any]] else {
    fatalError("Failed assertion: \(assertion) task-rows were missing")
  }
  return rows
}

private func dispatch(
  _ host: MosaicRuntimeHost,
  _ name: String,
  _ payload: NSDictionary = [:]
) -> NSDictionary {
  props(host.handleEvent(["payload": payload], name: name as NSString), name)
}

private func requireTask(_ taskRows: [[Any]], _ name: String) {
  require(taskRows.count == 1, "one task row")
  let row = taskRows[0]
  require(row.count >= 4, "task row projection width")
  require(row[1] as? String == name, "task name projection")
  require(row[2] as? String == "due \(due)", "task due projection")
  require(row[3] as? String == schedule, "Rust schedule start/finish projection")
}

private func canonical(_ value: NSDictionary?) -> Data? {
  guard let value else { return nil }
  return try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
}

private func run(libraryPath: String?) {
  let restoredOnLaunch = ProcessInfo.processInfo.environment["MOSAIC_EXPECT_RESTORED"] == "1"
  let expectedRestoredTask = ProcessInfo.processInfo.environment["MOSAIC_EXPECT_TASK_NAME"]
    ?? persistedTaskName
  let host = MosaicRuntimeHost.loadRequired(libraryPath: libraryPath)
  defer { host.close() }

  var current = props(host.applyProps(), "startup update")
  if restoredOnLaunch {
    requireTask(rows(current, "restored startup"), expectedRestoredTask)
    current = dispatch(host, "onDeleteTask", ["index": 0])
    require(rows(current, "restored delete").isEmpty, "delete restored task")
    print("TaskApp SwiftUI persisted-restart conformance passed")
    return
  }

  require(rows(current, "fresh startup").isEmpty, "fresh task list")
  let before = canonical(host.snapshot())
  let rejected = host.handleEvent(
    ["payload": ["value": 7]],
    name: "onNewTaskNameChange" as NSString
  )
  require(rejected?["error"] != nil, "invalid input rejected")
  require(canonical(host.snapshot()) == before, "invalid input preserved state")

  _ = dispatch(host, "onNewTaskNameChange", ["value": taskName])
  _ = dispatch(host, "onNewTaskDueChange", ["value": due])
  current = dispatch(host, "onAddTask")
  require(rows(current, "created task")[0][3] as? String == "", "Board mode hides schedule")
  current = dispatch(host, "onToggleProjectComplexity")
  requireTask(rows(current, "created task"), taskName)

  current = dispatch(host, "onToggleTask", ["index": 0])
  require(rows(current, "completed task")[0][0] as? String == "✓", "complete task")
  require(current["ring-percent"] as? String == "100%", "completion projection")
  current = dispatch(host, "onToggleTask", ["index": 0])
  require(rows(current, "reopened task")[0][0] as? String == "○", "reopen task")
  current = dispatch(host, "onDeleteTask", ["index": 0])
  require(rows(current, "deleted task").isEmpty, "delete task")

  _ = dispatch(host, "onNewTaskNameChange", ["value": persistedTaskName])
  _ = dispatch(host, "onNewTaskDueChange", ["value": due])
  current = dispatch(host, "onAddTask")
  requireTask(rows(current, "persisted task"), persistedTaskName)
  print("TaskApp SwiftUI native lifecycle conformance passed")
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
