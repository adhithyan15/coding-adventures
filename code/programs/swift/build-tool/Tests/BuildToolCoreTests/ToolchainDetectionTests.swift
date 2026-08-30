import BuildToolCore
import Foundation
import Testing

struct ToolchainDetectionTests {
  private static let fixtureNames = [
    "toolchain-detection-affected-only.json",
    "toolchain-detection-crlf-grammar.json",
    "toolchain-detection-declarations.json",
    "toolchain-detection-empty.json",
    "toolchain-detection-force-full.json",
    "toolchain-detection-null-all.json",
    "toolchain-detection-platform-darwin.json",
    "toolchain-detection-platform-linux.json",
    "toolchain-detection-platform-windows.json",
    "toolchain-detection-shared.json",
    "toolchain-detection-unsupported.json",
  ]

  @Test
  func consumesEveryLanguageNeutralToolchainFixture() throws {
    let fixtureRoot = try fixtureDirectory()
    let discoveredNames = try FileManager.default.contentsOfDirectory(
      at: fixtureRoot,
      includingPropertiesForKeys: nil
    )
    .map(\.lastPathComponent)
    .filter { $0.hasPrefix("toolchain-detection-") && $0.hasSuffix(".json") }
    .sorted()
    #expect(discoveredNames == Self.fixtureNames)

    for name in discoveredNames {
      let fixture = try JSONDecoder().decode(
        FixtureCase.self,
        from: Data(contentsOf: fixtureRoot.appendingPathComponent(name))
      )
      let options = fixture.input.options
      let actual = try ToolchainDetection.evaluateSnapshot(
        platform: options.platform,
        forceFull: options.forceFull,
        packages: options.packages.map(\.snapshot),
        scheduledPackages: options.scheduledPackages,
        forcedToolchains: options.forcedToolchains
      )

      #expect(actual.outcome == fixture.expected.outcome, Comment(rawValue: fixture.id))
      #expect(
        actual.toolchains == (fixture.expected.result.toolchains ?? [:]),
        Comment(rawValue: fixture.id)
      )
      #expect(
        actual.diagnostics == fixture.expected.diagnostics,
        Comment(rawValue: fixture.id)
      )
      if actual.outcome == "ok" {
        #expect(actual.toolchains.count == ToolchainDetection.canonicalToolchains.count)
        #expect(
          Set(actual.toolchains.keys) == Set(ToolchainDetection.canonicalToolchains)
        )
      }
    }
  }

  @Test
  func parserEnforcesByteAndLineLimitsBeforeSplitting() throws {
    let asciiAtLimit = String(repeating: "x", count: ToolchainDetection.maxBuildBytes)
    #expect(try ToolchainDetection.parseExtraToolchains(asciiAtLimit) == [])
    #expect(
      snapshotError {
        try ToolchainDetection.parseExtraToolchains(asciiAtLimit + "x")
      } == .perFileByteLimit
    )

    let multibyteAtLimit = String(
      repeating: "é",
      count: ToolchainDetection.maxBuildBytes / 2
    )
    #expect(multibyteAtLimit.utf8.count == ToolchainDetection.maxBuildBytes)
    #expect(try ToolchainDetection.parseExtraToolchains(multibyteAtLimit) == [])
    #expect(
      snapshotError {
        try ToolchainDetection.parseExtraToolchains(multibyteAtLimit + "é")
      } == .perFileByteLimit
    )

    let atLineLimit = String(
      repeating: "x\n",
      count: ToolchainDetection.maxBuildLines - 1
    )
    #expect(try ToolchainDetection.parseExtraToolchains(atLineLimit) == [])
    #expect(
      snapshotError {
        try ToolchainDetection.parseExtraToolchains(atLineLimit + "\n")
      } == .perFileLineLimit
    )
  }

  @Test
  func declarationGrammarIsByteExactAndStablyDeduplicated() throws {
    let content =
      " # needs-toolchain: python \r\n" + "# needs-toolchain:\tjava\t\n"
      + "# needs-toolchain: python\n" + "# needs-toolchain: ruby\r" + "# needs-toolchain: lua\r  \n"
      + "# needs-toolchain: perl\r\t\n" + "# needs-toolchain: swift\r\r\n"
      + "# needs-toolchain:python\n" + "# Needs-toolchain: kotlin\n" + "# needs-toolchain: zig\n"
      + "# needs-toolchain: kotlin trailing\n"
    #expect(
      try ToolchainDetection.parseExtraToolchains(content) == ["python", "java"]
    )
  }

  @Test
  func metersEveryFrontAndAggregateBeforeSelection() throws {
    let oversizedUnselectedFront = package(
      name: "rust/app",
      language: "rust",
      buildFiles: [
        "BUILD": "",
        "BUILD_windows": String(
          repeating: "x",
          count: ToolchainDetection.maxBuildBytes + 1
        ),
      ]
    )
    #expect(
      snapshotError {
        try ToolchainDetection.evaluateSnapshot(
          platform: "linux",
          forceFull: false,
          packages: [oversizedUnselectedFront],
          scheduledPackages: [],
          forcedToolchains: []
        )
      } == .perFileByteLimit
    )

    let exactAggregate = (0..<16).map { index in
      package(
        name: "rust/exact-\(index)",
        language: "rust",
        buildFiles: [
          "BUILD": String(
            repeating: "x",
            count: ToolchainDetection.maxBuildBytes
          )
        ]
      )
    }
    _ = try ToolchainDetection.evaluateSnapshot(
      platform: "linux",
      forceFull: false,
      packages: exactAggregate,
      scheduledPackages: [],
      forcedToolchains: []
    )

    let oversizedAggregate =
      exactAggregate + [
        package(
          name: "rust/over",
          language: "rust",
          buildFiles: [
            "BUILD": String(
              repeating: "x",
              count: ToolchainDetection.maxBuildBytes
            )
          ]
        )
      ]
    #expect(
      snapshotError {
        try ToolchainDetection.evaluateSnapshot(
          platform: "linux",
          forceFull: false,
          packages: oversizedAggregate,
          scheduledPackages: [],
          forcedToolchains: []
        )
      } == .aggregateByteLimit
    )
  }

  @Test
  func platformPresenceSchedulingAliasesAndFullModeAreClosed() throws {
    let emptyOverride = package(
      name: "rust/empty-override",
      language: "rust",
      buildFiles: [
        "BUILD": "# needs-toolchain: python\n",
        "BUILD_linux": "",
      ]
    )
    let emptyOverrideResult = try ToolchainDetection.evaluateSnapshot(
      platform: "linux",
      forceFull: false,
      packages: [emptyOverride],
      scheduledPackages: nil,
      forcedToolchains: []
    )
    #expect(emptyOverrideResult.toolchains["rust"] == true)
    #expect(emptyOverrideResult.toolchains["python"] == false)

    let snapshots = [
      package(name: "c/app", language: "c"),
      package(name: "cpp/app", language: "cpp"),
      package(name: "csharp/app", language: "csharp"),
      package(name: "fsharp/app", language: "fsharp"),
      package(name: "dotnet/app", language: "dotnet"),
      package(name: "wasm/app", language: "wasm"),
    ]
    let selected = try ToolchainDetection.evaluateSnapshot(
      platform: "windows",
      forceFull: false,
      packages: snapshots,
      scheduledPackages: nil,
      forcedToolchains: ["kotlin"]
    )
    #expect(selected.toolchains["cpp"] == true)
    #expect(selected.toolchains["dotnet"] == true)
    #expect(selected.toolchains["rust"] == true)
    #expect(selected.toolchains["kotlin"] == true)

    let none = try ToolchainDetection.evaluateSnapshot(
      platform: "windows",
      forceFull: false,
      packages: snapshots,
      scheduledPackages: [],
      forcedToolchains: []
    )
    #expect(none.toolchains.values.allSatisfy { !$0 })

    let full = try ToolchainDetection.evaluateSnapshot(
      platform: "windows",
      forceFull: true,
      packages: snapshots,
      scheduledPackages: nil,
      forcedToolchains: []
    )
    #expect(full.toolchains.count == ToolchainDetection.canonicalToolchains.count)
    #expect(full.toolchains.values.allSatisfy { $0 })
  }

  @Test
  func validatesClosedShapesAndStableErrorPrecedence() throws {
    #expect(
      snapshotError {
        try ToolchainDetection.evaluateSnapshot(
          platform: "solaris",
          forceFull: true,
          packages: [],
          scheduledPackages: nil,
          forcedToolchains: []
        )
      } == .unsupportedPlatform
    )
    #expect(
      snapshotError {
        try ToolchainDetection.evaluateSnapshot(
          platform: "solaris",
          forceFull: false,
          packages: [],
          scheduledPackages: [],
          forcedToolchains: []
        )
      } == .unsupportedPlatform
    )
    #expect(
      snapshotError {
        try ToolchainDetection.evaluateSnapshot(
          platform: "linux",
          forceFull: true,
          packages: [],
          scheduledPackages: [],
          forcedToolchains: []
        )
      } == .forceFullRequiresAllPackages
    )

    let unsupportedPackage = package(name: "zig/app", language: "zig")
    let selectedError = try ToolchainDetection.evaluateSnapshot(
      platform: "linux",
      forceFull: true,
      packages: [unsupportedPackage],
      scheduledPackages: nil,
      forcedToolchains: ["zig"]
    )
    #expect(selectedError.outcome == "error")
    #expect(selectedError.toolchains.isEmpty)
    #expect(
      selectedError.diagnostics == [
        ToolchainDiagnostic(
          code: "TOOLCHAIN_UNSUPPORTED",
          severity: "error",
          package: "zig/app"
        )
      ]
    )

    let forcedError = try ToolchainDetection.evaluateSnapshot(
      platform: "linux",
      forceFull: false,
      packages: [],
      scheduledPackages: nil,
      forcedToolchains: ["zig"]
    )
    #expect(
      forcedError.diagnostics == [
        ToolchainDiagnostic(
          code: "TOOLCHAIN_UNSUPPORTED",
          severity: "error",
          package: nil
        )
      ]
    )
  }

  @Test
  func returnsFreshCompleteResultsWithoutMutatingInputs() throws {
    let packages = [
      package(
        name: "rust/app",
        language: "rust",
        buildFiles: ["BUILD": "# needs-toolchain: python\n"]
      )
    ]
    let original = packages
    var first = try ToolchainDetection.evaluateSnapshot(
      platform: "linux",
      forceFull: false,
      packages: packages,
      scheduledPackages: nil,
      forcedToolchains: []
    )
    let second = try ToolchainDetection.evaluateSnapshot(
      platform: "linux",
      forceFull: false,
      packages: packages,
      scheduledPackages: nil,
      forcedToolchains: []
    )
    first.toolchains["rust"] = false

    #expect(second.toolchains["rust"] == true)
    #expect(packages == original)
    #expect(ToolchainDetection.canonicalToolchains == Self.fixtureRegistry)
    #expect(Set(second.toolchains.keys) == Set(Self.fixtureRegistry))
  }

  private static let fixtureRegistry = [
    "cpp",
    "dart",
    "dotnet",
    "elixir",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
  ]

  private func package(
    name: String,
    language: String,
    buildFiles: [String: String] = ["BUILD": ""]
  ) -> ToolchainPackageSnapshot {
    ToolchainPackageSnapshot(
      name: name,
      language: language,
      buildFiles: buildFiles
    )
  }

  private func snapshotError(
    _ operation: () throws -> some Any
  ) -> ToolchainSnapshotError? {
    do {
      _ = try operation()
      return nil
    } catch let error as ToolchainSnapshotError {
      return error
    } catch {
      return nil
    }
  }

  private func fixtureDirectory() throws -> URL {
    let packageRoot = URL(fileURLWithPath: #filePath)
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent()
    return
      packageRoot
      .appendingPathComponent("../../../specs/fixtures/build-tool-v1/cases")
      .standardizedFileURL
  }
}

private struct FixtureCase: Decodable {
  struct Input: Decodable {
    struct Options: Decodable {
      let platform: String
      let forceFull: Bool
      let packages: [FixturePackage]
      let scheduledPackages: [String]?
      let forcedToolchains: [String]

      enum CodingKeys: String, CodingKey {
        case platform
        case forceFull = "force_full"
        case packages
        case scheduledPackages = "scheduled_packages"
        case forcedToolchains = "forced_toolchains"
      }
    }

    let options: Options
  }

  struct Expected: Decodable {
    struct Result: Decodable {
      let toolchains: [String: Bool]?
    }

    let outcome: String
    let result: Result
    let diagnostics: [ToolchainDiagnostic]
  }

  let id: String
  let input: Input
  let expected: Expected
}

private struct FixturePackage: Decodable {
  let name: String
  let language: String
  let buildFiles: [String: String]

  enum CodingKeys: String, CodingKey {
    case name
    case language
    case buildFiles = "build_files"
  }

  var snapshot: ToolchainPackageSnapshot {
    ToolchainPackageSnapshot(
      name: name,
      language: language,
      buildFiles: buildFiles
    )
  }
}
