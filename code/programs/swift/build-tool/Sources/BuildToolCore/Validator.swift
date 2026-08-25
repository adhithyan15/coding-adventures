import Foundation

public enum TrackedArtifactEntryKind: String, Codable, Sendable {
    case regular
    case symlink
    case reparse
}

public struct TrackedArtifactEntry: Codable, Equatable, Sendable {
    public let ordinal: Int
    public let path: String
    public let entryKind: TrackedArtifactEntryKind

    public init(ordinal: Int, path: String, entryKind: TrackedArtifactEntryKind) {
        self.ordinal = ordinal
        self.path = path
        self.entryKind = entryKind
    }

    enum CodingKeys: String, CodingKey {
        case ordinal
        case path
        case entryKind = "entry_kind"
    }
}

public struct TrackedArtifactDiagnosticDetails: Codable, Equatable, Sendable {
    public let ordinal: Int
    public let entryKind: TrackedArtifactEntryKind
    public let problem: String?

    public init(ordinal: Int, entryKind: TrackedArtifactEntryKind, problem: String?) {
        self.ordinal = ordinal
        self.entryKind = entryKind
        self.problem = problem
    }

    enum CodingKeys: String, CodingKey {
        case ordinal
        case entryKind = "entry_kind"
        case problem
    }
}

public struct TrackedArtifactDiagnostic: Codable, Equatable, Sendable {
    public let code: String
    public let severity: String
    public let path: String
    public let details: TrackedArtifactDiagnosticDetails

    public init(
        code: String,
        severity: String,
        path: String,
        details: TrackedArtifactDiagnosticDetails
    ) {
        self.code = code
        self.severity = severity
        self.path = path
        self.details = details
    }
}

public struct TrackedArtifactValidationError: Error, Equatable, Sendable {
    public let message: String

    public init(message: String) {
        self.message = message
    }
}

public enum Validator {
    public static let trackedArtifactUnicodeVersion = TrackedArtifactUnicode17.unicodeVersion

    private static let trackedArtifactComponentIdentity = "node_modules"
    private static let trackedArtifactRedactedPath = "repository"
    private static let windowsReservedBasenames: Set<String> = Set(
        ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"] +
            ["COM", "LPT"].flatMap { prefix in
                (1 ... 9).map { "\(prefix)\($0)" } +
                    ["¹", "²", "³"].map { "\(prefix)\($0)" }
            }
    )

    public static let ciManagedToolchainLanguages: Set<String> = [
        "python",
        "ruby",
        "typescript",
        "rust",
        "elixir",
        "lua",
        "perl",
        "java",
        "kotlin",
        "haskell",
    ]

    /// Validate an already bounded, inert snapshot of tracked paths.
    ///
    /// Snapshot construction deliberately lives outside this pure adapter. It
    /// does not enumerate a checkout, follow links, invoke Git, open paths,
    /// read the environment, launch processes, or access the network.
    public static func validateTrackedArtifactSnapshot(
        unicodeVersion: String = trackedArtifactUnicodeVersion,
        entries: [TrackedArtifactEntry]
    ) throws -> [TrackedArtifactDiagnostic] {
        guard unicodeVersion == trackedArtifactUnicodeVersion else {
            throw TrackedArtifactValidationError(
                message: "tracked artifact Unicode version must be \(trackedArtifactUnicodeVersion)"
            )
        }

        return entries.compactMap { entry in
            switch normalizeTrackedArtifactPath(entry.path) {
            case let .failure(problem):
                return TrackedArtifactDiagnostic(
                    code: "TRACKED_ARTIFACT_PATH_INVALID",
                    severity: "error",
                    path: trackedArtifactRedactedPath,
                    details: TrackedArtifactDiagnosticDetails(
                        ordinal: entry.ordinal,
                        entryKind: entry.entryKind,
                        problem: problem.rawValue
                    )
                )
            case let .success(normalizedPath):
                let forbidden = normalizedPath.split(
                    separator: "/",
                    omittingEmptySubsequences: false
                ).contains { component in
                    unicodeScalarEqual(
                        TrackedArtifactUnicode17.nfkcCasefold(String(component)),
                        trackedArtifactComponentIdentity
                    )
                }
                guard forbidden else { return nil }
                return TrackedArtifactDiagnostic(
                    code: "TRACKED_ARTIFACT_FORBIDDEN",
                    severity: "error",
                    path: normalizedPath,
                    details: TrackedArtifactDiagnosticDetails(
                        ordinal: entry.ordinal,
                        entryKind: entry.entryKind,
                        problem: nil
                    )
                )
            }
        }.sorted(by: trackedArtifactDiagnosticLessThan)
    }

