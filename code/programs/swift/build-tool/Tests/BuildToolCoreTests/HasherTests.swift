@testable import BuildToolCore
import Foundation
import Testing
#if os(Windows)
import WinSDK
#endif

struct HasherTests {
    private struct SourceCollectionFixture: Decodable {
        struct Input: Decodable {
            struct Options: Decodable {
                struct Candidate: Decodable {
                    let path: String
                    let kind: String
                    let contentHex: String?

                    enum CodingKeys: String, CodingKey {
                        case path
                        case kind
                        case contentHex = "content_hex"
                    }
                }

                let mode: String
                let language: String
                let packageRoot: String
                let registrySHA256: String
                let declaredSrcs: [String]
                let candidates: [Candidate]

                enum CodingKeys: String, CodingKey {
                    case mode
                    case language
                    case packageRoot = "package_root"
                    case registrySHA256 = "registry_sha256"
                    case declaredSrcs = "declared_srcs"
                    case candidates
                }
            }

            let options: Options
        }

        struct Expected: Decodable {
            struct Result: Decodable {
                struct File: Decodable {
                    let path: String
                    let digest: String
                }

                let files: [File]
            }

            let result: Result
        }

        let input: Input
        let expected: Expected
    }

    private struct HashingFixture: Decodable {
        struct Workspace: Decodable {
            struct File: Decodable {
                let path: String
                let contentUTF8: String

                enum CodingKeys: String, CodingKey {
                    case path
                    case contentUTF8 = "content_utf8"
                }
            }

            let files: [File]
        }

        struct Input: Decodable {
            struct Options: Decodable {
                let package: String
                let includePaths: [String]

                enum CodingKeys: String, CodingKey {
                    case package
                    case includePaths = "include_paths"
                }
            }

            let options: Options
        }

        struct Expected: Decodable {
            struct Result: Decodable {
                let packageDigest: String

                enum CodingKeys: String, CodingKey {
                    case packageDigest = "package_digest"
                }
            }

            let result: Result
        }

        let workspace: Workspace
        let input: Input
        let expected: Expected
    }

