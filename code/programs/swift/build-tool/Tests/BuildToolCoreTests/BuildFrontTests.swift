import Foundation
import Testing

#if os(Windows)
struct BuildFrontTests {
    @Test
    func windowsFrontRunsTestsWhenSwiftIsPresent() throws {
        let result = try runWindowsFront(fakeSwiftExit: 0)

        #expect(result.status == 0)
        #expect(!result.output.contains("not available"))
    }

    @Test
    func windowsFrontPropagatesNativeTestFailure() throws {
        let result = try runWindowsFront(fakeSwiftExit: 17)

        #expect(result.status == 17)
        #expect(!result.output.contains("not available"))
    }

    @Test
    func windowsFrontSkipsOnlyWhenSwiftIsAbsent() throws {
        let result = try runWindowsFront(fakeSwiftExit: nil)

        #expect(result.status == 0)
        #expect(
            result.output.trimmingCharacters(in: .whitespacesAndNewlines)
                == "Swift not available on this runner - skipping"
        )
    }

    private func runWindowsFront(fakeSwiftExit: Int?) throws -> (
        status: Int32,
        output: String
    ) {
        let packageRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let frontPath = packageRoot.appendingPathComponent("BUILD_windows")
        let commands = try String(contentsOf: frontPath, encoding: .utf8)
            .split(whereSeparator: \Character.isNewline)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        let command = try #require(commands.count == 1 ? commands[0] : nil)

        let temporaryDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("swift-build-front-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: temporaryDirectory,
            withIntermediateDirectories: true
        )
        defer { try? FileManager.default.removeItem(at: temporaryDirectory) }

        if let fakeSwiftExit {
            try "@echo off\r\n@exit /b \(fakeSwiftExit)\r\n".write(
                to: temporaryDirectory.appendingPathComponent("swift.bat"),
                atomically: true,
                encoding: .utf8
            )
        }

        var environment = ProcessInfo.processInfo.environment
        let comSpec = environmentValue(named: "ComSpec", in: environment)
            ?? "C:\\Windows\\System32\\cmd.exe"
        let systemRoot = environmentValue(named: "SystemRoot", in: environment)
            ?? "C:\\Windows"
        for key in Array(environment.keys)
        where key.caseInsensitiveCompare("Path") == .orderedSame
            || key.caseInsensitiveCompare("PATHEXT") == .orderedSame
        {
            environment.removeValue(forKey: key)
        }
        environment["Path"] = [
            temporaryDirectory.path,
            (systemRoot as NSString).appendingPathComponent("System32"),
        ].joined(separator: ";")
        environment["PATHEXT"] = ".BAT;.CMD;.EXE;.COM"

        let process = Process()
        process.executableURL = URL(fileURLWithPath: comSpec)
        process.arguments = ["/d", "/c", command]
        process.currentDirectoryURL = packageRoot
        process.environment = environment
        let output = Pipe()
        process.standardOutput = output
        process.standardError = output
        try process.run()
        process.waitUntilExit()

        return (
            process.terminationStatus,
            String(
                decoding: output.fileHandleForReading.readDataToEndOfFile(),
                as: UTF8.self
            )
        )
    }

    private func environmentValue(
        named wantedName: String,
        in environment: [String: String]
    ) -> String? {
        environment.first { key, _ in
            key.caseInsensitiveCompare(wantedName) == .orderedSame
        }?.value
    }
}
#endif
