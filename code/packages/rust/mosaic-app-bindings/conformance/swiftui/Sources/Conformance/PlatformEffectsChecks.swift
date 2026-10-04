// Behaviour of the SwiftUI platform library (UI87 §7) without a display: the
// panels are replaced by a fake that returns a chosen file (or cancels), and
// the host by a fake that records deferrals and answers, so the real
// open/save logic, limits and routing run as they would in an app.
//
// Compiled only with -DMOSAIC_PLATFORM_EFFECTS, after copying a generated
// project's Sources/App/MosaicPlatformEffects.swift beside this file:
//
//   swift run --package-path <harness> -Xswiftc -DMOSAIC_PLATFORM_EFFECTS \
//     Conformance --platform-effects
//
// (CI does this in the SwiftUI runtime lane.) Without the flag the harness
// builds exactly as before, which is what the TaskApp release workflow uses.

import Foundation

#if MOSAIC_PLATFORM_EFFECTS

private func check(_ condition: @autoclosure () -> Bool, _ assertion: String) {
  guard condition() else { fatalError("Failed platform-effects assertion: \(assertion)") }
}

private final class FakeDialogs: MosaicFileDialogs {
  let choice: URL?
  private(set) var opened = 0
  private(set) var lastExtensions: [String] = []
  private(set) var lastSuggestedName: String?

  init(_ choice: URL?) { self.choice = choice }

  func chooseFileToOpen(extensions: [String]) -> URL? {
    opened += 1
    lastExtensions = extensions
    return choice
  }

  func chooseFileToSave(suggestedName: String, extensions: [String]) -> URL? {
    opened += 1
    lastExtensions = extensions
    lastSuggestedName = suggestedName
    return choice
  }
}

private final class FakeHost: MosaicPlatformEffectHost {
  var effectHandler: ((UInt64, String, Any, String) -> Void)?
  var waitingOn: Set<UInt64> = []
  private(set) var deferred: [UInt64] = []
  private(set) var answers: [UInt64: [String: Any]] = [:]

  func deferEffect(_ id: UInt64) -> Bool {
    guard waitingOn.contains(id) else { return false }
    deferred.append(id)
    return true
  }

  func completeEffect(_ id: UInt64, _ result: [String: Any]) -> NSDictionary? {
    answers[id] = result
    waitingOn.remove(id)
    return nil
  }
}

private func encoded(_ text: String) -> String { Data(text.utf8).base64EncodedString() }

private func failure(_ outcome: [String: Any]) -> String? {
  (outcome["failed"] as? [String: Any])?["message"] as? String
}

private func isCancelled(_ outcome: [String: Any]) -> Bool {
  outcome.count == 1 && (outcome["cancelled"] as? [String: Any])?.isEmpty == true
}

private func okValue(_ outcome: [String: Any]) -> [String: Any]? {
  outcome["ok"] as? [String: Any]
}

private func checkRouting() {
  // The app claims a kind: it goes to the app, even a standard one.
  check(mosaicRoutesToPlatform("files.save", appKinds: ["files.save"]) == false, "claimed standard")
  // A standard kind nobody claimed: the platform library.
  check(mosaicRoutesToPlatform("files.open", appKinds: ["importAnki"]) == true, "unclaimed open")
  check(mosaicRoutesToPlatform("files.save", appKinds: nil) == true, "unclaimed save")
  // A non-standard kind: the app when it claimed nothing (the original
  // meaning), nobody when it named its kinds and this is not one.
  check(mosaicRoutesToPlatform("importAnki", appKinds: nil) == false, "unclaimed custom")
  check(mosaicRoutesToPlatform("other", appKinds: ["importAnki"]) == nil, "unowned custom")
}

private func checkStartLocation() {
  // UI59 §2: a "pictures only" open starts in the Pictures folder.
  check(mosaicOnlyImages(["png", "jpg", "jpeg"]), "images start in Pictures")
  check(mosaicOnlyImages(["svg"]), "svg is an image")
  check(!mosaicOnlyImages(["png", "txt"]), "mixed types keep the default start")
  check(!mosaicOnlyImages([]), "any file keeps the default start")
  check(!mosaicOnlyImages(["pdf"]), "a document keeps the default start")
}