    private enum TrackedArtifactPathProblem: String, Error {
        case empty = "EMPTY"
        case tooLong = "TOO_LONG"
        case nonNFC = "NON_NFC"
        case absolute = "ABSOLUTE"
        case driveQualified = "DRIVE_QUALIFIED"
        case emptySegment = "EMPTY_SEGMENT"
        case dotSegment = "DOT_SEGMENT"
        case trailingDotOrSpace = "TRAILING_DOT_OR_SPACE"
        case unsafeCharacter = "UNSAFE_CHARACTER"
        case reservedBasename = "RESERVED_BASENAME"
    }

    private static func normalizeTrackedArtifactPath(
        _ rawPath: String
    ) -> Result<String, TrackedArtifactPathProblem> {
        // Separator replacement is intentionally lexical. Host path APIs can
        // collapse exactly the empty, dot, and traversal segments we reject.
        let normalizedPath = rawPath.replacingOccurrences(of: "\\", with: "/")
        guard !normalizedPath.isEmpty else { return .failure(.empty) }
        guard normalizedPath.unicodeScalars.count <= 512 else { return .failure(.tooLong) }
        guard unicodeScalarEqual(
            TrackedArtifactUnicode17.nfc(normalizedPath),
            normalizedPath
        ) else {
            return .failure(.nonNFC)
        }
        guard normalizedPath.first != "/" else { return .failure(.absolute) }

        let scalars = Array(normalizedPath.unicodeScalars)
        if scalars.count >= 2,
           asciiAlpha(scalars[0].value),
           scalars[1].value == 0x3A {
            return .failure(.driveQualified)
        }

        let segments = normalizedPath.split(separator: "/", omittingEmptySubsequences: false)
        guard !segments.contains(where: \.isEmpty) else {
            return .failure(.emptySegment)
        }
        if normalizedPath.unicodeScalars.contains(where: unsafeTrackedArtifactScalar) {
            return .failure(.unsafeCharacter)
        }

        for segmentSlice in segments {
            let segment = String(segmentSlice)
            if segment == "." || segment == ".." {
                return .failure(.dotSegment)
            }
            if segment.last == "." || segment.last == " " {
                return .failure(.trailingDotOrSpace)
            }
            let basename = String(segment.split(
                separator: ".",
                maxSplits: 1,
                omittingEmptySubsequences: false
            )[0])
            if windowsReservedBasenames.contains(
                TrackedArtifactUnicode17.fullUppercase(basename)
            ) {
                return .failure(.reservedBasename)
            }
        }

        return .success(normalizedPath)
    }

    private static func asciiAlpha(_ scalar: UInt32) -> Bool {
        (0x41 ... 0x5A).contains(scalar) || (0x61 ... 0x7A).contains(scalar)
    }

    private static func unsafeTrackedArtifactScalar(_ scalar: Unicode.Scalar) -> Bool {
        scalar.value < 0x20 || [0x3C, 0x3E, 0x3A, 0x22, 0x7C, 0x3F, 0x2A].contains(scalar.value)
    }

    private static func unicodeScalarEqual(_ left: String, _ right: String) -> Bool {
        left.unicodeScalars.elementsEqual(right.unicodeScalars, by: { $0.value == $1.value })
    }

    private static func trackedArtifactDiagnosticLessThan(
        _ left: TrackedArtifactDiagnostic,
        _ right: TrackedArtifactDiagnostic
    ) -> Bool {
        if left.code != right.code { return left.code < right.code }
        let pathComparison = compareUnicodeScalars(left.path, right.path)
        if pathComparison != 0 { return pathComparison < 0 }
        return canonicalDetails(left.details) < canonicalDetails(right.details)
    }

