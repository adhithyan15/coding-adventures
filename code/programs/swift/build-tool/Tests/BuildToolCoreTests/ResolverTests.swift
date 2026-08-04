import BuildToolCore
import Foundation
import Testing

struct ResolverTests {
    @Test
    func parseSwiftDepsReadsPackageManifestPaths() throws {
        let root = try makeTempDirectory(label: "resolver")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/swift/arc2d")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("Package.swift"),
            """
            import PackageDescription

            let package = Package(
                name: "arc2d",
                dependencies: [
                    .package(path: "../trig"),
                    .package(path: "../point2d"),
                ]
            )
            """
        )

        let package = BuildPackage(name: "swift/arc2d", path: packagePath, language: "swift")
        let deps = Resolver.parseSwiftDeps(
            package: package,
            knownNames: [
                "trig": "swift/trig",
                "point2d": "swift/point2d",
            ]
        )

        #expect(deps == ["swift/trig", "swift/point2d"])
    }

    @Test
    func resolveDependenciesCreatesSwiftEdge() throws {
        let root = try makeTempDirectory(label: "resolver_graph")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let trigPath = (root as NSString).appendingPathComponent("code/packages/swift/trig")
        let arcPath = (root as NSString).appendingPathComponent("code/packages/swift/arc2d")
        try writeFile((trigPath as NSString).appendingPathComponent("Package.swift"), "import PackageDescription\nlet package = Package(name: \"trig\")\n")
        try writeFile(
            (arcPath as NSString).appendingPathComponent("Package.swift"),
            """
            import PackageDescription
            let package = Package(
                name: "arc2d",
                dependencies: [.package(path: "../trig")]
            )
            """
        )

        let graph = try Resolver.resolveDependencies(
            packages: [
                BuildPackage(name: "swift/trig", path: trigPath, language: "swift"),
                BuildPackage(name: "swift/arc2d", path: arcPath, language: "swift"),
            ]
        )

        #expect(graph.successors(of: "swift/trig") == ["swift/arc2d"])
        #expect(graph.predecessors(of: "swift/arc2d") == ["swift/trig"])
    }

    @Test
    func luaResolutionConsumesSharedUTF8Fixtures() throws {
        for name in ["resolution-lua-utf8.json", "resolution-lua-invalid-utf8.json"] {
            let fixture = try loadSharedResolutionFixture(name)
            let materialized = try materializeSharedResolutionFixture(fixture, label: "resolver_fixture")
            defer { try? FileManager.default.removeItem(atPath: materialized.root) }

            if fixture.expected.outcome == "ok" {
                let graph = try Resolver.resolveDependencies(packages: materialized.packages)
                let actualEdges = graph.edges()
                    .map { [$0.0, $0.1] }
                    .sorted { $0.joined(separator: "\0") < $1.joined(separator: "\0") }
                let expectedEdges = (fixture.expected.result.edges ?? [])
                    .sorted { $0.joined(separator: "\0") < $1.joined(separator: "\0") }
                #expect(actualEdges == expectedEdges)
                continue
            }

            do {
                _ = try Resolver.resolveDependencies(packages: materialized.packages)
                Issue.record("invalid UTF-8 metadata must fail closed")
            } catch let error as MetadataEncodingError {
                let diagnostic = try #require(fixture.expected.diagnostics.first)
                #expect(error.code == diagnostic.code)
                #expect(error.package == diagnostic.package)
                #expect(error.manifest == diagnostic.path)
                #expect(error.encoding == diagnostic.details.encoding)
                #expect(
                    error.localizedDescription ==
                        "\(diagnostic.code): package=\(diagnostic.package) manifest=\(diagnostic.path) encoding=\(diagnostic.details.encoding)"
                )
                #expect(!error.localizedDescription.contains(materialized.root))
            } catch {
                Issue.record("expected MetadataEncodingError, got \(error)")
            }
        }
    }

    @Test
    func cliReturnsExitTwoForInvalidUTF8Metadata() throws {
        let fixture = try loadSharedResolutionFixture("resolution-lua-invalid-utf8.json")
        let materialized = try materializeSharedResolutionFixture(fixture, label: "resolver_cli")
        defer { try? FileManager.default.removeItem(atPath: materialized.root) }

        let exitCode = BuildTool.run(
            arguments: [
                "--root", materialized.root,
                "--force",
                "--dry-run",
                "--language", "lua",
            ]
        )

        #expect(exitCode == 2)
    }

    @Test
    func realCLIFailsClosedOnSharedInvalidUTF8Fixture() throws {
        let fixture = try loadSharedResolutionFixture("resolution-lua-invalid-utf8.json")
        let materialized = try materializeSharedResolutionFixture(fixture, label: "resolver_real_cli")
        defer { try? FileManager.default.removeItem(atPath: materialized.root) }

        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = try buildToolExecutableURL()
        process.arguments = [
            "--root", materialized.root,
            "--force",
            "--dry-run",
            "--language", "lua",
        ]
        process.standardOutput = stdout
        process.standardError = stderr
        try process.run()
        process.waitUntilExit()

        let diagnostic = try #require(fixture.expected.diagnostics.first)
        let expected =
            "\(diagnostic.code): package=\(diagnostic.package) manifest=\(diagnostic.path) encoding=\(diagnostic.details.encoding)\n"
        let actualStderr = String(decoding: stderr.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)

        #expect(process.terminationStatus == 2)
        #expect(actualStderr == expected)
        #expect(!actualStderr.contains(materialized.root))
    }
}