    private func fixtureURL(_ name: String) -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("../../../specs/fixtures/build-tool-v1/cases")
            .appendingPathComponent(name)
            .standardizedFileURL
    }

    private func registryURL() -> URL {
        fixtureURL("unused")
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("language-source-input-registry.json")
            .standardizedFileURL
    }

    private func decodeHex(_ value: String) throws -> Data {
        guard value.count.isMultiple(of: 2) else {
            throw CocoaError(.fileReadCorruptFile)
        }
        var bytes: [UInt8] = []
        bytes.reserveCapacity(value.count / 2)
        var index = value.startIndex
        while index < value.endIndex {
            let end = value.index(index, offsetBy: 2)
            guard let byte = UInt8(value[index ..< end], radix: 16) else {
                throw CocoaError(.fileReadCorruptFile)
            }
            bytes.append(byte)
            index = end
        }
        return Data(bytes)
    }

    private func relativePaths(_ paths: [String], root: String) -> [String] {
        let normalizedRoot = root.replacingOccurrences(of: "\\", with: "/") + "/"
        return paths.map {
            $0.replacingOccurrences(of: "\\", with: "/")
                .replacingOccurrences(of: normalizedRoot, with: "")
        }
    }

    @Test(arguments: [
        "source-collection-extension.json",
        "source-collection-declared.json",
        "source-collection-registry-roles.json",
        "source-collection-engram-wasm-exact-inputs.json",
    ])
    func consumesNeutralSourceCollectionFixture(_ name: String) throws {
        let fixture = try JSONDecoder().decode(
            SourceCollectionFixture.self,
            from: Data(contentsOf: fixtureURL(name))
        )
        let root = try makeTempDirectory(label: "hasher_fixture")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            fixture.input.options.packageRoot
        )

        for candidate in fixture.input.options.candidates
            where candidate.kind == "file"
                && !candidate.path.hasPrefix("linked/")
                && !candidate.path.hasPrefix("reparse/") {
            try writeData(
                (packageRoot as NSString).appendingPathComponent(candidate.path),
                try decodeHex(candidate.contentHex ?? "")
            )
        }

        let declared = fixture.input.options.mode == "declared_sources"
        let package = BuildPackage(
            name: fixture.input.options.packageRoot,
            path: packageRoot,
            language: fixture.input.options.language,
            isStarlark: declared,
            declaredSrcs: fixture.input.options.declaredSrcs
        )
        let actual = relativePaths(
            try Hasher.collectSourceFiles(package, repositoryRoot: root),
            root: packageRoot
        )

        #expect(actual == fixture.expected.result.files.map(\.path))
    }

    @Test
    func productionRegistryExactlyEqualsCheckedNeutralRegistry() throws {
        let checkedData = try Data(contentsOf: registryURL())
        let checked = try JSONDecoder().decode(
            LanguageSourceInputRegistry.self,
            from: checkedData
        )
        let checkedObject = try #require(
            JSONSerialization.jsonObject(with: checkedData) as? NSDictionary
        )
        let productionObject = try #require(
            JSONSerialization.jsonObject(
                with: JSONEncoder().encode(Hasher.languageSourceInputRegistry)
            ) as? NSDictionary
        )
        let checkedText = try #require(String(data: checkedData, encoding: .utf8))
        let missingSelectorData = Data(
            checkedText.replacingOccurrences(
                of: "required_capabilities.json",
                with: "required_capabilities.missing"
            ).utf8
        )
        let missingSelector = try #require(
            JSONSerialization.jsonObject(with: missingSelectorData) as? NSDictionary
        )
        var extraFields = try #require(checkedObject as? [String: Any])
        extraFields["undeclared_selector_role"] = [".extra"]
        let extraSelector = extraFields as NSDictionary

        #expect(checkedObject.isEqual(productionObject))
        #expect(!missingSelector.isEqual(productionObject))
        #expect(!extraSelector.isEqual(productionObject))
        #expect(checked.languages.count == 23)
        #expect(Set(checked.languages.map(\.language)).count == 23)
        #expect(
            Hasher.languageSourceInputRegistryDigest
                == "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70"
        )
    }

    @Test
    func swiftRegistryKeepsExactScopesInBothCollectionModes() throws {
        let root = try makeTempDirectory(label: "hasher_swift_registry")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/swift/demo"
        )
        let candidates = [
            "BUILD",
            "required_capabilities.json",
            "nested/required_capabilities.json",
            "regen-embedded-grammars.sh",
            "nested/regen-embedded-grammars.sh",
            "Sources/Demo.swift",
            "Sources/Native/bridge.c",
            "Sources/Native/include/bridge.h",
            "Sources/Native/module.modulemap",
            "Other/bridge.h",
        ]
        for path in candidates {
            try writeFile(
                (packageRoot as NSString).appendingPathComponent(path),
                "input\n"
            )
        }

        let extensionPackage = BuildPackage(
            name: "swift/demo",
            path: packageRoot,
            language: "swift"
        )
        #expect(
            relativePaths(
                try Hasher.collectSourceFiles(extensionPackage),
                root: packageRoot
            ) == [
                "BUILD",
                "Sources/Demo.swift",
                "Sources/Native/bridge.c",
                "Sources/Native/include/bridge.h",
                "Sources/Native/module.modulemap",
                "regen-embedded-grammars.sh",
                "required_capabilities.json",
            ]
        )

        let declaredPackage = BuildPackage(
            name: "swift/demo",
            path: packageRoot,
            language: "swift",
            isStarlark: true,
            declaredSrcs: ["Sources/**/*.swift"]
        )
        #expect(
            relativePaths(
                try Hasher.collectSourceFiles(declaredPackage),
                root: packageRoot
            ) == [
                "BUILD",
                "Sources/Demo.swift",
                "regen-embedded-grammars.sh",
                "required_capabilities.json",
            ]
        )
    }

    @Test
    func unknownSourceLanguageFailsBeforeCollectingUniversalInputs() throws {
        let root = try makeTempDirectory(label: "hasher_unknown_registry")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/unknown/demo"
        )
        try writeFile(
            (packageRoot as NSString).appendingPathComponent("BUILD"),
            "echo inert\n"
        )
        let package = BuildPackage(
            name: "unknown/demo",
            path: packageRoot,
            language: "unknown"
        )

        #expect(throws: (any Error).self) {
            _ = try Hasher.collectSourceFiles(package)
        }
    }

    @Test
    func packageExactInputsUseRepositoryPathInsteadOfPackageName() throws {
        let root = try makeTempDirectory(label: "hasher_package_exact_scope")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/rust/engram-wasm-copy"
        )
        for path in [
            "BUILD",
            "js/engram-mosaic-host-wasm.mjs",
            "js/smoke.mjs",
            "pkg/engram_engine.wasm",
        ] {
            try writeFile(
                (packageRoot as NSString).appendingPathComponent(path),
                "input\n"
            )
        }
        let package = BuildPackage(
            name: "rust/engram-wasm",
            path: packageRoot,
            language: "rust"
        )

        #expect(
            relativePaths(
                try Hasher.collectSourceFiles(package, repositoryRoot: root),
                root: packageRoot
            ) == ["BUILD"]
        )
    }

    @Test
    func matchesNeutralHashingV1PackageDigest() throws {
        let fixture = try JSONDecoder().decode(
            HashingFixture.self,
            from: Data(contentsOf: fixtureURL("hashing-cache-missing.json"))
        )
        let root = try makeTempDirectory(label: "hasher_oracle")
        defer { try? FileManager.default.removeItem(atPath: root) }

        for file in fixture.workspace.files {
            try writeData(
                (root as NSString).appendingPathComponent(file.path),
                Data(file.contentUTF8.utf8)
            )
        }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/python/demo"
        )
        let declared = fixture.input.options.includePaths.map { path in
            String(path.dropFirst("code/packages/python/demo/".count))
        }
        let package = BuildPackage(
            name: fixture.input.options.package,
            path: packageRoot,
            language: "python",
            isStarlark: true,
            declaredSrcs: declared
        )

        #expect(
            try Hasher.hashPackage(package, repositoryRoot: root)
                == fixture.expected.result.packageDigest
        )
    }

    @Test
    func hashesEmptyPackagesAsSHA256OfEmptyBytes() throws {
        let root = try makeTempDirectory(label: "hasher_empty")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/swift/empty"
        )
        try FileManager.default.createDirectory(
            atPath: packageRoot,
            withIntermediateDirectories: true
        )
        let package = BuildPackage(
            name: "swift/empty",
            path: packageRoot,
            language: "swift"
        )

        #expect(
            try Hasher.hashPackage(package, repositoryRoot: root)
                == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
    }

    @Test
    func pathFramingMakesSameContentRenamesObservable() throws {
        let root = try makeTempDirectory(label: "hasher_rename")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/swift/rename"
        )
        let before = (packageRoot as NSString).appendingPathComponent("Sources/a.swift")
        let after = (packageRoot as NSString).appendingPathComponent("Sources/b.swift")
        try writeFile(before, "let answer = 42\n")
        let package = BuildPackage(
            name: "swift/rename",
            path: packageRoot,
            language: "swift"
        )
        let first = try Hasher.hashPackage(package, repositoryRoot: root)
        try FileManager.default.moveItem(atPath: before, toPath: after)

        #expect(try Hasher.hashPackage(package, repositoryRoot: root) != first)
    }

    @Test
    func hashesRawBytesAndUsesDeterministicUnicodePathOrdering() throws {
        let root = try makeTempDirectory(label: "hasher_bytes")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/swift/bytes"
        )
        try writeData(
            (packageRoot as NSString).appendingPathComponent("Sources/é.swift"),
            Data([0x00, 0x0D, 0x0A, 0xFF])
        )
        try writeData(
            (packageRoot as NSString).appendingPathComponent("Sources/z.swift"),
            Data("same\r\n".utf8)
        )
        let package = BuildPackage(
            name: "swift/bytes",
            path: packageRoot,
            language: "swift"
        )
        let first = try Hasher.hashPackage(package, repositoryRoot: root)

        #expect(first.count == 64)
        #expect(try Hasher.hashPackage(package, repositoryRoot: root) == first)
        try writeData(
            (packageRoot as NSString).appendingPathComponent("Sources/é.swift"),
            Data([0x00, 0x0A, 0xFF])
        )
        #expect(try Hasher.hashPackage(package, repositoryRoot: root) != first)
    }

    @Test
    func declaredModeRetainsRootOCamlMetadata() throws {
        let root = try makeTempDirectory(label: "hasher_ocaml_metadata")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/ocaml/demo"
        )
        try writeFile(
            (packageRoot as NSString).appendingPathComponent("demo.opam"),
            "opam-version: \"2.0\"\n"
        )
        try writeFile(
            (packageRoot as NSString).appendingPathComponent("dune-project"),
            "(lang dune 3.0)\n"
        )
        try writeFile(
            (packageRoot as NSString).appendingPathComponent("src/main.ml"),
            "let answer = 42\n"
        )
        let package = BuildPackage(
            name: "ocaml/demo",
            path: packageRoot,
            language: "ocaml",
            isStarlark: true,
            declaredSrcs: ["src/**/*.ml"]
        )

        #expect(
            relativePaths(try Hasher.collectSourceFiles(package), root: packageRoot)
                == ["demo.opam", "dune-project", "src/main.ml"]
        )
    }

    @Test
    func extensionModeCoversEveryEstablishedLanguageLane() throws {
        let root = try makeTempDirectory(label: "hasher_lane_registry")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let cases: [(String, [String])] = [
            ("csharp", ["Demo.csproj", "Program.cs", "global.json"]),
            ("dart", ["lib/demo.dart", "pubspec.yaml"]),
            ("elixir", ["lib/demo.ex", "mix.exs"]),
            ("fsharp", ["Demo.fsproj", "Program.fs", "global.json"]),
            ("go", ["go.mod", "main.go"]),
            ("haskell", ["Demo.cabal", "Main.hs", "cabal.project"]),
            ("java", ["build.gradle.kts", "src/Main.java"]),
            ("kotlin", ["settings.gradle.kts", "src/Main.kt"]),
            ("lua", ["demo.rockspec", "main.lua"]),
            ("perl", ["cpanfile", "lib/Demo.pm"]),
            ("python", ["demo.py", "pyproject.toml"]),
            ("ruby", ["demo.gemspec", "lib/demo.rb"]),
            ("rust", ["Cargo.toml", "src/lib.rs"]),
            ("swift", ["Package.swift", "Sources/Demo.swift"]),
            ("typescript", ["package.json", "src/demo.ts"]),
            ("ocaml", ["demo.opam", "dune-project", "src/demo.ml"]),
        ]

        for (language, expected) in cases {
            let packageRoot = (root as NSString).appendingPathComponent(
                "code/packages/\(language)/demo"
            )
            for path in expected {
                try writeFile(
                    (packageRoot as NSString).appendingPathComponent(path),
                    "source\n"
                )
            }
            try writeFile(
                (packageRoot as NSString).appendingPathComponent("ignored.txt"),
                "not a build input\n"
            )
            let package = BuildPackage(
                name: "\(language)/demo",
                path: packageRoot,
                language: language
            )
            #expect(
                relativePaths(try Hasher.collectSourceFiles(package), root: packageRoot)
                    == expected.sorted { $0.utf8.lexicographicallyPrecedes($1.utf8) },
                "missing portable inputs for \(language)"
            )
        }
    }

    @Test
    func declaredModeRetainsVariableRootManifestsOnly() throws {
        let root = try makeTempDirectory(label: "hasher_declared_manifests")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let cases: [(String, String)] = [
            ("ruby", "demo.gemspec"),
            ("lua", "demo.rockspec"),
            ("haskell", "demo.cabal"),
            ("ocaml", "demo.opam"),
            ("csharp", "Demo.csproj"),
            ("fsharp", "Demo.fsproj"),
            ("dotnet", "Demo.csproj"),
        ]

        for (language, manifest) in cases {
            let packageRoot = (root as NSString).appendingPathComponent(
                "code/packages/\(language)/demo"
            )
            try writeFile(
                (packageRoot as NSString).appendingPathComponent(manifest),
                "manifest\n"
            )
            try writeFile(
                (packageRoot as NSString).appendingPathComponent("nested/\(manifest)"),
                "nested manifest\n"
            )
            let package = BuildPackage(
                name: "\(language)/demo",
                path: packageRoot,
                language: language,
                isStarlark: true,
                declaredSrcs: ["src/**/*.never"]
            )
            #expect(
                relativePaths(try Hasher.collectSourceFiles(package), root: packageRoot)
                    == [manifest],
                "declared mode mishandled the root \(language) manifest"
            )
        }
    }

    @Test
    func emptyStarlarkSrcsFallsBackToExtensionCollection() throws {
        let root = try makeTempDirectory(label: "hasher_empty_declared_srcs")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/swift/demo"
        )
        try writeFile(
            (packageRoot as NSString).appendingPathComponent("Sources/Demo.swift"),
            "let answer = 42\n"
        )
        let package = BuildPackage(
            name: "swift/demo",
            path: packageRoot,
            language: "swift",
            isStarlark: true,
            declaredSrcs: []
        )

        #expect(
            relativePaths(try Hasher.collectSourceFiles(package), root: packageRoot)
                == ["Sources/Demo.swift"]
        )
    }

    @Test
    func missingPackageRootFailsClosedWithoutLeakingItsPath() throws {
        let root = try makeTempDirectory(label: "hasher_missing")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let missing = (root as NSString).appendingPathComponent(
            "code/packages/swift/missing"
        )
        let package = BuildPackage(
            name: "swift/missing\n::error::forged",
            path: missing,
            language: "swift"
        )

        do {
            _ = try Hasher.hashPackage(package, repositoryRoot: root)
            Issue.record("missing package root was hashed as an empty package")
        } catch {
            #expect(error.localizedDescription.hasPrefix("HASH_PACKAGE_FAILED: package="))
            #expect(!error.localizedDescription.contains(root))
            #expect(!error.localizedDescription.contains("\n::error::"))
        }
    }

    @Test
    func freshCLIReportsCheckedRedactedHashFailure() throws {
        let root = try makeTempDirectory(label: "hasher_cli_failure")
        defer { try? FileManager.default.removeItem(atPath: root) }
        try FileManager.default.createDirectory(
            atPath: (root as NSString).appendingPathComponent(".git"),
            withIntermediateDirectories: false
        )
        let planPath = (root as NSString).appendingPathComponent("plan.json")
        let hostileName = "swift/missing\n\u{0085}\u{2028}\u{202E}::error::forged"
        try PlanIO.writePlan(
            BuildPlan(
                schemaVersion: PlanIO.currentSchemaVersion,
                diffBase: "origin/main",
                force: true,
                affectedPackages: nil,
                packages: [
                    PackageEntry(
                        name: hostileName,
                        relPath: "code/packages/swift/missing",
                        language: "swift",
                        buildCommands: ["swift test"]
                    ),
                ],
                dependencyEdges: [],
                languagesNeeded: ["swift": true]
            ),
            to: planPath
        )

        let process = Process()
        process.executableURL = try buildToolExecutableURL()
        process.arguments = [
            "--root", root,
            "--plan-file", "plan.json",
            "--dry-run",
        ]
        let standardOutput = Pipe()
        let standardError = Pipe()
        process.standardOutput = standardOutput
        process.standardError = standardError
        try process.run()
        process.waitUntilExit()
        let stdout = String(
            decoding: standardOutput.fileHandleForReading.readDataToEndOfFile(),
            as: UTF8.self
        ).replacingOccurrences(of: "\r\n", with: "\n")
        let stderr = String(
            decoding: standardError.fileHandleForReading.readDataToEndOfFile(),
            as: UTF8.self
        ).replacingOccurrences(of: "\r\n", with: "\n")

        #expect(process.terminationStatus == 2)
        #expect(stdout == "Loaded plan: 1 packages\n")
        #expect(
            stderr
                == "HASH_PACKAGE_FAILED: package=\"swift/missing\\n\\u0085\\u2028\\u202e::error::forged\"\n"
        )
        #expect(!stderr.contains(root))
        #expect(!stderr.contains("\u{0085}"))
        #expect(!stderr.contains("\u{2028}"))
        #expect(!stderr.contains("\u{202E}"))
    }

    @Test
    func freshCLIRejectsPackageBelowAncestorLink() throws {
        let root = try makeTempDirectory(label: "hasher_ancestor_link")
        let outside = try makeTempDirectory(label: "hasher_ancestor_outside")
        let link = (root as NSString).appendingPathComponent("code/packages/linked")
        defer {
            try? FileManager.default.removeItem(atPath: link)
            try? FileManager.default.removeItem(atPath: root)
            try? FileManager.default.removeItem(atPath: outside)
        }
        try FileManager.default.createDirectory(
            atPath: (root as NSString).appendingPathComponent(".git"),
            withIntermediateDirectories: false
        )
        try writeFile(
            (outside as NSString).appendingPathComponent("demo/Main.swift"),
            "let outside = true\n"
        )
        try FileManager.default.createDirectory(
            atPath: (link as NSString).deletingLastPathComponent,
            withIntermediateDirectories: true
        )

        #if os(Windows)
        let linkProcess = Process()
        linkProcess.executableURL = URL(
            fileURLWithPath: ProcessInfo.processInfo.environment["ComSpec"]
                ?? "C:\\Windows\\System32\\cmd.exe"
        )
        linkProcess.arguments = [
            "/d", "/c", "mklink", "/J",
            link.replacingOccurrences(of: "/", with: "\\"),
            outside.replacingOccurrences(of: "/", with: "\\"),
        ]
        let linkOutput = Pipe()
        linkProcess.standardOutput = linkOutput
        linkProcess.standardError = linkOutput
        try linkProcess.run()
        linkProcess.waitUntilExit()
        #expect(linkProcess.terminationStatus == 0)
        guard linkProcess.terminationStatus == 0 else { return }
        #else
        try FileManager.default.createSymbolicLink(
            atPath: link,
            withDestinationPath: outside
        )
        #endif

        let planPath = (root as NSString).appendingPathComponent("plan.json")
        try PlanIO.writePlan(
            BuildPlan(
                schemaVersion: PlanIO.currentSchemaVersion,
                diffBase: "origin/main",
                force: true,
                affectedPackages: nil,
                packages: [
                    PackageEntry(
                        name: "swift/linked-demo",
                        relPath: "code/packages/linked/demo",
                        language: "swift",
                        buildCommands: ["swift test"]
                    ),
                ],
                dependencyEdges: [],
                languagesNeeded: ["swift": true]
            ),
            to: planPath
        )

        let process = Process()
        process.executableURL = try buildToolExecutableURL()
        process.arguments = [
            "--root", root,
            "--plan-file", "plan.json",
            "--dry-run",
        ]
        let standardOutput = Pipe()
        let standardError = Pipe()
        process.standardOutput = standardOutput
        process.standardError = standardError
        try process.run()
        process.waitUntilExit()
        let stdout = String(
            decoding: standardOutput.fileHandleForReading.readDataToEndOfFile(),
            as: UTF8.self
        ).replacingOccurrences(of: "\r\n", with: "\n")
        let stderr = String(
            decoding: standardError.fileHandleForReading.readDataToEndOfFile(),
            as: UTF8.self
        ).replacingOccurrences(of: "\r\n", with: "\n")

        #expect(process.terminationStatus == 2)
        #expect(stdout == "Loaded plan: 1 packages\n")
        #expect(stderr == "HASH_PACKAGE_FAILED: package=\"swift/linked-demo\"\n")
        #expect(!stderr.contains(outside))
    }

    @Test
    func rejectsSourceHardlinkedToOutsideRepository() throws {
        let root = try makeTempDirectory(label: "hasher_hardlink_repo")
        let outside = try makeTempDirectory(label: "hasher_hardlink_outside")
        defer {
            try? FileManager.default.removeItem(atPath: root)
            try? FileManager.default.removeItem(atPath: outside)
        }
        let outsideFile = (outside as NSString).appendingPathComponent("outside.swift")
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/swift/demo"
        )
        let linkedFile = (packageRoot as NSString).appendingPathComponent(
            "Sources/Outside.swift"
        )
        try writeFile(outsideFile, "let outside = true\n")
        try FileManager.default.createDirectory(
            atPath: (linkedFile as NSString).deletingLastPathComponent,
            withIntermediateDirectories: true
        )
        #if os(Windows)
        let linked = linkedFile.withCString(encodedAs: UTF16.self) { destination in
            outsideFile.withCString(encodedAs: UTF16.self) { existing in
                CreateHardLinkW(destination, existing, nil)
            }
        }
        #expect(linked)
        guard linked else { return }
        #else
        try FileManager.default.linkItem(atPath: outsideFile, toPath: linkedFile)
        #endif
        let package = BuildPackage(
            name: "swift/demo",
            path: packageRoot,
            language: "swift"
        )

        #expect(throws: (any Error).self) {
            _ = try Hasher.hashPackage(package, repositoryRoot: root)
        }
    }

    #if os(Windows)
    @Test
    func rejectsPackageRootWindowsJunction() throws {
        let root = try makeTempDirectory(label: "hasher_root_junction")
        let outside = try makeTempDirectory(label: "hasher_root_outside")
        let junction = (root as NSString).appendingPathComponent(
            "code/packages/swift/linked"
        )
        defer {
            try? FileManager.default.removeItem(atPath: junction)
            try? FileManager.default.removeItem(atPath: root)
            try? FileManager.default.removeItem(atPath: outside)
        }
        try writeFile(
            (outside as NSString).appendingPathComponent("external.swift"),
            "let external = true\n"
        )
        try FileManager.default.createDirectory(
            atPath: (junction as NSString).deletingLastPathComponent,
            withIntermediateDirectories: true
        )
        let process = Process()
        process.executableURL = URL(
            fileURLWithPath: ProcessInfo.processInfo.environment["ComSpec"]
                ?? "C:\\Windows\\System32\\cmd.exe"
        )
        process.arguments = [
            "/d", "/c", "mklink", "/J",
            junction.replacingOccurrences(of: "/", with: "\\"),
            outside.replacingOccurrences(of: "/", with: "\\"),
        ]
        let output = Pipe()
        process.standardOutput = output
        process.standardError = output
        try process.run()
        process.waitUntilExit()
        #expect(process.terminationStatus == 0)
        guard process.terminationStatus == 0 else { return }

        let package = BuildPackage(
            name: "swift/linked",
            path: junction,
            language: "swift"
        )
        #expect(throws: (any Error).self) {
            _ = try Hasher.hashPackage(package, repositoryRoot: root)
        }
    }
    #else
    @Test
    func rejectsPackageRootSymbolicLink() throws {
        let root = try makeTempDirectory(label: "hasher_root_link")
        let outside = try makeTempDirectory(label: "hasher_root_outside")
        let link = (root as NSString).appendingPathComponent(
            "code/packages/swift/linked"
        )
        defer {
            try? FileManager.default.removeItem(atPath: root)
            try? FileManager.default.removeItem(atPath: outside)
        }
        try writeFile(
            (outside as NSString).appendingPathComponent("external.swift"),
            "let external = true\n"
        )
        try FileManager.default.createDirectory(
            atPath: (link as NSString).deletingLastPathComponent,
            withIntermediateDirectories: true
        )
        try FileManager.default.createSymbolicLink(
            atPath: link,
            withDestinationPath: outside
        )

        let package = BuildPackage(
            name: "swift/linked",
            path: link,
            language: "swift"
        )
        #expect(throws: (any Error).self) {
            _ = try Hasher.hashPackage(package, repositoryRoot: root)
        }
    }
    #endif
}
