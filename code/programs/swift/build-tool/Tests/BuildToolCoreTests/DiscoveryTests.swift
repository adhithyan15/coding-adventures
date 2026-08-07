import BuildToolCore
import Foundation
import Testing

struct DiscoveryTests {
    @Test
    func inferLanguageIncludesSwift() {
        #expect(Discovery.inferLanguage(path: "/repo/code/packages/swift/trig") == "swift")
        #expect(Discovery.inferLanguage(path: "/repo/code/packages/custom/go") == "unknown")
        #expect(
            Discovery.inferPackageName(
                path: "/repo/code/programs/elixir/grammar_tools",
                language: "elixir"
            ) == "elixir/programs/grammar_tools"
        )
    }

    @Test
    func discoverPackagesFindsSwiftPackage() throws {
        let root = try makeTempDirectory(label: "discovery")
        defer { try? FileManager.default.removeItem(atPath: root) }

        try writeFile(
            (root as NSString).appendingPathComponent("packages/swift/trig/BUILD"),
            "swift test\n"
        )
        try writeFile(
            (root as NSString).appendingPathComponent("packages/python/logic-gates/BUILD"),
            "pytest\n"
        )
        try writeFile(
            (root as NSString).appendingPathComponent("packages/swift/trig/Sources/Trig/main.swift"),
            "print(\"hi\")\n"
        )

        let packages = try Discovery.discoverPackages(root: root)
        #expect(packages.map(\.name) == ["python/logic-gates", "swift/trig"])
        #expect(packages.last?.language == "swift")
        #expect(packages.last?.buildCommands == ["swift test"])
    }

    @Test
    func languageRegistryConsumesSharedFixture() throws {
        let fixture = try loadSharedDiscoveryFixture("discovery-language-registry.json")
        let root = try materializeSharedDiscoveryFixture(fixture, label: "discovery_registry")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packages = try Discovery.discoverPackages(
            root: (root as NSString).appendingPathComponent("code")
        )
        let actual = packages.map { package in
            [
                package.name,
                package.language,
                package.path
                    .replacingOccurrences(of: "\\", with: "/")
                    .replacingOccurrences(
                        of: root.replacingOccurrences(of: "\\", with: "/") + "/",
                        with: ""
                    ),
            ]
        }
        let expected = try #require(fixture.expected.result.packages).map {
            [$0.name, $0.language, $0.relPath]
        }

        #expect(actual == expected)
    }

    @Test
    func duplicateIdentityConsumesSharedFixture() throws {
        let fixture = try loadSharedDiscoveryFixture("discovery-duplicate-identity.json")
        let root = try materializeSharedDiscoveryFixture(fixture, label: "discovery_duplicate")
        defer { try? FileManager.default.removeItem(atPath: root) }

        do {
            _ = try Discovery.discoverPackages(
                root: (root as NSString).appendingPathComponent("code")
            )
            Issue.record("duplicate qualified identities must fail closed")
        } catch let error as DuplicatePackageIdentityError {
            let diagnostic = try #require(fixture.expected.diagnostics.first)
            #expect(error.code == diagnostic.code)
            #expect(error.package == diagnostic.package)
            #expect(error.paths == diagnostic.details.paths)
            #expect(error.paths.first == diagnostic.path)
            let expected =
                "\(diagnostic.code): package=\(diagnostic.package) paths=\(diagnostic.details.paths.joined(separator: ","))"
            #expect(error.localizedDescription == expected)
            #expect(!error.localizedDescription.contains(root))
        }
    }

    @Test
    func realCLIFailsClosedOnSharedDuplicateIdentityFixture() throws {
        let fixture = try loadSharedDiscoveryFixture("discovery-duplicate-identity.json")
        let root = try materializeSharedDiscoveryFixture(fixture, label: "discovery_duplicate_cli")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = try buildToolExecutableURL()
        process.arguments = ["--root", root, "--force", "--dry-run"]
        process.standardOutput = stdout
        process.standardError = stderr
        try process.run()
        process.waitUntilExit()

        let diagnostic = try #require(fixture.expected.diagnostics.first)
        let expected =
            "\(diagnostic.code): package=\(diagnostic.package) paths=\(diagnostic.details.paths.joined(separator: ","))\n"
        let actualStderr = String(
            decoding: stderr.fileHandleForReading.readDataToEndOfFile(),
            as: UTF8.self
        )

        #expect(process.terminationStatus == 2)
        #expect(actualStderr == expected)
        #expect(!actualStderr.contains(root))
        #expect(stdout.fileHandleForReading.readDataToEndOfFile().isEmpty)
    }
}
