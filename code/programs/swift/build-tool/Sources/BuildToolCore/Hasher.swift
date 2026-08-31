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
}

public enum Hasher {
    private static let sourceExtensions: [String: Set<String>] = [
        "c": [".c", ".h", ".s", ".S"],
        "cpp": [".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".hxx"],
        "csharp": [".cs", ".csproj"],
        "dart": [".dart", ".yaml"],
        "python": [".py", ".toml", ".cfg"],
        "ruby": [".rb", ".gemspec"],
        "go": [".go"],
        "typescript": [".ts", ".tsx", ".js", ".mjs", ".cjs", ".json"],
        "rust": [".rs", ".toml"],
        "elixir": [".ex", ".exs"],
        "lua": [".lua", ".rockspec"],
        "perl": [".pl", ".pm", ".t", ".xs"],
        "swift": [".swift"],
        "fsharp": [".fs", ".fsi", ".fsx", ".fsproj"],
        "haskell": [".hs", ".cabal"],
        "java": [".java"],
        "kotlin": [".kt", ".kts"],
        "mosaic": [".msl", ".mll", ".mil", ".rs", ".toml"],
        "ocaml": [".ml", ".mli", ".opam"],
        "starlark": [".star"],
        "twig": [".tw"],
        "wasm": [".rs", ".toml", ".wat"],
        "dotnet": [".cs", ".fs", ".fsi", ".fsx", ".csproj", ".fsproj"],
    ]

    private static let specialFilenames: [String: Set<String>] = [
        "c": ["CMakeLists.txt", "meson.build"],
        "cpp": ["CMakeLists.txt", "meson.build"],
        "csharp": ["global.json", "NuGet.Config", "nuget.config"],
        "dart": ["pubspec.yaml", "pubspec.lock", "analysis_options.yaml"],
        "python": ["pyproject.toml", "setup.py", "setup.cfg"],
        "ruby": ["Gemfile", "Rakefile"],
        "go": ["go.mod", "go.sum"],
        "typescript": ["package.json", "package-lock.json", "tsconfig.json", "vitest.config.ts"],
        "rust": ["Cargo.toml", "Cargo.lock"],
        "elixir": ["mix.exs", "mix.lock"],
        "lua": [],
        "perl": ["Makefile.PL", "Build.PL", "cpanfile", "MANIFEST", "META.json", "META.yml"],
        "swift": ["Package.swift"],
        "fsharp": ["global.json", "NuGet.Config", "nuget.config"],
        "haskell": ["cabal.project"],
        "java": ["settings.gradle.kts", "build.gradle.kts", "gradle.properties"],
        "kotlin": ["settings.gradle.kts", "build.gradle.kts", "gradle.properties"],
        "mosaic": ["Cargo.toml", "Cargo.lock"],
        "ocaml": [".ocamlformat", "dune", "dune-project"],
        "starlark": [],
        "twig": [],
        "wasm": ["Cargo.toml", "Cargo.lock"],
        "dotnet": ["global.json", "NuGet.Config", "nuget.config"],
    ]

    /// Variable-name package manifests remain inputs in strict declared-source
    /// mode, but only at the package root. Nested project manifests describe
    /// other package identities unless a Starlark `srcs` glob opts them in.
    private static let declaredManifestExtensions: [String: Set<String>] = [
        "ruby": [".gemspec"],
        "lua": [".rockspec"],
        "haskell": [".cabal"],
        "ocaml": [".opam"],
        "csharp": [".csproj"],
        "fsharp": [".fsproj"],
        "dotnet": [".csproj", ".fsproj"],
    ]

    /// Source hashing has a deliberately separate registry from discovery:
    /// `specs` is not generated output, while these 26 exact components are.
    private static let skippedSourceDirectories: Set<String> = [
        ".git", ".hg", ".svn", ".venv", ".tox", ".mypy_cache",
        ".pytest_cache", ".ruff_cache", ".stack-work", "__pycache__",
        "node_modules", "vendor", "dist", "dist-newstyle", "_build",
        "build", "target", ".claude", "Pods", ".gradle", ".dart_tool",
        "gradle-build", "deps", ".build", ".cargo", "cover",
    ]

    public static func hashPackage(
        _ package: BuildPackage,
        repositoryRoot: String
    ) throws -> String {
        do {
            let packageRoot = try repositoryPackagePath(
                package.path,
                repositoryRoot: repositoryRoot
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
            for file in files {
                let relative = try portableRelativePath(file, root: package.path)
                let repositoryPath = packageRoot + "/" + relative
                try validatePortablePath(repositoryPath)
                let snapshot = try secureFileSnapshot(
                    file,
                    repositoryRoot: repositoryRoot
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

    private static func collectSourceInputs(
        _ package: BuildPackage,
        repositoryRoot: String?
    ) throws -> (
        files: [String],
        directoryStates: [(String, SecureObjectState)]
    ) {
        let root = package.path
        let fm = FileManager.default
        var files: [String] = []
        var directoryStates: [(String, SecureObjectState)] = []

        let extensions = sourceExtensions[package.language] ?? []
        let specials = specialFilenames[package.language] ?? []
        let manifestExtensions = declaredManifestExtensions[package.language] ?? []

        guard try entryKind(root) == .directory else {
            throw SourceHashInputError.unavailable
        }

        func visit(directory: String, relativeDirectory: String) throws {
            let before = try repositoryRoot.map {
                try secureDirectoryState(directory, repositoryRoot: $0)
            }
            let entries = try fm.contentsOfDirectory(atPath: directory)

            for entry in entries.sorted(by: utf8LessThan) {
                let relativePath = relativeDirectory.isEmpty
                    ? entry
                    : "\(relativeDirectory)/\(entry)"
                let fullPath = (directory as NSString).appendingPathComponent(entry)
                switch try entryKind(fullPath) {
                case .linked:
                    continue
                case .directory:
                    if !skippedSourceDirectories.contains(entry) {
                        try visit(directory: fullPath, relativeDirectory: relativePath)
                    }
                    continue
                case .other:
                    continue
                case .regular:
                    break
                }

                let normalized = try portableCandidatePath(relativePath)
                let filename = (normalized as NSString).lastPathComponent
                if isBuildFile(filename) {
                    files.append(fullPath)
                    continue
                }

                let fileExtension = (filename as NSString).pathExtension.isEmpty
                    ? ""
                    : ".\((filename as NSString).pathExtension)"
                if specials.contains(filename)
                    || (relativeDirectory.isEmpty
                        && manifestExtensions.contains(fileExtension)) {
                    files.append(fullPath)
                    continue
                }

                if package.isStarlark && !package.declaredSrcs.isEmpty {
                    if package.declaredSrcs.contains(where: { GlobMatch.matchPath($0, normalized) }) {
                        files.append(fullPath)
                    }
                    continue
                }

                if extensions.contains(fileExtension) {
                    files.append(fullPath)
                    continue
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

    private static func isBuildFile(_ filename: String) -> Bool {
        ["BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows", "BUILD_mac_and_linux"].contains(filename)
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
        repositoryRoot: String
    ) throws -> String {
        let relative = try portableRelativePath(path, root: repositoryRoot)
        let components = relative.split(separator: "/").map(String.init)
        guard components.count >= 3,
              components[0] == "code",
              components[1] == "packages" || components[1] == "programs" else {
            throw SourceHashInputError.unsafePath
        }
        try validatePortablePath(relative)
        return relative
    }

    private static func validatePortablePath(_ path: String) throws {
        guard !path.isEmpty,
              !path.hasPrefix("/"),
              !path.contains("\\"),
              !path.unicodeScalars.contains(where: { $0.value == 0 }) else {
            throw SourceHashInputError.unsafePath
        }
        for component in path.split(separator: "/", omittingEmptySubsequences: false)
            where component.isEmpty || component == "." || component == ".." {
            throw SourceHashInputError.unsafePath
        }
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
            var data = Data()
            var buffer = [UInt8](repeating: 0, count: 64 * 1024)
            while true {
                var count = DWORD(0)
                let succeeded = buffer.withUnsafeMutableBytes { bytes in
                    ReadFile(handle, bytes.baseAddress, DWORD(bytes.count), &count, nil)
                }
                guard succeeded else {
                    throw SourceHashInputError.unavailable
                }
                if count == 0 {
                    break
                }
                data.append(contentsOf: buffer.prefix(Int(count)))
            }
            guard UInt64(data.count) == before.size else {
                throw SourceHashInputError.unstable
            }
            return SecureFileSnapshot(data: data, state: before)
        }
        #else
        return try withSecurePOSIXObject(
            path,
            repositoryRoot: repositoryRoot,
            expectDirectory: false
        ) { descriptor, before in
            var data = Data()
            var buffer = [UInt8](repeating: 0, count: 64 * 1024)
            while true {
                let count = buffer.withUnsafeMutableBytes { bytes in
                    read(descriptor, bytes.baseAddress, bytes.count)
                }
                if count < 0 {
                    if errno == EINTR {
                        continue
                    }
                    throw SourceHashInputError.unavailable
                }
                if count == 0 {
                    break
                }
                data.append(contentsOf: buffer.prefix(Int(count)))
            }
            guard UInt64(data.count) == before.size else {
                throw SourceHashInputError.unstable
            }
            return SecureFileSnapshot(data: data, state: before)
        }
        #endif
    }

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
