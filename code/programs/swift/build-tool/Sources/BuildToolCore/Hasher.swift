import Foundation
import SHA256
#if os(Windows)
import WinSDK
#elseif canImport(Darwin)
import Darwin
#else
import Glibc
#endif

public struct PackageHashError: Error, LocalizedError, Equatable, Sendable {
    public let packageName: String

    public init(packageName: String) {
        self.packageName = packageName
    }

    public var errorDescription: String? {
        "HASH_PACKAGE_FAILED: package=\(Self.quote(packageName))"
    }

    /// Diagnostics quote controls explicitly so a directory basename cannot
    /// forge a second terminal or GitHub Actions command.
    private static func quote(_ value: String) -> String {
        var result = "\""
        for scalar in value.unicodeScalars {
            switch scalar.value {
            case 0x22:
                result += "\\\""
            case 0x5C:
                result += "\\\\"
            case 0x08:
                result += "\\b"
            case 0x09:
                result += "\\t"
            case 0x0A:
                result += "\\n"
            case 0x0C:
                result += "\\f"
            case 0x0D:
                result += "\\r"
            case 0x00 ... 0x1F, 0x7F ... 0x9F:
                result += String(format: "\\u%04x", scalar.value)
            default:
                switch scalar.properties.generalCategory {
                case .control, .format, .lineSeparator, .paragraphSeparator:
                    if scalar.value <= 0xFFFF {
                        result += String(format: "\\u%04x", scalar.value)
                    } else {
                        result += String(format: "\\U%08x", scalar.value)
                    }
                default:
                    result.unicodeScalars.append(scalar)
                }
            }
        }
        result += "\""
        return result
    }
}

private enum SourceHashInputError: Error {
    case unavailable
    case unsafePath
    case unstable
    case limitExceeded
}

public enum Hasher {
    /// This is the production selector, not a test copy. The checked JSON is
    /// embedded into generated Swift source, decoded without filesystem or
    /// environment authority, and compared field-for-field in the test suite.
    static let languageSourceInputRegistry = LanguageSourceInputRegistryProjection.value
    static let languageSourceInputRegistryDigest =
        "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70"

    private static let maximumCandidateCount = 100_000
    private static let maximumSelectedInputCount = 50_000
    private static let maximumFileBytes: UInt64 = 64 * 1024 * 1024
    private static let maximumPackageBytes: UInt64 = 1024 * 1024 * 1024
    private static let windowsReservedBasenames: Set<String> = Set(
        ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"] +
            ["COM", "LPT"].flatMap { prefix in
                (1 ... 9).map { "\(prefix)\($0)" } +
                    ["¹", "²", "³"].map { "\(prefix)\($0)" }
            }
    )

    public static func hashPackage(
        _ package: BuildPackage,
        repositoryRoot: String
    ) throws -> String {
        do {
            let packageRoot = try repositoryPackagePath(
                package.path,
                repositoryRoot: repositoryRoot,
                expectedLanguage: package.language
            )
            let rootBefore = try secureDirectoryState(
                package.path,
                repositoryRoot: repositoryRoot
            )
            let inputs = try collectSourceInputs(
                package,
                repositoryRoot: repositoryRoot
            )
            let files = inputs.files
            guard try secureDirectoryState(
                package.path,
                repositoryRoot: repositoryRoot
            ) == rootBefore else {
                throw SourceHashInputError.unstable
            }
            var digest = SHA256Hasher()
            var fileStates: [(String, SecureObjectState)] = []
            var packageBytes: UInt64 = 0
            for file in files {
                let relative = try portableRelativePath(file, root: package.path)
                let repositoryPath = packageRoot + "/" + relative
                try validatePortablePath(repositoryPath)
                let snapshot = try secureFileSnapshot(
                    file,
                    repositoryRoot: repositoryRoot
                )
                packageBytes = try checkedPackageByteTotal(
                    current: packageBytes,
                    fileBytes: UInt64(snapshot.data.count)
                )
                appendFrame(Data(repositoryPath.utf8), to: &digest)
                appendFrame(snapshot.data, to: &digest)
                fileStates.append((file, snapshot.state))
            }
            for (file, state) in fileStates {
                guard try secureFileState(
                    file,
                    repositoryRoot: repositoryRoot
                ) == state else {
                    throw SourceHashInputError.unstable
                }
            }
            for (directory, state) in inputs.directoryStates {
                guard try secureDirectoryState(
                    directory,
                    repositoryRoot: repositoryRoot
                ) == state else {
                    throw SourceHashInputError.unstable
                }
            }
            guard try secureDirectoryState(
                package.path,
                repositoryRoot: repositoryRoot
            ) == rootBefore else {
                throw SourceHashInputError.unstable
            }
            return digest.hexDigest()
        } catch {
            throw PackageHashError(packageName: package.name)
        }
    }

