// Answer Engram's Anki file-dialog effects on SwiftUI.
//
// Everything else about the application -- props, events, snapshot, restore --
// goes through the standard Mosaic host. This is the one thing a generated host
// cannot do on the application's behalf, because only the host has a window to
// hang a dialog off.
//
// Installed by the generated `App.swift` through `[host_effects]`, immediately
// after the host is assigned.
//
// DEFERRED, not inline -- the opposite of the Qt handler, and deliberately.
// Qt's settle runs on the thread with the event loop, so a modal dialog there
// is answered in place. This host's contract rules that out twice over:
//
//   1. `settleEffects` is not guaranteed to run on the main thread, and
//      `runModal()` off the main thread is invalid.
//   2. The host's lock is HELD across the handler call. A modal panel run
//      inline would hold it for as long as the dialog is open -- blocking
//      `applyProps()`, which SwiftUI calls every frame -- and the documented
//      escape, `DispatchQueue.main.sync`, is exactly the wedge the host warns
//      against.
//
// So each dialog takes ownership with `deferEffect`, which keeps the effect out
// of the fail sweep, and answers from the main thread when the person is done.
//
// LANGUAGE MODE: this hop compiles clean under Swift 5, which is what the
// generated `Package.swift` pins (`swift-tools-version: 5.10`). Checked, not
// assumed -- and checked in the other direction too: under `-swift-version 6`
// it does NOT compile, because `host` and the effect payload are non-Sendable
// and crossing into `DispatchQueue.main.async` is a sending violation. Whoever
// raises the pinned tools version will have to give the hop a Sendable-safe
// shape (an actor, or a `@MainActor` hop carrying only value types). It fails
// loudly at compile time, so this note is a pointer, not a guard.

import Foundation

#if os(macOS)
import AppKit
import UniformTypeIdentifiers
#endif

// The three tagged outcomes the protocol defines. Cancellation is not a
// failure: Escape in a file dialog is an ordinary thing for a person to do, and
// the application says "Import cancelled." rather than "Import failed."
private func okOutcome(_ value: [String: Any]) -> [String: Any] {
  ["ok": value]
}

private func cancelledOutcome() -> [String: Any] {
  ["cancelled": [String: Any]()]
}

private func failedOutcome(_ message: String) -> [String: Any] {
  ["failed": ["message": message]]
}

// The largest package this host will read into memory.
//
// Aligned with what the engine will actually accept: the package layer refuses
// a collection past 256 MiB on native targets, so a larger file cannot import
// whatever this does. Checked BEFORE the read, because the point is to never
// make the allocation -- one import costs several times the file's size in
// flight (the bytes, their base64, that base64 as a Swift string, the JSON
// envelope, and the runtime's own copy), and the application's own cap sits at
// the far end of all of it.
private let maxImportBytes: Int = 256 * 1024 * 1024

#if os(macOS)

// The extensions the application sent, so the picker shows what this build can
// actually read rather than a hardcoded guess that drifts from the engine.
//
// An extension is accepted only if it looks like one. The payload comes from
// this application rather than a user today, but a filter widened to everything
// is not something to leave to that staying true.
private func allowedExtensions(
  _ payload: Any,
  _ key: String,
  fallback: [String]
) -> [String] {
  let bare: (String) -> String = { $0.hasPrefix(".") ? String($0.dropFirst()) : $0 }
  let declared = (payload as? [String: Any])?[key] as? [String] ?? []
  let source = declared.isEmpty ? fallback : declared
  // `\A` and `\z`, NOT `^` and `$`.
  //
  // ICU's `$` matches before a trailing line terminator, so `^...$` accepts
  // `"apkg\n"` -- and `\r\n`, `\r`, U+2028, U+2029 and NEL besides. Measured,
  // not assumed: this is the same anchor question that decided the Rust-side
  // analysis of `host_effect_symbol_re`, where `$` is end-of-haystack and the
  // newline is refused. The two engines answer it differently, so the Rust
  // result does not transfer.
  //
  // It matters here because the failure is silent rather than loud:
  // `UTType(filenameExtension: "apkg\n")` does not return nil, it returns a
  // DYNAMIC type that matches no file on disk. The panel then opens with a
  // filter that hides everything, and nothing anywhere reports why.
  //
  // `\A`/`\z` are absolute -- start and end of input, with no line-terminator
  // concession.
  guard let shape = try? NSRegularExpression(pattern: "\\A\\.?[A-Za-z0-9_-]{1,16}\\z") else {
    return fallback.map(bare)
  }
  let accepted = source.filter { candidate in
    let range = NSRange(candidate.startIndex..., in: candidate)
    return shape.firstMatch(in: candidate, range: range) != nil
  }
  .map(bare)
  // Everything was refused, so fall back rather than hand the panel an empty
  // list, which reads as "match nothing" and shows the person a dead dialog.
  return accepted.isEmpty ? fallback.map(bare) : accepted
}

