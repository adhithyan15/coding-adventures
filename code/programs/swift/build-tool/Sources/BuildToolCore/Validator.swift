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

public struct OrphanManifest: Codable, Equatable, Sendable {
    public let path: String
    public let kind: String

    public init(path: String, kind: String) {
        self.path = path
        self.kind = kind
    }
}

public struct OrphanBuildFile: Codable, Equatable, Sendable {
    public let path: String
    public let state: String

    public init(path: String, state: String) {
        self.path = path
        self.state = state
    }
}

public struct OrphanExemption: Codable, Equatable, Sendable {
    public let line: Int
    public let kind: String
    public let path: String
    public let reason: String?

    public init(line: Int, kind: String, path: String, reason: String?) {
        self.line = line
        self.kind = kind
        self.path = path
        self.reason = reason
    }
}

public struct OrphanSnapshot: Codable, Equatable, Sendable {
    public let directories: [String]
    public let manifests: [OrphanManifest]
    public let buildFiles: [OrphanBuildFile]
    public let exemptions: [OrphanExemption]

    public init(
        directories: [String],
        manifests: [OrphanManifest],
        buildFiles: [OrphanBuildFile],
        exemptions: [OrphanExemption]
    ) {
        self.directories = directories
        self.manifests = manifests
        self.buildFiles = buildFiles
        self.exemptions = exemptions
    }

    enum CodingKeys: String, CodingKey {
        case directories
        case manifests
        case buildFiles = "build_files"
        case exemptions
    }
}

public struct OrphanDiagnosticDetails: Codable, Equatable, Sendable {
    public let buildPath: String?
    public let manifestKind: String?
    public let line: Int?
    public let problem: String?
    public let entryPath: String?
    public let kind: String?

    public init(
        buildPath: String? = nil,
        manifestKind: String? = nil,
        line: Int? = nil,
        problem: String? = nil,
        entryPath: String? = nil,
        kind: String? = nil
    ) {
        self.buildPath = buildPath
        self.manifestKind = manifestKind
        self.line = line
        self.problem = problem
        self.entryPath = entryPath
        self.kind = kind
    }

    enum CodingKeys: String, CodingKey {
        case buildPath = "build_path"
        case manifestKind = "manifest_kind"
        case line
        case problem
        case entryPath = "entry_path"
        case kind
    }
}

public struct OrphanDiagnostic: Codable, Equatable, Sendable {
    public let code: String
    public let severity: String
    public let path: String
    public let details: OrphanDiagnosticDetails

    public init(
        code: String,
        severity: String,
        path: String,
        details: OrphanDiagnosticDetails
    ) {
        self.code = code
        self.severity = severity
        self.path = path
        self.details = details
    }
}

public struct OrphanValidationResult: Codable, Equatable, Sendable {
    public let valid: Bool
    public let diagnosticCodes: [String]
    public let pendingExemptionCount: Int
    public let diagnostics: [OrphanDiagnostic]

    public init(
        valid: Bool,
        diagnosticCodes: [String],
        pendingExemptionCount: Int,
        diagnostics: [OrphanDiagnostic]
    ) {
        self.valid = valid
        self.diagnosticCodes = diagnosticCodes
        self.pendingExemptionCount = pendingExemptionCount
        self.diagnostics = diagnostics
    }

    enum CodingKeys: String, CodingKey {
        case valid
        case diagnosticCodes = "diagnostic_codes"
        case pendingExemptionCount = "pending_exemption_count"
        case diagnostics
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
    private static let orphanScanRoot = "code"
    private static let orphanLedgerPath = "code/BUILD-EXEMPTIONS"
    private static let orphanBuildRanks = [
        "BUILD": 0,
        "BUILD_windows": 1,
        "BUILD_mac": 2,
        "BUILD_linux": 3,
        "BUILD_mac_and_linux": 4,
    ]
    private static let orphanSkipComponents: Set<String> = [
        ".git",
        "target",
        "node_modules",
        "vendor",
        ".venv",
        "_build",
        "deps",
        ".build",
        "dist-newstyle",
        ".cargo",
    ]
    private static let pythonBlankScalars: Set<UInt32> = {
        var values = Set<UInt32>()
        values.formUnion(0x0009 ... 0x000D)
        values.formUnion(0x001C ... 0x0020)
        values.formUnion([0x0085, 0x00A0, 0x1680])
        values.formUnion(0x2000 ... 0x200A)
        values.formUnion([0x2028, 0x2029, 0x202F, 0x205F, 0x3000])
        return values
    }()

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