    public static func hashDeps(
        packageName: String,
        graph: DirectedGraph,
        packageHashes: [String: String]
    ) -> String {
        let dependencies = graph.transitivePrerequisites(of: packageName).sorted()
        if dependencies.isEmpty {
            return hash(data: Data())
        }
        let combined = dependencies.map { packageHashes[$0] ?? "" }.joined()
        return hash(string: combined)
    }

    public static func collectSourceFiles(_ package: BuildPackage) throws -> [String] {
        try collectSourceInputs(package, repositoryRoot: nil).files
    }

    static func collectSourceFiles(
        _ package: BuildPackage,
        repositoryRoot: String
    ) throws -> [String] {
        try collectSourceInputs(package, repositoryRoot: repositoryRoot).files
    }

    private static func collectSourceInputs(
        _ package: BuildPackage,
        repositoryRoot: String?
    ) throws -> (
        files: [String],
        directoryStates: [(String, SecureObjectState)]
    ) {
        let root = package.path
        var files: [String] = []
        var directoryStates: [(String, SecureObjectState)] = []
        var candidateCount = 0
        var portableIdentities: [String: String] = [:]

        guard let languageInputs = languageSourceInputRegistry.inputs(
            for: package.language
        ) else {
            throw SourceHashInputError.unsafePath
        }
        let universal = languageSourceInputRegistry.universalInputs
        let generatedDirectories = Set(universal.generatedDirectoryComponents)
        let repositoryPackageRoot = try repositoryRoot.map {
            try repositoryPackagePath(
                package.path,
                repositoryRoot: $0,
                expectedLanguage: package.language
            )
        }
        let packageExactPaths = Set(
            languageInputs.packageExactInputs
                .filter { $0.packageRoot == repositoryPackageRoot }
                .flatMap(\.paths)
        )
        let declaredMode = package.isStarlark && !package.declaredSrcs.isEmpty

        guard try entryKind(root) == .directory else {
            throw SourceHashInputError.unavailable
        }

        func visit(directory: String, relativeDirectory: String) throws {
            let before = try repositoryRoot.map {
                try secureDirectoryState(directory, repositoryRoot: $0)
            }
            let entries = try boundedDirectoryEntries(
                directory,
                maximumEntries: maximumCandidateCount - candidateCount
            )

            for entry in entries {
                candidateCount += 1
                guard candidateCount <= maximumCandidateCount else {
                    throw SourceHashInputError.limitExceeded
                }
                let relativePath = relativeDirectory.isEmpty
                    ? entry
                    : "\(relativeDirectory)/\(entry)"
                let fullPath = (directory as NSString).appendingPathComponent(entry)
                let normalized = try portableCandidatePath(relativePath)
                try registerPortableIdentity(normalized, in: &portableIdentities)
                switch try entryKind(fullPath) {
                case .linked:
                    continue
                case .directory:
                    if !generatedDirectories.contains(entry) {
                        try visit(directory: fullPath, relativeDirectory: relativePath)
                    }
                    continue
                case .other:
                    continue
                case .regular:
                    break
                }

                let filename = (normalized as NSString).lastPathComponent
                let isRoot = !normalized.contains("/")
                var included = universal.buildFilenames.contains(filename)
                if isRoot && universal.rootExactBasenames.contains(filename) {
                    included = true
                }
                if isRoot && languageInputs.rootExactBasenames.contains(filename) {
                    included = true
                }
                if isRoot && languageInputs.rootVariableSuffixes.contains(
                    where: { filename.hasSuffix($0) }
                ) {
                    included = true
                }
                if languageInputs.rootExactRelativePaths.contains(normalized)
                    || packageExactPaths.contains(normalized) {
                    included = true
                }

                if !declaredMode {
                    if languageInputs.recursiveExactBasenames.contains(filename)
                        || languageInputs.recursiveSuffixes.contains(
                            where: { filename.hasSuffix($0) }
                        )
                        || languageInputs.scopedInputs.contains(
                            where: { $0.matches(path: normalized, basename: filename) }
                        ) {
                        included = true
                    }
                } else if !included && package.declaredSrcs.contains(
                    where: { GlobMatch.matchPath($0, normalized) }
                ) {
                    included = true
                }

                if included {
                    try appendSelectedInput(fullPath, to: &files)
                }
            }

            if let repositoryRoot, let before {
                guard try secureDirectoryState(
                    directory,
                    repositoryRoot: repositoryRoot
                ) == before else {
                    throw SourceHashInputError.unstable
                }
                directoryStates.append((directory, before))
            }
        }

        try visit(directory: root, relativeDirectory: "")

        let sortedFiles = files.sorted {
            let left = (try? portableRelativePath($0, root: root)) ?? $0
            let right = (try? portableRelativePath($1, root: root)) ?? $1
            return utf8LessThan(left, right)
        }
        return (sortedFiles, directoryStates)
    }

