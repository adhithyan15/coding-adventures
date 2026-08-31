import BuildToolCore
import Foundation
import Testing

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
                let declaredSrcs: [String]
                let candidates: [Candidate]

                enum CodingKeys: String, CodingKey {
                    case mode
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
    ])
    func consumesNeutralSourceCollectionFixture(_ name: String) throws {
        let fixture = try JSONDecoder().decode(
            SourceCollectionFixture.self,
            from: Data(contentsOf: fixtureURL(name))
        )
        let root = try makeTempDirectory(label: "hasher_fixture")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let packageRoot = (root as NSString).appendingPathComponent(
            "code/packages/ocaml/demo"
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
            name: "ocaml/demo",
            path: packageRoot,
            language: "ocaml",
            isStarlark: declared,
            declaredSrcs: fixture.input.options.declaredSrcs
        )
        let actual = relativePaths(
            try Hasher.collectSourceFiles(package),
            root: packageRoot
        )

        #expect(actual == fixture.expected.result.files.map(\.path))
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

        #expect(try Hasher.hashPackage(package) == fixture.expected.result.packageDigest)
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
            try Hasher.hashPackage(package)
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
        let first = try Hasher.hashPackage(package)
        try FileManager.default.moveItem(atPath: before, toPath: after)

        #expect(try Hasher.hashPackage(package) != first)
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
        let first = try Hasher.hashPackage(package)

        #expect(first.count == 64)
        #expect(try Hasher.hashPackage(package) == first)
        try writeData(
            (packageRoot as NSString).appendingPathComponent("Sources/é.swift"),
            Data([0x00, 0x0A, 0xFF])
        )
        #expect(try Hasher.hashPackage(package) != first)
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
            _ = try Hasher.hashPackage(package)
            Issue.record("missing package root was hashed as an empty package")
        } catch {
            #expect(error.localizedDescription.hasPrefix("HASH_PACKAGE_FAILED: package="))
            #expect(!error.localizedDescription.contains(root))
            #expect(!error.localizedDescription.contains("\n::error::"))
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
            _ = try Hasher.hashPackage(package)
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
            _ = try Hasher.hashPackage(package)
        }
    }
    #endif
}
