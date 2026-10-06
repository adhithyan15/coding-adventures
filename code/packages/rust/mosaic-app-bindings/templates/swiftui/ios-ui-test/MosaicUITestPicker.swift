// The document picker an XCUITest answers (UI89 §4.4).
//
// iOS draws its document picker (`UIDocumentPickerViewController`) in
// another process, with a layout that changes from one iOS release to the
// next. A UI test should not drive it. So when a UI test launches the app with
// one fixed argument, this fake stands in for the picker instead. Every rule
// the platform library keeps (one file operation at a time, the size limit,
// answering exactly once, the app's own checks on the bytes) still runs,
// because only the picker is replaced.
//
// The fake has one file, inside the app's own container:
//
//   tmp/mosaic-ui-test-picker/document
//
//   create (a save)  writes the bytes the app is saving to that file
//   open             reads that file back; if it does not exist yet, the
//                    open answers as a cancel does
//
// So an export followed by an import is a round trip through the same bytes,
// and the test needs no fixture. The argument only selects the fake. It
// carries no path or URL, and nothing the test sends is read from it.
//
// How it stays out of everything else:
//
//   - The builder writes this file only for `mosaic-compile pkg
//     --ios-ui-test`, and adds it only to the generated Xcode project. The
//     Swift package (`Sources/App`) never compiles it.
//   - Even there, all of it is inside `#if MOSAIC_UI_TEST_PICKER`. The
//     builder defines that condition only in the app target's Debug
//     configuration. A Release build of the same project compiles nothing
//     from this file.
//   - `mosaicSystemPicker` asks for this fake under the same condition, so
//     a build without it has no code that reads the argument at all.

#if MOSAIC_UI_TEST_PICKER && os(iOS)
import Foundation

/// The launch argument a UI test passes to select the fake
/// (`app.launchArguments = ["-MosaicUITestPicker"]`).
let mosaicUITestPickerArgument = "-MosaicUITestPicker"

/// The fake picker when this launch asked for it, otherwise nil (the
/// system's picker).
func mosaicUITestPicker() -> MosaicDocumentPicker? {
  ProcessInfo.processInfo.arguments.contains(mosaicUITestPickerArgument)
    ? MosaicUITestPicker() : nil
}

/// The fake's one file, in the app's temporary directory.
private func mosaicUITestPickerFile() -> URL {
  FileManager.default.temporaryDirectory
    .appendingPathComponent("mosaic-ui-test-picker", isDirectory: true)
    .appendingPathComponent("document", isDirectory: false)
}

/// Answers on the main queue, later, as the real picker does. It never
/// answers before returning.
private struct MosaicUITestPicker: MosaicDocumentPicker {
  func open(_ accept: MosaicAccept, done: @escaping (MosaicOpenedDocument?) -> Void) throws {
    let file = mosaicUITestPickerFile()
    DispatchQueue.main.async {
      done(FileManager.default.fileExists(atPath: file.path) ? MosaicUITestDocument(file: file) : nil)
    }
  }

  func create(_ request: MosaicSaveRequest, done: @escaping (MosaicSaveTarget?) -> Void) throws {
    let file = mosaicUITestPickerFile()
    DispatchQueue.main.async {
      done(MosaicUITestTarget(file: file, name: request.suggestedName))
    }
  }
}

private struct MosaicUITestDocument: MosaicOpenedDocument {
  let file: URL
  var name: String { "document" }
  var mimeType: String? { nil }

  func read(limit: Int) throws -> Data? {
    let bytes = try Data(contentsOf: file)
    return bytes.count > limit ? nil : bytes
  }
}

private struct MosaicUITestTarget: MosaicSaveTarget {
  let file: URL
  let name: String

  func write(_ bytes: Data) throws {
    try FileManager.default.createDirectory(
      at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
    try bytes.write(to: file, options: .atomic)
  }
}
#endif