    private enum EntryKind: Equatable {
        case directory
        case regular
        case linked
        case other
    }

    private static func entryKind(_ path: String) throws -> EntryKind {
        #if os(Windows)
        let windowsAttributes = path.withCString(encodedAs: UTF16.self) {
            GetFileAttributesW($0)
        }
        guard windowsAttributes != INVALID_FILE_ATTRIBUTES else {
            throw SourceHashInputError.unavailable
        }
        if windowsAttributes & DWORD(FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
            return .linked
        }
        #else
        if (try? FileManager.default.destinationOfSymbolicLink(atPath: path)) != nil {
            return .linked
        }
        #endif

        let values = try URL(fileURLWithPath: path).resourceValues(
            forKeys: [.isDirectoryKey, .isRegularFileKey, .isSymbolicLinkKey]
        )
        if values.isSymbolicLink == true {
            return .linked
        }
        if values.isDirectory == true {
            return .directory
        }
        if values.isRegularFile == true {
            return .regular
        }
        return .other
    }

    private static func portableCandidatePath(_ path: String) throws -> String {
        #if os(Windows)
        let normalized = path.replacingOccurrences(of: "\\", with: "/")
        #else
        guard !path.contains("\\") else {
            throw SourceHashInputError.unsafePath
        }
        let normalized = path
        #endif
        try validatePortablePath(normalized)
        return normalized
    }

    private static func portableRelativePath(_ path: String, root: String) throws -> String {
        #if os(Windows)
        let normalizedPath = path.replacingOccurrences(of: "\\", with: "/")
        let normalizedRoot = root.replacingOccurrences(of: "\\", with: "/")
        #else
        guard !path.contains("\\"), !root.contains("\\") else {
            throw SourceHashInputError.unsafePath
        }
        let normalizedPath = path
        let normalizedRoot = root
        #endif
        if normalizedPath.hasPrefix(normalizedRoot + "/") {
            let relative = String(normalizedPath.dropFirst(normalizedRoot.count + 1))
            try validatePortablePath(relative)
            return relative
        }
        throw SourceHashInputError.unsafePath
    }

    private static func repositoryPackagePath(
        _ path: String,
        repositoryRoot: String,
        expectedLanguage: String
    ) throws -> String {
        let relative = try portableRelativePath(path, root: repositoryRoot)
        let components = relative.split(separator: "/").map(String.init)
        guard components.count >= 4,
              components[0] == "code",
              components[1] == "packages" || components[1] == "programs",
              components[2] == expectedLanguage else {
            throw SourceHashInputError.unsafePath
        }
        try validatePortablePath(relative)
        return relative
    }

