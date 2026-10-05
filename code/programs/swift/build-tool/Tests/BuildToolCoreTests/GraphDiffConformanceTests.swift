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

struct GraphDiffConformanceTests {
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
                changedPaths: fixture.input.changedPaths
            ))
            #expect(actual.changedPackages == (fixture.expected.result.changedPackages ?? []), "\(fixture.id) changed")
            #expect(actual.affectedPackages == (fixture.expected.result.affectedPackages ?? []), "\(fixture.id) affected")
            #expect(actual.prerequisitePackages == (fixture.expected.result.prerequisitePackages ?? []), "\(fixture.id) prerequisites")
            #expect(actual.diagnosticCodes == fixture.expected.diagnostics.map(\.code), "\(fixture.id) diagnostics")
        }
    }
}
