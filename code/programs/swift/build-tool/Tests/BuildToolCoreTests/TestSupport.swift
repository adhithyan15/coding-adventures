import BuildToolCore
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

func writeData(_ path: String, _ contents: Data) throws {
    let url = URL(fileURLWithPath: path)
    try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    try contents.write(to: url, options: .atomic)
}

struct SharedResolutionFixture: Decodable {
    struct Workspace: Decodable {
        let files: [File]
    }

    struct File: Decodable {
        let path: String
        let contentUTF8: String?
        let contentBase64: String?

        enum CodingKeys: String, CodingKey {
            case path
            case contentUTF8 = "content_utf8"
            case contentBase64 = "content_base64"
        }

        func data() throws -> Data {
            if let contentBase64 {
                guard let decoded = Data(base64Encoded: contentBase64) else {
                    throw CocoaError(.fileReadCorruptFile)
                }
                return decoded
            }
            return Data((contentUTF8 ?? "").utf8)
        }
    }

    struct Expected: Decodable {
        struct Result: Decodable {
            let edges: [[String]]?
        }

        struct Diagnostic: Decodable {
            struct Details: Decodable {
                let encoding: String
            }

            let code: String
            let path: String
            let package: String
            let details: Details
        }

        let outcome: String
        let result: Result
        let diagnostics: [Diagnostic]
    }

    let workspace: Workspace
    let expected: Expected
}

func loadSharedResolutionFixture(_ name: String) throws -> SharedResolutionFixture {
    let packageRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let fixtureURL = packageRoot
        .appendingPathComponent("../../../specs/fixtures/build-tool-v1/cases")
        .appendingPathComponent(name)
        .standardizedFileURL
    return try JSONDecoder().decode(SharedResolutionFixture.self, from: Data(contentsOf: fixtureURL))
}

func materializeSharedResolutionFixture(
    _ fixture: SharedResolutionFixture,
    label: String
) throws -> (root: String, packages: [BuildPackage]) {
    let root = try makeTempDirectory(label: label)
    try FileManager.default.createDirectory(
        atPath: (root as NSString).appendingPathComponent(".git"),
        withIntermediateDirectories: false
    )
    for file in fixture.workspace.files {
        try writeData((root as NSString).appendingPathComponent(file.path), file.data())
    }
    let codeRoot = (root as NSString).appendingPathComponent("code")
    return (root, Discovery.discoverPackages(root: codeRoot))
}

func buildToolExecutableURL() throws -> URL {
    #if os(Windows)
    let executableName = "build-tool.exe"
    #else
    let executableName = "build-tool"
    #endif

    let fileManager = FileManager.default
    let testExecutables = [
        Bundle.main.executableURL,
        URL(fileURLWithPath: CommandLine.arguments[0]),
    ].compactMap { $0 }

    for testExecutable in testExecutables {
        var directory = testExecutable.deletingLastPathComponent()
        for _ in 0 ..< 8 {
            let candidate = directory.appendingPathComponent(executableName)
            if fileManager.isExecutableFile(atPath: candidate.path) {
                return candidate
            }
            directory.deleteLastPathComponent()
        }
    }

    // SwiftPM can copy the test bundle away from the package build directory
    // on macOS. Fall back to the package-local scratch tree in that case.
    let packageRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    if let enumerator = fileManager.enumerator(
        at: packageRoot.appendingPathComponent(".build"),
        includingPropertiesForKeys: [.isRegularFileKey, .isExecutableKey]
    ) {
        for case let candidate as URL in enumerator where candidate.lastPathComponent == executableName {
            let values = try candidate.resourceValues(forKeys: [.isRegularFileKey, .isExecutableKey])
            if values.isRegularFile == true, values.isExecutable == true {
                return candidate
            }
        }
    }
    throw CocoaError(.fileNoSuchFile)
}
