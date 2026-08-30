/// Pure, bounded extra-CI toolchain decisions over caller-supplied snapshots.
///
/// BUILD fronts are inert UTF-8 data. This adapter deliberately has no
/// filesystem, environment, process, Git, clock, randomness, credential, or
/// network authority.
public struct ToolchainPackageSnapshot: Equatable, Sendable {
  public let name: String
  public let language: String
  public let buildFiles: [String: String]

  public init(name: String, language: String, buildFiles: [String: String]) {
    self.name = name
    self.language = language
    self.buildFiles = buildFiles
  }
}

public struct ToolchainDiagnostic: Codable, Equatable, Sendable {
  public let code: String
  public let severity: String
  public let package: String?

  public init(code: String, severity: String, package: String?) {
    self.code = code
    self.severity = severity
    self.package = package
  }
}

public struct ToolchainEvaluation: Equatable, Sendable {
  public let outcome: String
  public var toolchains: [String: Bool]
  public let diagnostics: [ToolchainDiagnostic]

  public init(
    outcome: String,
    toolchains: [String: Bool],
    diagnostics: [ToolchainDiagnostic]
  ) {
    self.outcome = outcome
    self.toolchains = toolchains
    self.diagnostics = diagnostics
  }
}

public enum ToolchainSnapshotError: Error, Equatable, Sendable {
  case unsupportedPlatform
  case perFileByteLimit
  case perFileLineLimit
  case aggregateByteLimit
  case forceFullRequiresAllPackages
}

public enum ToolchainDetection {
  public static let maxBuildBytes = 65_536
  public static let maxBuildLines = 4_096
  public static let maxAggregateBuildBytes = 1_048_576