    /// Incremental immediate-child enumeration applies the candidate ceiling
    /// before a directory can be materialized and sorted in memory.
    static func boundedDirectoryEntries(
        _ directory: String,
        maximumEntries: Int
    ) throws -> [String] {
        guard maximumEntries >= 0 else {
            throw SourceHashInputError.limitExceeded
        }
        var enumerationFailed = false
        guard let enumerator = FileManager.default.enumerator(
            at: URL(fileURLWithPath: directory),
            includingPropertiesForKeys: nil,
            options: [.skipsSubdirectoryDescendants],
            errorHandler: { _, _ in
                enumerationFailed = true
                return false
            }
        ) else {
            throw SourceHashInputError.unavailable
        }
        var entries: [String] = []
        entries.reserveCapacity(min(maximumEntries, 1_024))
        while let value = enumerator.nextObject() {
            guard !enumerationFailed, let url = value as? URL else {
                throw SourceHashInputError.unavailable
            }
            guard entries.count < maximumEntries else {
                throw SourceHashInputError.limitExceeded
            }
            entries.append(url.lastPathComponent)
        }
        guard !enumerationFailed else {
            throw SourceHashInputError.unavailable
        }
        return entries.sorted(by: utf8LessThan)
    }

    static func appendSelectedInput(
        _ path: String,
        to files: inout [String],
        maximumInputs: Int = maximumSelectedInputCount
    ) throws {
        guard files.count < maximumInputs else {
            throw SourceHashInputError.limitExceeded
        }
        files.append(path)
    }

    static func checkedPackageByteTotal(
        current: UInt64,
        fileBytes: UInt64,
        maximumFile: UInt64 = maximumFileBytes,
        maximumPackage: UInt64 = maximumPackageBytes
    ) throws -> UInt64 {
        guard fileBytes <= maximumFile else {
            throw SourceHashInputError.limitExceeded
        }
        let (next, overflow) = current.addingReportingOverflow(fileBytes)
        guard !overflow, next <= maximumPackage else {
            throw SourceHashInputError.limitExceeded
        }
        return next
    }

    static func canonicalLanguageSourceInputRegistryDigest(
        from data: Data
    ) throws -> String {
        let object = try JSONSerialization.jsonObject(with: data)
        let canonical = try JSONSerialization.data(
            withJSONObject: object,
            options: [.sortedKeys, .withoutEscapingSlashes]
        )
        var framed = Data(
            "coding-adventures/build-tool-language-source-input-registry/v1\0".utf8
        )
        var length = UInt64(canonical.count).bigEndian
        withUnsafeBytes(of: &length) { bytes in
            framed.append(contentsOf: bytes)
        }
        framed.append(canonical)
        return hash(data: framed)
    }

    static func validatePortablePath(_ path: String) throws {
        guard !path.isEmpty,
              path.unicodeScalars.count <= 512,
              !path.hasPrefix("/"),
              !path.contains("\\"),
              unicodeScalarEqual(TrackedArtifactUnicode17.nfc(path), path),
              !path.unicodeScalars.contains(where: unsafePortablePathScalar) else {
            throw SourceHashInputError.unsafePath
        }
        let scalars = Array(path.unicodeScalars)
        if scalars.count >= 2,
           asciiAlpha(scalars[0].value),
           scalars[1].value == 0x3A {
            throw SourceHashInputError.unsafePath
        }
        for componentSlice in path.split(
            separator: "/",
            omittingEmptySubsequences: false
        ) {
            let component = String(componentSlice)
            guard !component.isEmpty,
                  component != ".",
                  component != "..",
                  component.last != ".",
                  component.last != " " else {
                throw SourceHashInputError.unsafePath
            }
            let basename = String(component.split(
                separator: ".",
                maxSplits: 1,
                omittingEmptySubsequences: false
            )[0])
            if windowsReservedBasenames.contains(
                TrackedArtifactUnicode17.fullUppercase(basename)
            ) {
                throw SourceHashInputError.unsafePath
            }
        }
    }

