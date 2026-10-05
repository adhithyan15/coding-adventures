@testable import BuildToolCore
import Foundation
import Testing

// These adapters read checked-in JSON only in the test target. The production
// graph/diff operations receive already-materialized values and never see a URL.
private struct GraphCase: Decodable {
    struct Input: Decodable {
        struct Options: Decodable {
            let packages: [String]
            let edges: [[String]]
        }
        let options: Options
    }

    struct Expected: Decodable {
        struct Result: Decodable {
            let edges: [[String]]?
            let levels: [[String]]?
        }
        struct Diagnostic: Decodable { let code: String }
        let outcome: String
        let result: Result
        let diagnostics: [Diagnostic]
    }

    let id: String
    let input: Input
    let expected: Expected
}

private struct DiffCase: Decodable {
    struct Input: Decodable {
        struct Options: Decodable {
            struct Package: Decodable {
                let name: String
                let relPath: String
                let sourceMode: String
                let sourceGlobs: [String]?

                enum CodingKeys: String, CodingKey {
                    case name
                    case relPath = "rel_path"
                    case sourceMode = "source_mode"
                    case sourceGlobs = "source_globs"
                }
            }
            let packages: [Package]
            let edges: [[String]]
            let forcedPackages: [String]
            let unknownPathPolicy: String
            let boundarySHA256: String?

            enum CodingKeys: String, CodingKey {
                case packages, edges
                case forcedPackages = "forced_packages"
                case unknownPathPolicy = "unknown_path_policy"
                case boundarySHA256 = "boundary_sha256"
            }
        }
        let options: Options
        let changedPaths: [String]

        enum CodingKeys: String, CodingKey {
            case options
            case changedPaths = "changed_paths"
        }
    }

    struct Expected: Decodable {
        struct Result: Decodable {
            let changedPackages: [String]?
            let affectedPackages: [String]?
            let prerequisitePackages: [String]?

            enum CodingKeys: String, CodingKey {
                case changedPackages = "changed_packages"
                case affectedPackages = "affected_packages"
                case prerequisitePackages = "prerequisite_packages"
            }
        }
        struct Diagnostic: Decodable { let code: String }
        let outcome: String
        let result: Result
        let diagnostics: [Diagnostic]
    }

    let id: String
    let input: Input
    let expected: Expected
}

private let graphCaseNames: Set<String> = [
    "graph-canonical-edge-order.json", "graph-chain.json", "graph-cycle.json",
    "graph-diamond.json", "graph-empty.json", "graph-isolated.json",
    "graph-multiple-components.json", "graph-partial-cycle-no-output.json",
]

private let diffCaseNames: Set<String> = [
    "diff-selection-exact-build-fronts.json", "diff-selection-forced-package.json",
    "diff-selection-known-unmatched-near-build.json",
    "diff-selection-match-work-at-limit.json",
    "diff-selection-match-work-over-limit.json",
    "diff-selection-package-prefix.json", "diff-selection-repository-boundary.json",
    "diff-selection-shared-input-multiconsumer.json",
    "diff-selection-strict-glob-character-classes.json",
    "diff-selection-transitive.json", "diff-selection-unknown-all.json",
    "diff-selection-unknown-error.json",
]

private func sharedCaseDirectory() -> URL {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appendingPathComponent("../../../specs/fixtures/build-tool-v1/cases")
        .standardizedFileURL
}

private func exactCaseNames(prefix: String) throws -> Set<String> {
    Set(try FileManager.default.contentsOfDirectory(atPath: sharedCaseDirectory().path)
        .filter { $0.hasPrefix(prefix) && $0.hasSuffix(".json") })
}

private func loadCase<T: Decodable>(_ name: String, as type: T.Type) throws -> T {
    let data = try Data(contentsOf: sharedCaseDirectory().appendingPathComponent(name))
    return try JSONDecoder().decode(type, from: data)
}