    private static func compareUnicodeScalars(_ left: String, _ right: String) -> Int {
        let leftScalars = left.unicodeScalars.map(\.value)
        let rightScalars = right.unicodeScalars.map(\.value)
        for (leftValue, rightValue) in zip(leftScalars, rightScalars) where leftValue != rightValue {
            return leftValue < rightValue ? -1 : 1
        }
        if leftScalars.count == rightScalars.count { return 0 }
        return leftScalars.count < rightScalars.count ? -1 : 1
    }

    private static func canonicalDetails(_ details: TrackedArtifactDiagnosticDetails) -> String {
        var value = "{\"entry_kind\": \"\(details.entryKind.rawValue)\", \"ordinal\": \(details.ordinal)"
        if let problem = details.problem {
            value += ", \"problem\": \"\(problem)\""
        }
        return value + "}"
    }

    public static func validateCIFullBuildToolchains(repoRoot: String, packages: [BuildPackage]) -> String? {
        let ciPath = (repoRoot as NSString).appendingPathComponent(".github/workflows/ci.yml")
        guard let workflow = try? String(contentsOfFile: ciPath, encoding: .utf8) else {
            return nil
        }

        guard workflow.contains("Full build on main merge") else {
            return nil
        }

        let compactWorkflow = workflow.replacingOccurrences(
            of: #"\s+"#,
            with: "",
            options: .regularExpression
        )

        let languages = Set(packages.map(\.language)).intersection(ciManagedToolchainLanguages).sorted()
        var missingOutputBinding: [String] = []
        var missingMainForce: [String] = []

        for language in languages {
            let outputBinding = "needs_\(language):${{steps.toolchains.outputs.needs_\(language)}}"
            if !compactWorkflow.contains(outputBinding) {
                missingOutputBinding.append(language)
            }
            if !compactWorkflow.contains("needs_\(language)=true") {
                missingMainForce.append(language)
            }
        }

        if missingOutputBinding.isEmpty, missingMainForce.isEmpty {
            return nil
        }

        var parts: [String] = []
        if !missingOutputBinding.isEmpty {
            parts.append(
                "detect outputs for forced main full builds are not normalized through steps.toolchains for: \(missingOutputBinding.joined(separator: ", "))"
            )
        }
        if !missingMainForce.isEmpty {
            parts.append(
                "forced main full-build path does not explicitly enable toolchains for: \(missingMainForce.joined(separator: ", "))"
            )
        }

        return "\(ciPath.replacingOccurrences(of: "\\", with: "/")): \(parts.joined(separator: "; "))"
    }

    public static func validateBuildContracts(repoRoot: String, packages: [BuildPackage]) -> String? {
        var errors: [String] = []

        if let ciError = validateCIFullBuildToolchains(repoRoot: repoRoot, packages: packages) {
            errors.append(ciError)
        }

        errors.append(contentsOf: validateLuaIsolatedBuildFiles(packages: packages))
        errors.append(contentsOf: validatePerlBuildFiles(packages: packages))

        return errors.isEmpty ? nil : errors.joined(separator: "\n  - ")
    }