    static func registerPortableIdentity(
        _ normalizedPath: String,
        in identities: inout [String: String]
    ) throws {
        let identity = TrackedArtifactUnicode17.casefold(normalizedPath)
        if let existing = identities[identity], existing != normalizedPath {
            throw SourceHashInputError.unsafePath
        }
        identities[identity] = normalizedPath
    }

    private static func asciiAlpha(_ scalar: UInt32) -> Bool {
        (0x41 ... 0x5A).contains(scalar) || (0x61 ... 0x7A).contains(scalar)
    }

    private static func unsafePortablePathScalar(_ scalar: Unicode.Scalar) -> Bool {
        if scalar.value < 0x20
            || [0x3C, 0x3E, 0x3A, 0x22, 0x7C, 0x3F, 0x2A].contains(scalar.value) {
            return true
        }
        switch scalar.properties.generalCategory {
        case .control, .format, .lineSeparator, .paragraphSeparator:
            return true
        default:
            return false
        }
    }

    private static func unicodeScalarEqual(_ left: String, _ right: String) -> Bool {
        left.unicodeScalars.elementsEqual(
            right.unicodeScalars,
            by: { $0.value == $1.value }
        )
    }

    /// Identity and mutation fields read from an already-open object. The
    /// fields deliberately include link count and metadata timestamps in
    /// addition to size so same-length replacements cannot pass unnoticed.
    private struct SecureObjectState: Equatable {
        let device: UInt64
        let identityHigh: UInt64
        let identityLow: UInt64
        let linkCount: UInt64
        let size: UInt64
        let modifiedHigh: UInt64
        let modifiedLow: UInt64
        let changedHigh: UInt64
        let changedLow: UInt64
        let attributes: UInt64
    }