    /// Validate a bounded, inert Cargo/BUILD/exemption snapshot.
    ///
    /// The caller owns snapshot construction. This pure adapter never
    /// enumerates the checkout, follows links, invokes Git or another process,
    /// reads environment state, opens credentials, or accesses the network.
    public static func validateOrphanCrateSnapshot(
        _ snapshot: OrphanSnapshot
    ) -> OrphanValidationResult {
        let manifests = snapshot.manifests.filter { !orphanArtifactPath($0.path) }
        let directoryKeys = Set(snapshot.directories.map(scalarKey))
        let manifestKeys = Set(manifests.map { scalarKey($0.path) })
        let exemptionValidation = validateOrphanExemptions(snapshot.exemptions)
        let activation = activateOrphanExemptions(
            exemptions: exemptionValidation.valid,
            directoryKeys: directoryKeys,
            manifestKeys: manifestKeys,
            buildFiles: snapshot.buildFiles
        )

        var diagnostics = exemptionValidation.diagnostics + activation.diagnostics
        for manifest in manifests {
            if coveringOrphanBuild(
                state: "runnable",
                manifestPath: manifest.path,
                buildFiles: snapshot.buildFiles
            ) != nil || activation.activePaths.contains(scalarKey(manifest.path)) {
                continue
            }

            let emptyBuild = coveringOrphanBuild(
                state: "empty",
                manifestPath: manifest.path,
                buildFiles: snapshot.buildFiles
            )
            diagnostics.append(OrphanDiagnostic(
                code: emptyBuild == nil ? "ORPHAN_CRATE_UNLISTED" : "ORPHAN_CRATE_EMPTY_BUILD",
                severity: "error",
                path: manifest.path,
                details: OrphanDiagnosticDetails(
                    buildPath: emptyBuild?.path,
                    manifestKind: manifest.kind
                )
            ))
        }

        diagnostics.sort(by: orphanDiagnosticLessThan)
        return OrphanValidationResult(
            valid: diagnostics.isEmpty,
            diagnosticCodes: Array(Set(diagnostics.map(\.code))).sorted(),
            pendingExemptionCount: activation.pendingCount,
            diagnostics: diagnostics
        )
    }

    private static func validateOrphanExemptions(
        _ exemptions: [OrphanExemption]
    ) -> (diagnostics: [OrphanDiagnostic], valid: [OrphanExemption]) {
        var seen = Set<[UInt32]>()
        var diagnostics: [OrphanDiagnostic] = []
        var valid: [OrphanExemption] = []

        for exemption in exemptions {
            let portable = portableOrphanPath(exemption.path)
            let identity = portable
                ? scalarKey(TrackedArtifactUnicode17.casefold(
                    TrackedArtifactUnicode17.nfc(exemption.path)
                ))
                : nil
            let duplicate = identity.map { seen.contains($0) } ?? false
            if let identity {
                seen.insert(identity)
            }

            let pathProblem: String?
            if !portable {
                pathProblem = "PATH_UNSAFE"
            } else if !underOrphanScanRoot(exemption.path) {
                pathProblem = "PATH_OUTSIDE_SCAN"
            } else if orphanArtifactPath(exemption.path) {
                pathProblem = "PATH_ARTIFACT"
            } else {
                pathProblem = nil
            }

            let problem: String?
            if exemption.kind != "EXCLUDED", exemption.kind != "PENDING" {
                problem = "UNKNOWN_KIND"
            } else if !validOrphanReason(exemption.reason) {
                problem = "REASON_MISSING"
            } else if duplicate {
                problem = "DUPLICATE_PATH"
            } else {
                problem = pathProblem
            }

            if let problem {
                diagnostics.append(OrphanDiagnostic(
                    code: "ORPHAN_EXEMPTION_INVALID",
                    severity: "error",
                    path: orphanLedgerPath,
                    details: OrphanDiagnosticDetails(line: exemption.line, problem: problem)
                ))
            } else {
                valid.append(exemption)
            }
        }
        return (diagnostics, valid)
    }