private func checkSave(in directory: URL) {
  let target = directory.appendingPathComponent("journal-2026-09-25.json")
  let dialogs = FakeDialogs(target)
  let outcome = mosaicRunFilesSave(
    [
      "suggestedName": "journal-2026-09-25.json",
      "accept": ["application/json"],
      "bytes": encoded("{\"version\":1}"),
    ] as [String: Any],
    dialogs: dialogs
  )
  check(okValue(outcome)?["name"] as? String == "journal-2026-09-25.json", "save ok name")
  check((try? String(contentsOf: target, encoding: .utf8)) == "{\"version\":1}", "saved bytes")
  check(dialogs.lastExtensions == ["json"], "save panel filtered to json")
  check(dialogs.lastSuggestedName == "journal-2026-09-25.json", "suggested name reached panel")
  let leftovers = (try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []
  check(!leftovers.contains { $0.hasSuffix(".tmp") }, "no temporary file left behind")

  // Saving over a private file keeps it private.
  let secret = directory.appendingPathComponent("secret.json")
  FileManager.default.createFile(
    atPath: secret.path, contents: Data("old".utf8), attributes: [.posixPermissions: 0o600])
  let replaced = mosaicRunFilesSave(
    ["suggestedName": "secret.json", "bytes": encoded("new")] as [String: Any],
    dialogs: FakeDialogs(secret)
  )
  check(okValue(replaced) != nil, "save over existing file")
  let permissions =
    (try? FileManager.default.attributesOfItem(atPath: secret.path))?[.posixPermissions] as? Int
  check(permissions == 0o600, "replaced file keeps 0600, got \(String(describing: permissions))")
  check((try? String(contentsOf: secret, encoding: .utf8)) == "new", "replaced contents")

  // Only the rwx bits carry over: never setuid, setgid or sticky.
  let tool = directory.appendingPathComponent("tool.txt")
  FileManager.default.createFile(
    atPath: tool.path, contents: Data("old".utf8), attributes: [.posixPermissions: 0o4755])
  check(
    okValue(
      mosaicRunFilesSave(
        ["suggestedName": "tool.txt", "bytes": encoded("new")] as [String: Any],
        dialogs: FakeDialogs(tool))) != nil, "save over a setuid file")
  let toolPermissions =
    (try? FileManager.default.attributesOfItem(atPath: tool.path))?[.posixPermissions] as? Int
  check(toolPermissions == 0o755, "setuid is not copied, got \(String(describing: toolPermissions))")

  // A new file stays owner-only.
  let fresh = directory.appendingPathComponent("fresh.txt")
  _ = mosaicRunFilesSave(
    ["suggestedName": "fresh.txt", "bytes": encoded("x")] as [String: Any],
    dialogs: FakeDialogs(fresh))
  let freshPermissions =
    (try? FileManager.default.attributesOfItem(atPath: fresh.path))?[.posixPermissions] as? Int
  check(freshPermissions == 0o600, "a new file is 0600, got \(String(describing: freshPermissions))")
}

private func checkSaveRefusals(in directory: URL) {
  let cancelled = mosaicRunFilesSave(
    ["suggestedName": "a.json", "bytes": encoded("x")] as [String: Any],
    dialogs: FakeDialogs(nil)
  )
  check(isCancelled(cancelled), "a cancelled panel is not a failure")

  let target = directory.appendingPathComponent("never.txt")
  for name in [
    "", ".", "..", "../escape.json", "a/b.json", "a\\b.json", "C:x.json", "trailing.",
    "trailing ", "bell\u{7}.json", "invoice\u{202E}fdp.exe", String(repeating: "a", count: 256),
    ".zshrc", " leading.json", "nbsp\u{00A0}", "\u{3000}ideographic.json",
    "line\u{2028}break.json", "para\u{2029}break.json", "tag\u{E0001}.json",
    "Invoice.pdf      .command",
    ".\u{0301}zshrc", "Invoice.pdf\u{2800}\u{2800}.txt",
    "x\u{0D4E}.", "a\u{E000}.json",
    "Invoice.pdf" + String(repeating: " \u{FE00}", count: 30) + " x.html", "\u{FE00}.zshrc",
    "notes.txt\u{FE00}.",
  ] {
    let dialogs = FakeDialogs(target)
    let outcome = mosaicRunFilesSave(
      ["suggestedName": name, "bytes": encoded("x")] as [String: Any], dialogs: dialogs)
    check(failure(outcome) == "suggestedName must be a plain file name", "refused name \(name)")
    check(dialogs.opened == 0, "no panel for refused name \(name)")
  }

  let mismatched = mosaicRunFilesSave(
    ["suggestedName": "notes.exe", "accept": ["application/json"], "bytes": encoded("x")]
      as [String: Any],
    dialogs: FakeDialogs(target)
  )
  check(
    failure(mismatched) == "suggestedName must end in an extension of an accepted type",
    "extension must match the accepted type")

  // With no accepted type, a name that would run when opened is refused.
  for name in [
    "run.command", "open.terminal", "site.webloc", "setup.EXE", "go.desktop", "a.ps1",
    "a.j\u{017F}", "img.iso", "clip.scf", "app.AppImage",
    // A Prepend letter merges with the dot into one Character; by scalar the
    // extension is still `.terminal` (the security review's bypass).
    "run\u{0D4E}.terminal",
  ] {
    let dialogs = FakeDialogs(directory.appendingPathComponent(name))
    let outcome = mosaicRunFilesSave(
      ["suggestedName": name, "bytes": encoded("x")] as [String: Any], dialogs: dialogs)
    check(
      failure(outcome) == "suggestedName must not end in an executable extension",
      "executable extension \(name)")
    check(dialogs.opened == 0, "no panel for executable \(name)")
  }
  check(mosaicIsPlainFileName("caf\u{00E9} menu.json"), "accented names still pass")
  check(mosaicIsPlainFileName("\u{2764}\u{FE0F} list.txt"), "an emoji's own selector is fine")
  check(!mosaicHasExecutableExtension("notes.txt") && !mosaicHasExecutableExtension("README"),
    "an ordinary document is not executable")

  let notBase64 = mosaicRunFilesSave(
    ["suggestedName": "a.txt", "bytes": "%%%"] as [String: Any], dialogs: FakeDialogs(target))
  check(failure(notBase64) == "bytes must be base64 text", "invalid base64")

  let tooLarge = String(repeating: "A", count: (mosaicMaxSaveBytes / 3 + 2) * 4)
  let oversized = mosaicRunFilesSave(
    ["suggestedName": "a.txt", "bytes": tooLarge] as [String: Any], dialogs: FakeDialogs(target))
  check(failure(oversized)?.hasPrefix("the file is larger than") == true, "oversized save")
  check(!FileManager.default.fileExists(atPath: target.path), "refusals write nothing")
}

private func checkNameSafety() {
  // UI87 §3.1: Windows device names are never plain names, on any host.
  for name in ["CON", "con.txt", "Nul.json", "COM1.json", "lpt9", "COM\u{B9}.json", "CON .txt", "CONIN$.log", "aux.tar.gz"] {
    check(!mosaicIsPlainFileName(name), "device name refused: \(name)")
  }
  for name in ["console.txt", "CONFIG.json", "aux-notes.txt", "COM10.json", "my.CON", "nul report.json", "CON\u{131}N$.txt"] {
    check(mosaicIsPlainFileName(name), "not a device name: \(name)")
  }
  // Active content and non-ASCII extensions count as executable.
  for name in ["page.html", "page.HTM", "card.svg", "archive.mht", "shortcut.website", "report.xlsm", "deck.pptm", "tool.py", "invoice.\u{435}x\u{435}", "setup.exe\u{301}", "macros.xlsb", "addin.xla", "link.iqy", "sheet.slk", "remote.rdp", "app.pyzw", "cache.pyc"] {
    check(mosaicHasExecutableExtension(name), "executable: \(name)")
  }
  for name in ["notes.txt", "data.xlsx", "report.docx", "photo.png"] {
    check(!mosaicHasExecutableExtension(name), "not executable: \(name)")
  }
}

private func checkOpen(in directory: URL) {
  let source = directory.appendingPathComponent("photo.PNG")
  FileManager.default.createFile(atPath: source.path, contents: Data([1, 2, 3]))
  let dialogs = FakeDialogs(source)
  let outcome = mosaicRunFilesOpen(
    ["accept": ["image/png", "image/jpeg", "image/png"]] as [String: Any], dialogs: dialogs)
  let value = okValue(outcome)
  check(value?["name"] as? String == "photo.PNG", "open returns the name, never a path")
  check(value?["mimeType"] as? String == "image/png", "mime type from the extension")
  check(value?["bytes"] as? String == Data([1, 2, 3]).base64EncodedString(), "open bytes")
  check(dialogs.lastExtensions == ["png", "jpg", "jpeg"], "extensions deduplicated in order")

  check(isCancelled(mosaicRunFilesOpen(NSNull(), dialogs: FakeDialogs(nil))), "open cancelled")
  check(
    failure(mosaicRunFilesOpen(NSNull(), dialogs: FakeDialogs(directory)))
      == "that is not a regular file", "a directory is refused")

  // A FIFO is refused from the open descriptor, at once -- opening it for a
  // blocking read would hang the main queue for ever.
  let fifo = directory.appendingPathComponent("pipe")
  check(mkfifo(fifo.path, 0o600) == 0, "mkfifo")
  check(
    failure(mosaicRunFilesOpen(NSNull(), dialogs: FakeDialogs(fifo)))
      == "that is not a regular file", "a FIFO is refused without blocking")

  // A symlink the person chose is resolved when the panel returns, and read.
  let link = directory.appendingPathComponent("link.png")
  check(symlink(source.path, link.path) == 0, "symlink")
  let linked = okValue(mosaicRunFilesOpen(NSNull(), dialogs: FakeDialogs(link)))
  check(linked?["bytes"] as? String == Data([1, 2, 3]).base64EncodedString(), "chosen symlink")
  check(linked?["name"] as? String == "link.png", "the chosen name, not the link target")

  // One byte over the limit, bounded while reading.
  let large = directory.appendingPathComponent("large.bin")
  FileManager.default.createFile(atPath: large.path, contents: nil)
  if let handle = try? FileHandle(forWritingTo: large) {
    try? handle.truncate(atOffset: UInt64(mosaicMaxOpenBytes + 1))
    try? handle.close()
  }
  check(
    failure(mosaicRunFilesOpen(NSNull(), dialogs: FakeDialogs(large)))?
      .hasPrefix("the selected file is larger than") == true, "oversized open")
}

private func checkRouter(in directory: URL) {
  let target = directory.appendingPathComponent("routed.txt")
  let payload: [String: Any] = ["suggestedName": "routed.txt", "bytes": encoded("hi")]

  // A standard kind is deferred, then answered from the UI queue.
  var queued: [() -> Void] = []
  let host = FakeHost()
  var appCalls: [String] = []
  host.effectHandler = { _, kind, _, _ in appCalls.append(kind) }
  installMosaicPlatformEffects(
    host, appKinds: ["importAnki"], dialogs: FakeDialogs(target), hasDialogs: true,
    runOnUI: { queued.append($0) })
  // Idempotent: a second install does not stack a second router.
  installMosaicPlatformEffects(
    host, appKinds: nil, dialogs: FakeDialogs(nil), hasDialogs: true,
    runOnUI: { $0() })

  host.waitingOn = [1, 2]
  host.effectHandler?(1, "files.save", payload, "await")
  check(host.deferred == [1], "a standard kind is deferred before any panel")
  check(host.answers[1] == nil && queued.count == 1, "answered later, on the UI queue")
  // One file operation at a time: a second request while the first is open.
  host.effectHandler?(2, "files.save", payload, "await")
  check(failure(host.answers[2] ?? [:]) == "another file operation is in progress", "busy")
  queued.removeFirst()()
  check(okValue(host.answers[1] ?? [:])?["name"] as? String == "routed.txt", "deferred answer")

  // The app's claimed kind reaches the app; an unclaimed custom kind reaches
  // nobody (the host fails it); a notify needs no answer.
  host.effectHandler?(3, "importAnki", NSNull(), "await")
  host.effectHandler?(4, "somethingElse", NSNull(), "await")
  host.waitingOn = [5]
  host.effectHandler?(5, "files.open", NSNull(), "notify")
  check(appCalls == ["importAnki"], "only the claimed kind reaches the app: \(appCalls)")
  check(host.answers[4] == nil && host.answers[5] == nil, "nothing answered for 4 and 5")
  check(queued.isEmpty, "a notify opens no panel")

  // Not waiting on the id: no panel, and the router is free again.
  host.waitingOn = []
  host.effectHandler?(6, "files.open", NSNull(), "await")
  check(queued.isEmpty && host.answers[6] == nil, "an id nobody awaits opens nothing")
  host.waitingOn = [7]
  host.effectHandler?(7, "files.open", NSNull(), "await")
  check(queued.count == 1, "the router is not left busy by a refused deferral")

  // Without panels (iOS for now): failed at once, never deferred.
  let phone = FakeHost()
  installMosaicPlatformEffects(
    phone, appKinds: nil, dialogs: FakeDialogs(target), hasDialogs: false,
    runOnUI: { _ in fatalError("no panel may be scheduled without dialogs") })
  phone.waitingOn = [1]
  phone.effectHandler?(1, "files.open", NSNull(), "await")
  check(phone.deferred.isEmpty, "nothing deferred without dialogs")
  check(
    failure(phone.answers[1] ?? [:]) == "files.open is not available on this platform yet",
    "a clear failure without dialogs")
}

func runPlatformEffectsChecks() {
  let directory = FileManager.default.temporaryDirectory
    .appendingPathComponent("mosaic-platform-effects-\(UUID().uuidString)")
  try! FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
  defer { try? FileManager.default.removeItem(at: directory) }

  checkRouting()
  checkStartLocation()
  checkSave(in: directory)
  checkSaveRefusals(in: directory)
  checkNameSafety()
  checkOpen(in: directory)
  checkRouter(in: directory)
  print("Mosaic SwiftUI platform effects passed")
}

#else

func runPlatformEffectsChecks() {
  fatalError(
    "built without MOSAIC_PLATFORM_EFFECTS: copy a generated MosaicPlatformEffects.swift "
      + "here and run with -Xswiftc -DMOSAIC_PLATFORM_EFFECTS")
}

#endif