    private static func secureDirectoryState(
        _ path: String,
        repositoryRoot: String
    ) throws -> SecureObjectState {
        #if os(Windows)
        return try withSecureWindowsObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: true
        ) { _, state in state }
        #else
        return try withSecurePOSIXObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: true
        ) { _, state in state }
        #endif
    }

    private struct SecureFileSnapshot {
        let data: Data
        let state: SecureObjectState
    }

    private static func secureFileState(
        _ path: String,
        repositoryRoot: String
    ) throws -> SecureObjectState {
        #if os(Windows)
        return try withSecureWindowsObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: false
        ) { _, state in state }
        #else
        return try withSecurePOSIXObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: false
        ) { _, state in state }
        #endif
    }

    private static func secureFileSnapshot(
        _ path: String,
        repositoryRoot: String
    ) throws -> SecureFileSnapshot {
        #if os(Windows)
        return try withSecureWindowsObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: false
        ) { handle, before in
            guard before.size <= maximumFileBytes else {
                throw SourceHashInputError.limitExceeded
            }
            let data = try readWindowsSnapshotData(
                handle,
                expectedSize: before.size
            )
            return SecureFileSnapshot(data: data, state: before)
        }
        #else
        return try withSecurePOSIXObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: false
        ) { descriptor, before in
            guard before.size <= maximumFileBytes else {
                throw SourceHashInputError.limitExceeded
            }
            let data = try readPOSIXSnapshotData(
                descriptor,
                expectedSize: before.size
            )
            return SecureFileSnapshot(data: data, state: before)
        }
        #endif
    }

    #if os(Windows)
    private static func readWindowsSnapshotData(
        _ handle: HANDLE,
        expectedSize: UInt64
    ) throws -> Data {
        var data = Data()
        data.reserveCapacity(Int(expectedSize))
        var buffer = [UInt8](repeating: 0, count: 64 * 1024)
        var remaining = expectedSize
        while remaining > 0 {
            var count = DWORD(0)
            let requested = DWORD(min(UInt64(buffer.count), remaining))
            let succeeded = buffer.withUnsafeMutableBytes { bytes in
                ReadFile(handle, bytes.baseAddress, requested, &count, nil)
            }
            guard succeeded, count > 0 else {
                throw SourceHashInputError.unstable
            }
            data.append(contentsOf: buffer.prefix(Int(count)))
            remaining -= UInt64(count)
        }
        var probe: UInt8 = 0
        var probeCount = DWORD(0)
        let succeeded = withUnsafeMutableBytes(of: &probe) { bytes in
            ReadFile(handle, bytes.baseAddress, 1, &probeCount, nil)
        }
        guard succeeded, probeCount == 0 else {
            throw SourceHashInputError.unstable
        }
        return data
    }
    #else
    private static func readPOSIXSnapshotData(
        _ descriptor: Int32,
        expectedSize: UInt64
    ) throws -> Data {
        var data = Data()
        data.reserveCapacity(Int(expectedSize))
        var buffer = [UInt8](repeating: 0, count: 64 * 1024)
        var remaining = expectedSize
        while remaining > 0 {
            let requested = min(UInt64(buffer.count), remaining)
            let count = buffer.withUnsafeMutableBytes { bytes in
                read(descriptor, bytes.baseAddress, Int(requested))
            }
            if count < 0 {
                if errno == EINTR {
                    continue
                }
                throw SourceHashInputError.unavailable
            }
            guard count > 0 else {
                throw SourceHashInputError.unstable
            }
            data.append(contentsOf: buffer.prefix(count))
            remaining -= UInt64(count)
        }
        var probe: UInt8 = 0
        while true {
            let count = withUnsafeMutableBytes(of: &probe) { bytes in
                read(descriptor, bytes.baseAddress, 1)
            }
            if count < 0, errno == EINTR {
                continue
            }
            if count < 0 {
                throw SourceHashInputError.unavailable
            }
            guard count == 0 else {
                throw SourceHashInputError.unstable
            }
            break
        }
        return data
    }

    static func readSecurePOSIXFileForGrowthTest(
        _ path: String,
        repositoryRoot: String,
        afterSnapshot: () throws -> Void
    ) throws -> Data {
        try withSecurePOSIXObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: false
        ) { descriptor, before in
            guard before.size <= maximumFileBytes else {
                throw SourceHashInputError.limitExceeded
            }
            try afterSnapshot()
            return try readPOSIXSnapshotData(
                descriptor,
                expectedSize: before.size
            )
        }
    }
    #endif

    #if os(Windows)
    private static func windowsState(
        _ handle: HANDLE,
        expectDirectory: Bool
    ) throws -> SecureObjectState {
        var information = BY_HANDLE_FILE_INFORMATION()
        guard GetFileInformationByHandle(handle, &information) else {
            throw SourceHashInputError.unavailable
        }
        var basicInformation = FILE_BASIC_INFO()
        guard GetFileInformationByHandleEx(
            handle,
            FileBasicInfo,
            &basicInformation,
            DWORD(MemoryLayout<FILE_BASIC_INFO>.size)
        ) else {
            throw SourceHashInputError.unavailable
        }
        let attributes = information.dwFileAttributes
        guard attributes & DWORD(FILE_ATTRIBUTE_REPARSE_POINT) == 0,
              (attributes & DWORD(FILE_ATTRIBUTE_DIRECTORY) != 0) == expectDirectory,
              expectDirectory || information.nNumberOfLinks == 1 else {
            throw SourceHashInputError.unsafePath
        }
        let modified = UInt64(bitPattern: basicInformation.LastWriteTime.QuadPart)
        let changed = UInt64(bitPattern: basicInformation.ChangeTime.QuadPart)
        return SecureObjectState(
            device: UInt64(information.dwVolumeSerialNumber),
            identityHigh: UInt64(information.nFileIndexHigh),
            identityLow: UInt64(information.nFileIndexLow),
            linkCount: UInt64(information.nNumberOfLinks),
            size: UInt64(information.nFileSizeHigh) << 32 | UInt64(information.nFileSizeLow),
            modifiedHigh: modified >> 32,
            modifiedLow: modified & 0xFFFF_FFFF,
            changedHigh: changed >> 32,
            changedLow: changed & 0xFFFF_FFFF,
            attributes: UInt64(attributes)
        )
    }

    private static func openWindowsObject(
        _ path: String,
        expectDirectory: Bool
    ) throws -> HANDLE {
        let flags = DWORD(FILE_FLAG_OPEN_REPARSE_POINT)
            | (expectDirectory ? DWORD(FILE_FLAG_BACKUP_SEMANTICS) : DWORD(FILE_FLAG_SEQUENTIAL_SCAN))
        let handle = path.withCString(encodedAs: UTF16.self) {
            CreateFileW(
                $0,
                DWORD(GENERIC_READ),
                DWORD(FILE_SHARE_READ),
                nil,
                DWORD(OPEN_EXISTING),
                flags,
                nil
            )
        }
        guard let handle, handle != INVALID_HANDLE_VALUE else {
            throw SourceHashInputError.unavailable
        }
        do {
            _ = try windowsState(handle, expectDirectory: expectDirectory)
            return handle
        } catch {
            CloseHandle(handle)
            throw error
        }
    }

    private static func withSecureWindowsObject<T>(
        _ path: String,
        repositoryRoot: String,
        expectDirectory: Bool,
        _ body: (HANDLE, SecureObjectState) throws -> T
    ) throws -> T {
        let relative = try portableRelativePath(path, root: repositoryRoot)
        let components = relative.split(separator: "/").map(String.init)
        guard !components.isEmpty else {
            throw SourceHashInputError.unsafePath
        }

        var handles: [HANDLE] = []
        defer {
            for handle in handles.reversed() {
                CloseHandle(handle)
            }
        }

        let rootHandle = try openWindowsObject(repositoryRoot, expectDirectory: true)
        handles.append(rootHandle)
        var directoryStates: [SecureObjectState] = [
            try windowsState(rootHandle, expectDirectory: true)
        ]
        var current = repositoryRoot
        for component in components.dropLast() {
            current = (current as NSString).appendingPathComponent(component)
            let handle = try openWindowsObject(current, expectDirectory: true)
            handles.append(handle)
            directoryStates.append(try windowsState(handle, expectDirectory: true))
        }

        current = (current as NSString).appendingPathComponent(components.last!)
        let finalHandle = try openWindowsObject(current, expectDirectory: expectDirectory)
        handles.append(finalHandle)
        let before = try windowsState(finalHandle, expectDirectory: expectDirectory)
        let result = try body(finalHandle, before)
        guard try windowsState(finalHandle, expectDirectory: expectDirectory) == before else {
            throw SourceHashInputError.unstable
        }
        for (index, handle) in handles.dropLast().enumerated() {
            guard try windowsState(handle, expectDirectory: true) == directoryStates[index] else {
                throw SourceHashInputError.unstable
            }
        }
        return result
    }
    #else
    private static func posixState(
        _ descriptor: Int32,
        expectDirectory: Bool
    ) throws -> SecureObjectState {
        var information = stat()
        guard fstat(descriptor, &information) == 0 else {
            throw SourceHashInputError.unavailable
        }
        let kind = information.st_mode & mode_t(S_IFMT)
        let expectedKind = expectDirectory ? mode_t(S_IFDIR) : mode_t(S_IFREG)
        guard kind == expectedKind,
              expectDirectory || information.st_nlink == 1 else {
            throw SourceHashInputError.unsafePath
        }
        #if canImport(Darwin)
        let modifiedSeconds = UInt64(bitPattern: Int64(information.st_mtimespec.tv_sec))
        let modifiedNanoseconds = UInt64(bitPattern: Int64(information.st_mtimespec.tv_nsec))
        let changedSeconds = UInt64(bitPattern: Int64(information.st_ctimespec.tv_sec))
        let changedNanoseconds = UInt64(bitPattern: Int64(information.st_ctimespec.tv_nsec))
        #else
        let modifiedSeconds = UInt64(bitPattern: Int64(information.st_mtim.tv_sec))
        let modifiedNanoseconds = UInt64(bitPattern: Int64(information.st_mtim.tv_nsec))
        let changedSeconds = UInt64(bitPattern: Int64(information.st_ctim.tv_sec))
        let changedNanoseconds = UInt64(bitPattern: Int64(information.st_ctim.tv_nsec))
        #endif
        guard information.st_size >= 0 else {
            throw SourceHashInputError.unstable
        }
        return SecureObjectState(
            device: UInt64(information.st_dev),
            identityHigh: 0,
            identityLow: UInt64(information.st_ino),
            linkCount: UInt64(information.st_nlink),
            size: UInt64(information.st_size),
            modifiedHigh: modifiedSeconds,
            modifiedLow: modifiedNanoseconds,
            changedHigh: changedSeconds,
            changedLow: changedNanoseconds,
            attributes: UInt64(information.st_mode)
        )
    }

    private static func openPOSIXObject(
        at parent: Int32?,
        path: String,
        expectDirectory: Bool
    ) throws -> Int32 {
        let flags = O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC
            | (expectDirectory ? O_DIRECTORY : 0)
        let descriptor = path.withCString { pointer in
            if let parent {
                return openat(parent, pointer, flags)
            }
            return open(pointer, flags)
        }
        guard descriptor >= 0 else {
            throw SourceHashInputError.unavailable
        }
        do {
            _ = try posixState(descriptor, expectDirectory: expectDirectory)
            return descriptor
        } catch {
            close(descriptor)
            throw error
        }
    }

    private static func withSecurePOSIXObject<T>(
        _ path: String,
        repositoryRoot: String,
        expectDirectory: Bool,
        _ body: (Int32, SecureObjectState) throws -> T
    ) throws -> T {
        let relative = try portableRelativePath(path, root: repositoryRoot)
        let components = relative.split(separator: "/").map(String.init)
        guard !components.isEmpty else {
            throw SourceHashInputError.unsafePath
        }

        var descriptors: [Int32] = []
        defer {
            for descriptor in descriptors.reversed() {
                close(descriptor)
            }
        }

        let rootDescriptor = try openPOSIXObject(
            at: nil,
            path: repositoryRoot,
            expectDirectory: true
        )
        descriptors.append(rootDescriptor)
        var directoryStates: [SecureObjectState] = [
            try posixState(rootDescriptor, expectDirectory: true)
        ]
        var parent = rootDescriptor
        for component in components.dropLast() {
            let descriptor = try openPOSIXObject(
                at: parent,
                path: component,
                expectDirectory: true
            )
            descriptors.append(descriptor)
            directoryStates.append(try posixState(descriptor, expectDirectory: true))
            parent = descriptor
        }

        let finalDescriptor = try openPOSIXObject(
            at: parent,
            path: components.last!,
            expectDirectory: expectDirectory
        )
        descriptors.append(finalDescriptor)
        let before = try posixState(finalDescriptor, expectDirectory: expectDirectory)
        let result = try body(finalDescriptor, before)
        guard try posixState(finalDescriptor, expectDirectory: expectDirectory) == before else {
            throw SourceHashInputError.unstable
        }
        for (index, descriptor) in descriptors.dropLast().enumerated() {
            guard try posixState(descriptor, expectDirectory: true) == directoryStates[index] else {
                throw SourceHashInputError.unstable
            }
        }

        let reopened = try openPOSIXObject(
            at: parent,
            path: components.last!,
            expectDirectory: expectDirectory
        )
        defer { close(reopened) }
        guard try posixState(reopened, expectDirectory: expectDirectory) == before else {
            throw SourceHashInputError.unstable
        }
        return result
    }
    #endif

    private static func appendFrame(_ data: Data, to hasher: inout SHA256Hasher) {
        var length = UInt64(data.count).bigEndian
        withUnsafeBytes(of: &length) { bytes in
            hasher.update(Data(bytes))
        }
        hasher.update(data)
    }

    private static func utf8LessThan(_ left: String, _ right: String) -> Bool {
        left.utf8.lexicographicallyPrecedes(right.utf8)
    }

    private static func hash(string: String) -> String {
        hash(data: Data(string.utf8))
    }

    private static func hash(data: Data) -> String {
        sha256Hex(data)
    }
}