  public static let canonicalToolchains = [
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

  private static let declarationPrefix = Array("# needs-toolchain:".utf8)

  /// Parses exact inert declarations from one already supplied BUILD front.
  ///
  /// This public helper retains the same per-file bounds as the top-level
  /// evaluator so callers cannot bypass metering by invoking it directly.
  public static func parseExtraToolchains(
    _ content: String
  ) throws -> [String] {
    _ = try meterFront(content)

    let bytes = Array(content.utf8)
    var seen = Set<String>()
    var declarations: [String] = []
    var lineStart = 0

    while lineStart <= bytes.count {
      var lineEnd = lineStart
      while lineEnd < bytes.count && bytes[lineEnd] != 0x0A {
        lineEnd += 1
      }

      let isLFTerminated = lineEnd < bytes.count
      var contentEnd = lineEnd
      if isLFTerminated && contentEnd > lineStart && bytes[contentEnd - 1] == 0x0D {
        contentEnd -= 1
      }

      if let name = declarationName(in: bytes[lineStart..<contentEnd]),
        seen.insert(name).inserted
      {
        declarations.append(name)
      }

      if !isLFTerminated {
        break
      }
      lineStart = lineEnd + 1
    }

    return declarations
  }

  /// Evaluates a complete caller-owned toolchain snapshot without host access.
  public static func evaluateSnapshot(
    platform: String,
    forceFull: Bool,
    packages: [ToolchainPackageSnapshot],
    scheduledPackages: [String]?,
    forcedToolchains: [String]
  ) throws -> ToolchainEvaluation {
    let precedence = try frontPrecedence(for: platform)

    var aggregateBytes = 0
    for package in packages {
      for key in package.buildFiles.keys.sorted() {
        guard let content = package.buildFiles[key] else {
          continue
        }
        let bytes = try meterFront(content)
        let (nextTotal, overflow) = aggregateBytes.addingReportingOverflow(bytes)
        guard !overflow, nextTotal <= maxAggregateBuildBytes else {
          throw ToolchainSnapshotError.aggregateByteLimit
        }
        aggregateBytes = nextTotal
      }
    }

    if forceFull && scheduledPackages != nil {
      throw ToolchainSnapshotError.forceFullRequiresAllPackages
    }

    let scheduled = scheduledPackages.map(Set.init)
    var selected: [(ToolchainPackageSnapshot, String)] = []
    for package in packages where scheduled == nil || scheduled!.contains(package.name) {
      guard let toolchain = toolchain(for: package.language) else {
        return unsupported(package: package.name)
      }
      selected.append((package, toolchain))
    }

    for forced in forcedToolchains where !isCanonicalToolchain(forced) {
      return unsupported(package: nil)
    }

    var toolchains = freshToolchainMap(enabled: forceFull)
    if !forceFull {
      for (package, languageToolchain) in selected {
        toolchains[languageToolchain] = true
        let front = selectedFront(
          from: package.buildFiles,
          precedence: precedence
        )
        for extra in try parseExtraToolchains(front) {
          toolchains[extra] = true
        }
      }
    }
    for forced in forcedToolchains {
      toolchains[forced] = true
    }

    return ToolchainEvaluation(
      outcome: "ok",
      toolchains: toolchains,
      diagnostics: []
    )
  }

  private static func declarationName(
    in rawLine: ArraySlice<UInt8>
  ) -> String? {
    var start = rawLine.startIndex
    var end = rawLine.endIndex
    while start < end && isSpaceOrTab(rawLine[start]) {
      start += 1
    }
    while start < end && isSpaceOrTab(rawLine[end - 1]) {
      end -= 1
    }

    let line = rawLine[start..<end]
    guard line.count > declarationPrefix.count,
      line.starts(with: declarationPrefix)
    else {
      return nil
    }

    var nameStart = line.index(line.startIndex, offsetBy: declarationPrefix.count)
    guard nameStart < line.endIndex, isSpaceOrTab(line[nameStart]) else {
      return nil
    }
    while nameStart < line.endIndex && isSpaceOrTab(line[nameStart]) {
      nameStart += 1
    }
    var nameEnd = line.endIndex
    while nameStart < nameEnd && isSpaceOrTab(line[nameEnd - 1]) {
      nameEnd -= 1
    }

    let name = String(decoding: line[nameStart..<nameEnd], as: UTF8.self)
    return isCanonicalToolchain(name) ? name : nil
  }

  private static func meterFront(_ content: String) throws -> Int {
    let bytes = content.utf8
    let byteCount = bytes.count
    guard byteCount <= maxBuildBytes else {
      throw ToolchainSnapshotError.perFileByteLimit
    }

    var logicalLines = 1
    for byte in bytes where byte == 0x0A {
      logicalLines += 1
      guard logicalLines <= maxBuildLines else {
        throw ToolchainSnapshotError.perFileLineLimit
      }
    }
    return byteCount
  }

  private static func frontPrecedence(
    for platform: String
  ) throws -> [String] {
    switch platform {
    case "windows":
      return ["BUILD_windows", "BUILD"]
    case "darwin":
      return ["BUILD_mac", "BUILD_mac_and_linux", "BUILD"]
    case "linux":
      return ["BUILD_linux", "BUILD_mac_and_linux", "BUILD"]
    default:
      throw ToolchainSnapshotError.unsupportedPlatform
    }
  }

  private static func selectedFront(
    from buildFiles: [String: String],
    precedence: [String]
  ) -> String {
    for front in precedence {
      if let content = buildFiles[front] {
        return content
      }
    }
    return ""
  }

  private static func toolchain(for language: String) -> String? {
    switch language {
    case "c", "cpp":
      return "cpp"
    case "csharp", "fsharp", "dotnet":
      return "dotnet"
    case "wasm":
      return "rust"
    default:
      return isCanonicalToolchain(language) ? language : nil
    }
  }

  private static func isCanonicalToolchain(_ name: String) -> Bool {
    canonicalToolchains.contains(name)
  }

  private static func isSpaceOrTab(_ byte: UInt8) -> Bool {
    byte == 0x20 || byte == 0x09
  }

  private static func freshToolchainMap(enabled: Bool) -> [String: Bool] {
    Dictionary(uniqueKeysWithValues: canonicalToolchains.map { ($0, enabled) })
  }

  private static func unsupported(package: String?) -> ToolchainEvaluation {
    ToolchainEvaluation(
      outcome: "error",
      toolchains: [:],
      diagnostics: [
        ToolchainDiagnostic(
          code: "TOOLCHAIN_UNSUPPORTED",
          severity: "error",
          package: package
        )
      ]
    )
  }
}