private func applyFilter(_ panel: NSSavePanel, _ extensions: [String]) {
  if #available(macOS 11.0, *) {
    let types = extensions.compactMap { UTType(filenameExtension: $0) }
    if !types.isEmpty {
      panel.allowedContentTypes = types
      return
    }
  }
  // Older systems, or an extension the system has no type for: leave the panel
  // unfiltered rather than filtered to nothing.
}

// The bytes travel, not the path.
//
// The application built this package and sent it out in the payload, so the
// host only has to write it. That is the arrangement sandboxing forces: on
// macOS a process may open only what the person picked in the host's own
// dialog, so the file writing has to happen here, on this side of that dialog.
//
// Returns the outcome rather than answering, so that every path through the
// dialog funnels to exactly one `completeEffect` at the call site. The effect
// has been deferred by then, which takes it out of the fail sweep -- an early
// `return` that forgot to answer would wedge the app permanently, because the
// runtime gates snapshot and restore on nothing being pending.
@MainActor
private func runExport(_ payload: Any) -> [String: Any] {
  // Forced to a bare filename. A suggestion of `../.ssh/authorized_keys` would
  // otherwise open the dialog in a different directory with only the basename
  // visible. Nothing sends this key today; that is not a reason to trust it.
  var suggested = ((payload as? [String: Any])?["suggestedName"] as? String)
    .map { ($0 as NSString).lastPathComponent } ?? ""
  if suggested == "." || suggested == ".." {
    suggested = ""
  }

  let panel = NSSavePanel()
  panel.title = "Export Anki package"
  panel.nameFieldStringValue = suggested.isEmpty ? "engram.apkg" : suggested
  applyFilter(panel, allowedExtensions(payload, "extensions", fallback: ["apkg"]))

  guard panel.runModal() == .OK, var url = panel.url else {
    return cancelledOutcome()
  }
  if url.pathExtension.isEmpty {
    url = url.appendingPathExtension("apkg")
  }

  guard let encoded = (payload as? [String: Any])?["apkg"] as? String, !encoded.isEmpty else {
    return failedOutcome("the export carried no package")
  }
  // Strict decoding -- `options: []`, not `.ignoreUnknownCharacters`. Verified
  // rather than assumed: with `[]`, a bad character, embedded whitespace, a
  // newline and a truncated group all return nil, matching Qt's
  // `AbortOnBase64DecodingErrors`. Silently discarding a bad character would
  // write a corrupt `.apkg` that only fails later, inside Anki, where nothing
  // points back here.
  guard let decoded = Data(base64Encoded: encoded, options: []) else {
    return failedOutcome("the export package was not valid base64")
  }
  // Separate from the nil check, and not redundant with the `encoded.isEmpty`
  // guard above: `Data(base64Encoded:)` reports SUCCESS for input that decodes
  // to nothing, so without this a zero-byte package would be written out as a
  // real `.apkg` and reported as an ok outcome.
  if decoded.isEmpty {
    return failedOutcome("the export package was empty")
  }

  do {
    // Atomic, so a failure partway leaves no truncated `.apkg` behind looking
    // like a real one. This is what Qt has to reach for an explicit `close()`
    // and error re-check to get: `QFile` flushes on destruction and swallows
    // the failure, so a full disk would otherwise report success.
    try decoded.write(to: url, options: .atomic)
  } catch {
    return failedOutcome(error.localizedDescription)
  }
  return okOutcome([:])
}

