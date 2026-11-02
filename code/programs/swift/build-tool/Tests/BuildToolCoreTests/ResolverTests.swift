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

    /// Runs the shared case every build-tool front implements: inline and
    /// sub-table entries in `[dependencies]`, `[build-dependencies]` and
    /// their target forms are edges; dev-dependencies (every form),
    /// `[patch]`, registry entries, features and comments are not.
    @Test
    func rustResolutionConsumesSharedFieldAwareFixture() throws {
        let fixture = try loadSharedResolutionFixture("resolution-rust-field-aware.json")
        #expect(fixture.expected.outcome == "ok")
        let materialized = try materializeSharedResolutionFixture(fixture, label: "resolver_rust_fixture")
        defer { try? FileManager.default.removeItem(atPath: materialized.root) }

        let graph = try Resolver.resolveDependencies(packages: materialized.packages)
        let actualEdges = graph.edges()
            .map { [$0.0, $0.1] }
            .sorted { $0.joined(separator: "\0") < $1.joined(separator: "\0") }
        let expectedEdges = (fixture.expected.result.edges ?? [])
            .sorted { $0.joined(separator: "\0") < $1.joined(separator: "\0") }
        #expect(actualEdges == expectedEdges)
    }

    /// Pins each line rule of the Cargo contract in
    /// code/specs/build-tool-conformance.md, one manifest shape per case.
    /// The table is the Go reference's TestCargoLinkDependencyNames, case
    /// for case, so the two readers are held to the same shapes.
    @Test
    func cargoLinkDependencyNamesFollowsTheLineContract() {
        #expect(Self.cargoLinkCases.count == 46)
        for (name, manifest, want) in Self.cargoLinkCases {
            #expect(Resolver.cargoLinkDependencyNames(manifest) == want, "case \(name)")
        }
    }

    static let cargoLinkCases: [(String, String, [String])] = [
        ("top-level inline", "[dependencies]\na = { path = \"../a\" }\n", ["a"]),
        ("build-dependencies", "[build-dependencies]\na = { path = \"../a\" }\n", ["a"]),
        ("legacy build_dependencies", "[build_dependencies]\na = { path = \"../a\" }\n", ["a"]),
        ("quoted cfg target", "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\na = { path = \"../a\" }\n", ["a"]),
        ("bare triple target build", "[target.x86_64-pc-windows-msvc.build-dependencies]\na = { path = \"../a\" }\n", ["a"]),
        ("legacy target build", "[target.'cfg(unix)'.build_dependencies]\na = { path = \"../a\" }\n", ["a"]),
        ("spaced header segments", "[ target . 'cfg(unix)' . dependencies ]\na = { path = \"../a\" }\n", ["a"]),
        ("quoted table segment", "[\"dependencies\"]\na = { path = \"../a\" }\n", ["a"]),
        ("sub-table", "[dependencies.a]\nversion = \"1\"\npath = \"../a\"\n", ["a"]),
        ("sub-table rename before path", "[build-dependencies.x]\npackage = \"A\"\npath = '../a'\n", ["a"]),
        ("target sub-table, dotted quoted triple", "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\npath = \"../a\"\n", ["a"]),
        ("quoted sub-table key", "[dependencies.\"a\"]\npath = \"../a\"\n", ["a"]),
        ("sub-table without path", "[dependencies.a]\nversion = \"1\"\n", []),
        ("sub-table first path wins, ended by header", "[dependencies.a]\npath = \"../a\"\npath = \"../b\"\n[package]\nname = \"z\"\n", ["a"]),
        ("sub-table non-string path", "[dependencies.a]\npath = 1\n", []),
        ("sub-table empty path", "[dependencies.a]\npath = \"\"\n", []),
        ("sub-table capitalised Path key", "[dependencies.a]\nPath = \"../a\"\n", []),
        ("rename", "[dependencies]\nx = { package = \"Logic-Gates\", path = \"../g\" }\n", ["logic-gates"]),
        ("capitalised Package is not a rename", "[dependencies]\nx = { Package = \"y\", path = \"../g\" }\n", ["x"]),
        ("capitalised Path is not a path", "[dependencies]\nx = { Path = \"../g\" }\n", []),
        ("version plus path with trailing comment", "[dependencies]\na = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n", ["a"]),
        ("hash inside a string is not a comment", "[dependencies]\na = { path = \"../a#1\" }\n", ["a"]),
        ("quoted entry key", "[dependencies]\n\"a\" = { path = \"../a\" }\n", ["a"]),
        ("registry entry with path feature", "[dependencies]\na = { version = \"1\", features = [\"path\", \"x\"] }\nb = \"1\"\n", []),
        ("nested table path is not the entry's", "[dependencies]\na = { x = { path = \"../a\" } }\n", []),
        ("features array commas do not split fields", "[dependencies]\na = { features = [\"p\", \"q\"], path = \"../a\" }\n", ["a"]),
        ("empty path", "[dependencies]\na = { path = \"\" }\n", []),
        ("array continued onto later lines", "[dependencies]\na = { path = \"../a\", features = [\n    \"x\", # [not-a-header]\n] }\nb = { path = \"../b\" }\n", ["a", "b"]),
        ("continuation lines are not headers", "[dependencies]\na = { path = \"../a\", features = [\n[\"x\"]\n] }\nb = { path = \"../b\" }\n", ["a", "b"]),
        ("braces continued onto later lines", "[dependencies]\na = { version = \"1\",\n  path = \"../a\" }\n", ["a"]),
        ("file ends inside the value", "[dependencies]\na = { path = \"../a\", features = [\n  \"x\",\n", []),
        ("continued value must end with a brace", "[dependencies]\na = { path = \"../a\", features = [\n] } x\n", []),
        ("dotted key is out of scope", "[dependencies]\na.path = \"../a\"\n", []),
        ("commented-out entry", "[dependencies]\n# a = { path = \"../a\" }\n", []),
        ("dev-dependencies", "[dev-dependencies]\na = { path = \"../a\" }\n[dev_dependencies]\nb = { path = \"../b\" }\n", []),
        ("target dev-dependencies", "[target.'cfg(unix)'.dev-dependencies]\na = { path = \"../a\" }\n", []),
        ("dev-dependencies sub-table", "[dev-dependencies.a]\npath = \"../a\"\n", []),
        ("patch, replace, workspace", "[patch.crates-io]\na = { path = \"../a\" }\n[workspace.dependencies]\nb = { path = \"../b\" }\n[replace]\n\"c:1.0.0\" = { path = \"../c\" }\n", []),
        ("capitalised table name", "[Dependencies]\na = { path = \"../a\" }\n", []),
        ("features sub-table of a dependency", "[dependencies.a.features]\npath = \"../a\"\n", []),
        ("target table itself", "[target.'cfg(unix)']\na = { path = \"../a\" }\n", []),
        ("array of tables ends the table", "[dependencies]\n[[bin]]\na = { path = \"../a\" }\n", []),
        ("malformed header ends the table", "[dependencies]\n[a b]\nc = { path = \"../c\" }\n", []),
        ("unterminated quoted header ends the table", "[dependencies]\n[target.'cfg(unix).dependencies]\nc = { path = \"../c\" }\n", []),
        ("CRLF and BOM", "\u{FEFF}[dependencies]\r\na = { path = \"../a\" }\r\n", ["a"]),
        ("union across tables keeps duplicates for the caller", "[dependencies]\na = { path = \"../a\" }\n[build-dependencies]\na = { path = \"../a\" }\n", ["a", "a"]),
    ]

    /// A crate named by three link tables yields one edge, and an unknown
    /// path dependency (`serde`) is dropped.
    @Test
    func rustResolutionCollapsesDuplicateLinks() throws {
        let root = try makeTempDirectory(label: "resolver_rust_dedupe")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let appPath = (root as NSString).appendingPathComponent("code/packages/rust/app")
        let libPath = (root as NSString).appendingPathComponent("code/packages/rust/lib")
        try writeFile(
            (appPath as NSString).appendingPathComponent("Cargo.toml"),
            "[package]\nname = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n"
                + "serde = { path = \"../vendored-serde\" }\n\n[build-dependencies]\nlib = { path = \"../lib\" }\n\n"
                + "[target.'cfg(windows)'.dependencies.lib]\npath = \"../lib\"\n"
        )
        try writeFile((libPath as NSString).appendingPathComponent("Cargo.toml"), "[package]\nname = \"lib\"\n")

        let graph = try Resolver.resolveDependencies(
            packages: [
                BuildPackage(name: "rust/app", path: appPath, language: "rust"),
                BuildPackage(name: "rust/lib", path: libPath, language: "rust"),
            ]
        )

        #expect(graph.predecessors(of: "rust/app") == ["rust/lib"])
        #expect(graph.edges().count == 1)
    }

    /// A Rust crate is known by its directory name and its declared name; a
    /// WASM wrapper only by its declared name, so the wrapper's own
    /// `graph = { path = "../../rust/graph" }` names the Rust crate rather
    /// than drawing a self-edge.
    @Test
    func wasmWrapperResolvesItsRustCrateWithoutASelfEdge() throws {
        let root = try makeTempDirectory(label: "resolver_rust_wasm")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let rustPath = (root as NSString).appendingPathComponent("code/packages/rust/graph")
        let wasmPath = (root as NSString).appendingPathComponent("code/packages/wasm/graph")
        try writeFile((rustPath as NSString).appendingPathComponent("Cargo.toml"), "[package]\nname = \"coding-adventures-graph\"\n")
        try writeFile(
            (wasmPath as NSString).appendingPathComponent("Cargo.toml"),
            "[package]\nname = \"graph-wasm\"\n\n[dependencies]\ngraph = { path = \"../../rust/graph\" }\n"
        )
        let packages = [
            BuildPackage(name: "rust/graph", path: rustPath, language: "rust"),
            BuildPackage(name: "wasm/graph", path: wasmPath, language: "wasm"),
        ]

        let known = Resolver.buildKnownNames(packages: packages, language: "wasm")
        #expect(known["graph"] == "rust/graph")
        #expect(known["coding-adventures-graph"] == "rust/graph")
        #expect(known["graph-wasm"] == "wasm/graph")

        let graph = try Resolver.resolveDependencies(packages: packages)
        #expect(graph.edges().map { [$0.0, $0.1] } == [["rust/graph", "wasm/graph"]])
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