    private static func activateOrphanExemptions(
        exemptions: [OrphanExemption],
        directoryKeys: Set<[UInt32]>,
        manifestKeys: Set<[UInt32]>,
        buildFiles: [OrphanBuildFile]
    ) -> (diagnostics: [OrphanDiagnostic], activePaths: Set<[UInt32]>, pendingCount: Int) {
        var diagnostics: [OrphanDiagnostic] = []
        var activePaths = Set<[UInt32]>()
        var pendingCount = 0

        for exemption in exemptions {
            let key = scalarKey(exemption.path)
            let problem: String?
            if !directoryKeys.contains(key) {
                problem = "MISSING_DIRECTORY"
            } else if !manifestKeys.contains(key) {
                problem = "NO_MANIFEST"
            } else if coveringOrphanBuild(
                state: "runnable",
                manifestPath: exemption.path,
                buildFiles: buildFiles
            ) != nil {
                problem = "COVERED"
            } else {
                problem = nil
            }

            if let problem {
                diagnostics.append(OrphanDiagnostic(
                    code: "ORPHAN_EXEMPTION_STALE",
                    severity: "error",
                    path: orphanLedgerPath,
                    details: OrphanDiagnosticDetails(
                        line: exemption.line,
                        problem: problem,
                        entryPath: exemption.path,
                        kind: exemption.kind
                    )
                ))
            } else {
                activePaths.insert(key)
                if exemption.kind == "PENDING" {
                    pendingCount += 1
                }
            }
        }
        return (diagnostics, activePaths, pendingCount)
    }

    private static func coveringOrphanBuild(
        state: String,
        manifestPath: String,
        buildFiles: [OrphanBuildFile]
    ) -> OrphanBuildFile? {
        let candidates = buildFiles.compactMap {
            buildFile -> (file: OrphanBuildFile, parent: String, rank: Int)? in
            guard buildFile.state == state,
                  let split = splitOrphanBuildPath(buildFile.path),
                  let rank = orphanBuildRanks[split.name],
                  underOrphanScanRoot(split.parent),
                  unicodeScalarEqual(manifestPath, split.parent) ||
                  scalarHasPrefix(manifestPath, split.parent + "/") else {
                return nil
            }
            return (buildFile, split.parent, rank)
        }
        return candidates.sorted { left, right in
            let leftDepth = orphanPathDepth(left.parent)
            let rightDepth = orphanPathDepth(right.parent)
            if leftDepth != rightDepth { return leftDepth > rightDepth }
            if left.rank != right.rank { return left.rank < right.rank }
            return compareUnicodeScalars(left.file.path, right.file.path) < 0
        }.first?.file
    }

    private static func splitOrphanBuildPath(_ path: String) -> (parent: String, name: String)? {
        guard let slash = path.lastIndex(of: "/") else { return nil }
        return (String(path[..<slash]), String(path[path.index(after: slash)...]))
    }

    private static func portableOrphanPath(_ path: String) -> Bool {
        let scalars = Array(path.unicodeScalars)
        guard !scalars.isEmpty,
              scalars.count <= 512,
              unicodeScalarEqual(TrackedArtifactUnicode17.nfc(path), path),
              scalars[0].value != 0x2F,
              !(scalars.count >= 2 && asciiAlpha(scalars[0].value) && scalars[1].value == 0x3A),
              !scalars.contains(where: { $0.value == 0x5C }),
              !path.contains("//"),
              !scalars.contains(where: unsafeTrackedArtifactScalar) else {
            return false
        }

        let components = path.split(separator: "/", omittingEmptySubsequences: false)
        for componentSlice in components {
            let component = String(componentSlice)
            let componentScalars = Array(component.unicodeScalars)
            guard !componentScalars.isEmpty,
                  component != ".",
                  component != "..",
                  componentScalars.last?.value != 0x2E,
                  componentScalars.last?.value != 0x20 else {
                return false
            }
            let basename = String(component.split(
                separator: ".",
                maxSplits: 1,
                omittingEmptySubsequences: false
            )[0])
            if windowsReservedBasenames.contains(TrackedArtifactUnicode17.fullUppercase(basename)) {
                return false
            }
        }
        return true
    }