@MainActor
private func runImport(_ payload: Any) -> [String: Any] {
  let panel = NSOpenPanel()
  panel.title = "Import Anki package"
  panel.allowsMultipleSelection = false
  panel.canChooseDirectories = false
  panel.canChooseFiles = true
  applyFilter(panel, allowedExtensions(payload, "accept", fallback: ["apkg", "colpkg"]))

  guard panel.runModal() == .OK, let url = panel.url else {
    return cancelledOutcome()
  }

  // Sized up before it is opened. A read on a fifo or a character device never
  // reaches EOF -- and reports no size -- so the regular-file test is doing
  // real work here, not restating the size test.
  let attributes = try? FileManager.default.attributesOfItem(atPath: url.path)
  guard (attributes?[.type] as? FileAttributeType) == .typeRegular else {
    return failedOutcome("that is not a regular file")
  }
  let size = (attributes?[.size] as? NSNumber)?.intValue ?? 0
  if size <= 0 {
    return failedOutcome("that file is empty")
  }
  if size > maxImportBytes {
    return failedOutcome("that package is too large to open")
  }

  let data: Data
  do {
    data = try Data(contentsOf: url)
  } catch {
    return failedOutcome(error.localizedDescription)
  }
  if data.isEmpty {
    return failedOutcome("that file is empty")
  }

  // The application decodes and merges. Reading the file is the host's whole
  // job here, for the same sandboxing reason the export writes it.
  return okOutcome(["apkg": data.base64EncodedString()])
}

#endif

/// Answer Engram's awaited file-dialog effects.
///
/// Internal, not `public`. The generated `MosaicRuntimeHost` is declared
/// `final class` with no access modifier, so a `public` function taking one is
/// rejected outright: "cannot be declared public because its parameter uses an
/// internal type". Internal is also what this wants to be -- the only caller is
/// the generated `App.swift`, in this same module.
func installEngramEffects(_ host: MosaicRuntimeHost) {
  // Weak, as the host's own documentation requires: a strong capture cycles
  // through this property, so `deinit` never runs, the Rust app handle is never
  // destroyed, and the final persist never happens.
  host.effectHandler = { [weak host] id, kind, payload, delivery in
    guard let host else { return }
    // Only the awaited kinds are answered. `openCard` arrives as a `Notify` and
    // is deliberately not handled: nothing is waiting on it, and answering an
    // effect the runtime is not awaiting is refused anyway.
    guard delivery.lowercased() == "await" else { return }

    #if os(macOS)
    switch kind {
    case "importAnki", "exportAnki":
      // Ownership first, inside the settle. `deferEffect` returns false for an
      // id the runtime is not actually waiting on, and in that case the right
      // move is to do nothing at all rather than open a dialog whose answer
      // would be refused.
      guard host.deferEffect(id) else { return }
      DispatchQueue.main.async { [weak host] in
        guard let host else { return }
        let outcome = kind == "importAnki" ? runImport(payload) : runExport(payload)
        // Exactly one answer, on every path through the dialog. The effect is
        // out of the fail sweep now, so not answering is not a degraded
        // outcome -- it is a permanent one.
        _ = host.completeEffect(id, outcome)
      }
    default:
      // An awaited kind this host does not know is left alone on purpose. The
      // host's own sweep then fails it with a reason the application shows,
      // which is a better outcome than this file inventing one -- and it is
      // what keeps a new effect kind from silently doing nothing while
      // appearing handled.
      break
    }
    #else
    // Not left to the sweep, because these are kinds this host DOES know and
    // simply cannot serve here: the modal panels are AppKit-only, and the iOS
    // document picker would need a presenting view controller this file has no
    // handle on. Saying so beats a generic "unanswered".
    switch kind {
    case "importAnki", "exportAnki":
      _ = host.completeEffect(id, failedOutcome("file dialogs are not available on this platform"))
    default:
      break
    }
    #endif
  }
}