private func loadBoundary() throws -> BuildToolGraphDiff.Boundary {
    let path = sharedCaseDirectory().deletingLastPathComponent()
        .appendingPathComponent("repository-source-input-boundary.json")
    return try JSONDecoder().decode(BuildToolGraphDiff.Boundary.self, from: Data(contentsOf: path))
}

struct GraphDiffConformanceTests {
    private func diffInput(
        packages: [BuildToolGraphDiff.DiffPackage] = [
            .init(name: "fixture/a", relPath: "code/packages/swift/a", sourceMode: "strict_globs", sourceGlobs: ["*.swift"]),
        ],
        edges: [[String]] = [],
        forced: [String] = [],
        policy: String = "error",
        boundary: String? = nil,
        boundaryValue: BuildToolGraphDiff.Boundary? = nil,
        paths: [String] = []
    ) -> BuildToolGraphDiff.DiffInput {
        .init(
            packages: packages, edges: edges, forcedPackages: forced,
            unknownPathPolicy: policy, boundarySHA256: boundary, boundary: boundaryValue,
            changedPaths: paths
        )
    }

    @Test
    func graphRejectsMalformedStructureAndCyclesWithoutPartialOutput() {
        let cases: [(BuildToolGraphDiff.GraphInput, String)] = [
            (.init(packages: ["bad"], edges: []), "GRAPH_PACKAGE_INVALID"),
            (.init(packages: ["fixture/a", "fixture/a"], edges: []), "GRAPH_PACKAGE_DUPLICATE"),
            (.init(packages: ["fixture/a"], edges: [["fixture/a"]]), "GRAPH_EDGE_INVALID"),
            (.init(packages: ["fixture/a"], edges: [["fixture/a", "fixture/b"]]), "GRAPH_EDGE_UNKNOWN"),
            (.init(packages: ["fixture/a"], edges: [["fixture/a", "fixture/a"]]), "GRAPH_EDGE_SELF"),
            (.init(packages: ["fixture/a", "fixture/b"], edges: [["fixture/a", "fixture/b"], ["fixture/a", "fixture/b"]]), "GRAPH_EDGE_DUPLICATE"),
            (.init(packages: ["fixture/a", "fixture/b"], edges: [["fixture/a", "fixture/b"], ["fixture/b", "fixture/a"]]), "GRAPH_CYCLE"),
        ]
        for (input, code) in cases {
            let result = BuildToolGraphDiff.evaluateGraph(input)
            #expect(result.diagnosticCodes == [code])
            #expect(result.edges.isEmpty)
            #expect(result.levels.isEmpty)
        }
        let overLimit = (0...4_096).map { "fixture/p\($0)" }
        #expect(BuildToolGraphDiff.evaluateGraph(.init(packages: overLimit, edges: [])).diagnosticCodes
                == ["GRAPH_PACKAGE_LIMIT_EXCEEDED"])
    }

    @Test
    func diffRejectsPortablePathGlobAndRootAliases() {
        let aliasPackages: [BuildToolGraphDiff.DiffPackage] = [
            .init(name: "fixture/a", relPath: "code/packages/swift/a", sourceMode: "package_prefix", sourceGlobs: []),
            .init(name: "fixture/b", relPath: "code/packages/swift/A/child", sourceMode: "package_prefix", sourceGlobs: []),
        ]
        let cases: [(BuildToolGraphDiff.DiffInput, String)] = [
            (diffInput(packages: aliasPackages), "DIFF_PATH_INVALID"),
            (diffInput(packages: [.init(name: "fixture/a", relPath: "../escape", sourceMode: "package_prefix", sourceGlobs: [])]), "DIFF_PATH_INVALID"),
            (diffInput(packages: [.init(name: "fixture/a", relPath: "code/packages/swift/a", sourceMode: "bad", sourceGlobs: [])]), "DIFF_SOURCE_MODE_INVALID"),
            (diffInput(packages: [.init(name: "fixture/a", relPath: "code/packages/swift/a", sourceMode: "strict_globs", sourceGlobs: ["[z-a].swift"])]), "DIFF_GLOB_INVALID"),
            (diffInput(paths: ["code/packages/swift/a/../secret"]), "DIFF_PATH_INVALID"),
            (diffInput(forced: ["fixture/b"]), "DIFF_FORCED_PACKAGE_UNKNOWN"),
            (diffInput(policy: "ignore"), "DIFF_POLICY_INVALID"),
        ]
        for (input, code) in cases {
            let result = BuildToolGraphDiff.evaluateDiffSelection(input)
            #expect(result.diagnosticCodes == [code])
            #expect(result.changedPackages.isEmpty)
            #expect(result.affectedPackages.isEmpty)
            #expect(result.prerequisitePackages.isEmpty)
        }
    }

    @Test
    func boundaryFailurePrecedesUnknownPathAndForcedSelection() {
        let result = BuildToolGraphDiff.evaluateDiffSelection(diffInput(
            forced: ["fixture/a"], boundary: String(repeating: "0", count: 64),
            paths: ["outside/unknown.txt"]
        ))
        #expect(result.diagnosticCodes == ["DIFF_BOUNDARY_DIGEST_MISMATCH"])
        #expect(result.changedPackages.isEmpty)
        #expect(result.affectedPackages.isEmpty)
        #expect(result.prerequisitePackages.isEmpty)
    }

    @Test
    func callerSuppliedBoundaryDigestAndExactFanout() throws {
        let boundary = BuildToolGraphDiff.Boundary(
            schemaVersion: 1,
            languageSourceInputRegistrySHA256: String(repeating: "a", count: 64),
            boundaries: [
                .init(
                    id: "test-shared-config", inputOrigin: "repository",
                    appliesTo: .init(
                        exactRoots: ["code/packages/swift/a"],
                        descendantRoots: [], excludedRoots: []
                    ),
                    inputs: [.init(path: "shared/config", role: "cross_package_exact")],
                    reason: "Test exact inert input", owner: "test"
                ),
            ]
        )
        let digest = try boundary.digest()
        let selected = BuildToolGraphDiff.evaluateDiffSelection(diffInput(
            boundary: digest, boundaryValue: boundary, paths: ["shared/config"]
        ))
        #expect(selected.diagnosticCodes.isEmpty)
        #expect(selected.changedPackages == ["fixture/a"])
        #expect(selected.affectedPackages == ["fixture/a"])

        let mismatch = BuildToolGraphDiff.evaluateDiffSelection(diffInput(
            boundary: String(repeating: "0", count: 64),
            boundaryValue: boundary, paths: ["shared/config"]
        ))
        #expect(mismatch.diagnosticCodes == ["DIFF_BOUNDARY_DIGEST_MISMATCH"])
        #expect(mismatch.changedPackages.isEmpty)
    }

    @Test
    func boundaryDescendantsExcludeOnlyExactRoots() throws {
        let boundary = BuildToolGraphDiff.Boundary(
            schemaVersion: 1,
            languageSourceInputRegistrySHA256: String(repeating: "a", count: 64),
            boundaries: [
                .init(
                    id: "test-descendant", inputOrigin: "repository",
                    appliesTo: .init(
                        exactRoots: [], descendantRoots: ["code/packages/swift"],
                        excludedRoots: ["code/packages/swift/b"]
                    ),
                    inputs: [.init(path: "shared/config", role: "cross_package_exact")],
                    reason: "Test descendant projection", owner: "test"
                ),
            ]
        )
        let packages: [BuildToolGraphDiff.DiffPackage] = [
            .init(name: "fixture/a", relPath: "code/packages/swift/a", sourceMode: "package_prefix", sourceGlobs: []),
            .init(name: "fixture/b", relPath: "code/packages/swift/b", sourceMode: "package_prefix", sourceGlobs: []),
            .init(name: "fixture/c", relPath: "code/packages/swift/c", sourceMode: "package_prefix", sourceGlobs: []),
        ]
        let result = BuildToolGraphDiff.evaluateDiffSelection(diffInput(
            packages: packages, boundary: try boundary.digest(),
            boundaryValue: boundary, paths: ["shared/config"]
        ))
        #expect(result.diagnosticCodes.isEmpty)
        #expect(result.changedPackages == ["fixture/a", "fixture/c"])
    }

    @Test
    func oversizedOrUnsafeBoundaryFailsBeforeEncodingAndSelection() {
        let rule = BuildToolGraphDiff.Boundary.Rule(
            id: "test-rule", inputOrigin: "repository",
            appliesTo: .init(exactRoots: ["code/packages/swift/a"], descendantRoots: [], excludedRoots: []),
            inputs: [.init(path: "shared/config", role: "cross_package_exact")],
            reason: "Test finite boundary", owner: "test"
        )
        let oversized = BuildToolGraphDiff.Boundary(
            schemaVersion: 1,
            languageSourceInputRegistrySHA256: String(repeating: "a", count: 64),
            boundaries: Array(repeating: rule, count: 257)
        )
        #expect((try? oversized.digest()) == nil)
        let result = BuildToolGraphDiff.evaluateDiffSelection(diffInput(
            boundary: String(repeating: "0", count: 64),
            boundaryValue: oversized, paths: ["shared/config"]
        ))
        #expect(result.diagnosticCodes == ["DIFF_BOUNDARY_DIGEST_MISMATCH"])
        #expect(result.changedPackages.isEmpty)
        let unsafe = BuildToolGraphDiff.Boundary(
            schemaVersion: 1,
            languageSourceInputRegistrySHA256: String(repeating: "a", count: 64),
            boundaries: [.init(
                id: "test-rule", inputOrigin: "repository", appliesTo: rule.appliesTo,
                inputs: [.init(path: "../secret", role: "cross_package_exact")],
                reason: "Test unsafe input", owner: "test"
            )]
        )
        #expect((try? unsafe.digest()) == nil)
    }

    @Test
    func allEightGraphCasesUseProductionCore() throws {
        #expect(try exactCaseNames(prefix: "graph-") == graphCaseNames)
        for name in graphCaseNames.sorted() {
            let fixture = try loadCase(name, as: GraphCase.self)
            let actual = BuildToolGraphDiff.evaluateGraph(.init(
                packages: fixture.input.options.packages,
                edges: fixture.input.options.edges
            ))
            #expect(actual.edges == (fixture.expected.result.edges ?? []), "\(fixture.id) edges")
            #expect(actual.levels == (fixture.expected.result.levels ?? []), "\(fixture.id) levels")
            #expect(actual.diagnosticCodes == fixture.expected.diagnostics.map(\.code), "\(fixture.id) diagnostics")
        }
    }

    @Test
    func allTwelveDiffCasesUseProductionCore() throws {
        #expect(try exactCaseNames(prefix: "diff-selection-") == diffCaseNames)
        for name in diffCaseNames.sorted() {
            let fixture = try loadCase(name, as: DiffCase.self)
            let options = fixture.input.options
            let actual = BuildToolGraphDiff.evaluateDiffSelection(.init(
                packages: options.packages.map {
                    .init(
                        name: $0.name,
                        relPath: $0.relPath,
                        sourceMode: $0.sourceMode,
                        sourceGlobs: $0.sourceGlobs ?? []
                    )
                },
                edges: options.edges,
                forcedPackages: options.forcedPackages,
                unknownPathPolicy: options.unknownPathPolicy,
                boundarySHA256: options.boundarySHA256,
                boundary: options.boundarySHA256 == nil ? nil : try loadBoundary(),
                changedPaths: fixture.input.changedPaths
            ))
            #expect(actual.changedPackages == (fixture.expected.result.changedPackages ?? []), "\(fixture.id) changed")
            #expect(actual.affectedPackages == (fixture.expected.result.affectedPackages ?? []), "\(fixture.id) affected")
            #expect(actual.prerequisitePackages == (fixture.expected.result.prerequisitePackages ?? []), "\(fixture.id) prerequisites")
            #expect(actual.diagnosticCodes == fixture.expected.diagnostics.map(\.code), "\(fixture.id) diagnostics")
        }
    }
}
