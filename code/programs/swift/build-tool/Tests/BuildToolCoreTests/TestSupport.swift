import Foundation

func makeTempDirectory(label: String = "build_tool_swift") throws -> String {
    let base = FileManager.default.temporaryDirectory
    let directory = base.appendingPathComponent("\(label)_\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    return directory.path
}

func writeFile(_ path: String, _ contents: String) throws {
    let url = URL(fileURLWithPath: path)
    try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    try contents.write(to: url, atomically: true, encoding: .utf8)
}