    static func validateLuaIsolatedBuildFiles(packages: [BuildPackage]) -> [String] {
        var errors: [String] = []

        for package in packages where package.language == "lua" {
            let selfRock = "coding-adventures-\((package.path as NSString).lastPathComponent.replacingOccurrences(of: "_", with: "-"))"
            var buildLines: [String: [String]] = [:]

            for buildPath in luaBuildFiles(packagePath: package.path) {
                let lines = readBuildLines(buildPath: buildPath)
                buildLines[(buildPath as NSString).lastPathComponent] = lines
                guard !lines.isEmpty else {
                    continue
                }

                if let foreignRemove = firstForeignLuaRemove(lines: lines, selfRock: selfRock) {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD removes unrelated rock \(foreignRemove); isolated package builds should only remove the package they are rebuilding"
                    )
                }

                let stateMachineIndex = firstLineContaining(lines: lines, needles: ["../state_machine", "..\\state_machine"])
                let directedGraphIndex = firstLineContaining(lines: lines, needles: ["../directed_graph", "..\\directed_graph"])
                if let stateMachineIndex, let directedGraphIndex, stateMachineIndex < directedGraphIndex {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first"
                    )
                }

                if (hasGuardedLocalLuaInstall(lines: lines) ||
                    (((buildPath as NSString).lastPathComponent == "BUILD_windows") &&
                     hasLocalLuaSiblingInstall(lines: lines))) &&
                    !selfInstallDisablesDeps(lines: lines, selfRock: selfRock) {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest"
                    )
                }
            }

            let missingWindowsDeps = missingLuaSiblingInstalls(
                unixLines: buildLines["BUILD"] ?? [],
                windowsLines: buildLines["BUILD_windows"] ?? []
            )
            if !missingWindowsDeps.isEmpty {
                let buildPath = (package.path as NSString).appendingPathComponent("BUILD_windows")
                errors.append(
                    "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD_windows is missing sibling installs present in BUILD: \(missingWindowsDeps.joined(separator: ", "))"
                )
            }
        }

        return errors
    }

    static func validatePerlBuildFiles(packages: [BuildPackage]) -> [String] {
        var errors: [String] = []

        for package in packages where package.language == "perl" {
            for buildPath in luaBuildFiles(packagePath: package.path) {
                let lines = readBuildLines(buildPath: buildPath)
                if lines.contains(where: { line in
                    line.contains("cpanm") &&
                        line.contains("Test2::V0") &&
                        !line.contains("--notest")
                }) {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself"
                    )
                }
            }
        }

        return errors
    }

    static func luaBuildFiles(packagePath: String) -> [String] {
        guard let entries = try? FileManager.default.contentsOfDirectory(atPath: packagePath) else {
            return []
        }

        return entries
            .filter { $0.hasPrefix("BUILD") }
            .sorted()
            .map { (packagePath as NSString).appendingPathComponent($0) }
            .filter { FileManager.default.fileExists(atPath: $0) }
    }

    static func readBuildLines(buildPath: String) -> [String] {
        guard let contents = try? String(contentsOfFile: buildPath, encoding: .utf8) else {
            return []
        }

        return contents
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty && !$0.hasPrefix("#") }
    }

    static func firstForeignLuaRemove(lines: [String], selfRock: String) -> String? {
        let pattern = try? NSRegularExpression(pattern: #"\bluarocks remove --force ([^ \t]+)"#)

        for line in lines {
            guard let pattern else { return nil }
            let range = NSRange(line.startIndex..<line.endIndex, in: line)
            guard let match = pattern.firstMatch(in: line, options: [], range: range),
                  let targetRange = Range(match.range(at: 1), in: line)
            else {
                continue
            }

            let target = String(line[targetRange])
            if target != selfRock {
                return target
            }
        }

        return nil
    }

    static func firstLineContaining(lines: [String], needles: [String]) -> Int? {
        for (index, line) in lines.enumerated() where needles.contains(where: { line.contains($0) }) {
            return index
        }
        return nil
    }

    static func hasGuardedLocalLuaInstall(lines: [String]) -> Bool {
        lines.contains { line in
            line.contains("luarocks show ") && (line.contains("../") || line.contains("..\\"))
        }
    }

    static func hasLocalLuaSiblingInstall(lines: [String]) -> Bool {
        !luaSiblingInstallDirs(lines: lines).isEmpty
    }

    static func selfInstallDisablesDeps(lines: [String], selfRock: String) -> Bool {
        lines.contains { line in
            line.contains("luarocks make") &&
                line.contains(selfRock) &&
                (line.contains("--deps-mode=none") ||
                    line.contains("--deps-mode none") ||
                    line.contains("--no-manifest"))
        }
    }

    static func missingLuaSiblingInstalls(unixLines: [String], windowsLines: [String]) -> [String] {
        let windowsDeps = Set(luaSiblingInstallDirs(lines: windowsLines))
        return luaSiblingInstallDirs(lines: unixLines).filter { !windowsDeps.contains($0) }
    }

    static func luaSiblingInstallDirs(lines: [String]) -> [String] {
        let pattern = try? NSRegularExpression(pattern: #"\bcd\s+([.][.][\\/][^ \t\r\n&()]+)"#)
        var deps = Set<String>()

        for line in lines {
            guard line.contains("luarocks make"), let pattern else {
                continue
            }

            let range = NSRange(line.startIndex..<line.endIndex, in: line)
            guard let match = pattern.firstMatch(in: line, options: [], range: range),
                  let depRange = Range(match.range(at: 1), in: line)
            else {
                continue
            }

            deps.insert(String(line[depRange]).replacingOccurrences(of: "\\", with: "/"))
        }

        return deps.sorted()
    }
}