    private static func validOrphanReason(_ reason: String?) -> Bool {
        guard let reason else { return false }
        let scalars = Array(reason.unicodeScalars)
        return scalars.count <= 4_096 && !scalars.allSatisfy {
            pythonBlankScalars.contains($0.value)
        }
    }

    private static func underOrphanScanRoot(_ path: String) -> Bool {
        unicodeScalarEqual(path, orphanScanRoot) || scalarHasPrefix(path, orphanScanRoot + "/")
    }

    private static func orphanArtifactPath(_ path: String) -> Bool {
        path.split(separator: "/", omittingEmptySubsequences: false).contains {
            orphanSkipComponents.contains(String($0))
        }
    }

    private static func orphanPathDepth(_ path: String) -> Int {
        path.split(separator: "/", omittingEmptySubsequences: false).count
    }

    private static func scalarKey(_ value: String) -> [UInt32] {
        value.unicodeScalars.map(\.value)
    }

    private static func scalarHasPrefix(_ value: String, _ prefix: String) -> Bool {
        scalarKey(value).starts(with: scalarKey(prefix))
    }

    private static func orphanDiagnosticLessThan(
        _ left: OrphanDiagnostic,
        _ right: OrphanDiagnostic
    ) -> Bool {
        if left.code != right.code { return left.code < right.code }
        let pathComparison = compareUnicodeScalars(left.path, right.path)
        if pathComparison != 0 { return pathComparison < 0 }
        return canonicalOrphanDetails(left.details) < canonicalOrphanDetails(right.details)
    }

    private static func canonicalOrphanDetails(_ details: OrphanDiagnosticDetails) -> String {
        if let manifestKind = details.manifestKind {
            let build = details.buildPath.map { "\"build_path\":" + jsonASCIIString($0) + "," } ?? ""
            return "{" + build + "\"manifest_kind\":" + jsonASCIIString(manifestKind) + "}"
        }
        if let entryPath = details.entryPath,
           let kind = details.kind,
           let line = details.line,
           let problem = details.problem {
            return "{\"entry_path\":" + jsonASCIIString(entryPath) +
                ",\"kind\":" + jsonASCIIString(kind) +
                ",\"line\":\(line),\"problem\":" + jsonASCIIString(problem) + "}"
        }
        return "{\"line\":\(details.line ?? 0),\"problem\":" +
            jsonASCIIString(details.problem ?? "") + "}"
    }

    private static func jsonASCIIString(_ value: String) -> String {
        var result = "\""
        for scalar in value.unicodeScalars {
            switch scalar.value {
            case 0x22: result += "\\\""
            case 0x5C: result += "\\\\"
            case 0x08: result += "\\b"
            case 0x0C: result += "\\f"
            case 0x0A: result += "\\n"
            case 0x0D: result += "\\r"
            case 0x09: result += "\\t"
            case 0x20 ... 0x7E: result += String(scalar)
            case 0x0000 ... 0xFFFF:
                result += unicodeEscape(scalar.value)
            default:
                let adjusted = scalar.value - 0x10000
                result += unicodeEscape(0xD800 + adjusted / 0x400)
                result += unicodeEscape(0xDC00 + adjusted % 0x400)
            }
        }
        return result + "\""
    }

    private static func unicodeEscape(_ value: UInt32) -> String {
        let digits = Array("0123456789abcdef".unicodeScalars)
        var result = "\\u"
        for shift in stride(from: 12, through: 0, by: -4) {
            result += String(digits[Int((value >> UInt32(shift)) & 0xF)])
        }
        return result
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
