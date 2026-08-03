import Foundation

public enum Discovery {
    public static let languages: Set<String> = [
        "csharp",
        "dart",
        "elixir",
        "fsharp",
        "go",
        "haskell",
        "java",
        "kotlin",
        "lua",
        "perl",
        "python",
        "ruby",
        "rust",
        "swift",
        "typescript",
        "c",
        "cpp",
        "ocaml",
        "wasm",
        "mosaic",
        "twig",
        "starlark",
        "dotnet",
    ]

    public static let skipDirectories: Set<String> = [
        ".git",
        ".hg",
        ".svn",
        ".venv",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        "__pycache__",
        "node_modules",
        "vendor",
        "dist",
        "build",
        "target",
        ".claude",
        "specs",
        "Pods",
        ".dart_tool",
        ".build",
        ".gradle",
        "gradle-build",
    ]

    public static func discoverPackages(root: String) throws -> [BuildPackage] {
        var packages: [BuildPackage] = []
        walk(directory: root, packages: &packages)
        packages.sort {
            $0.name == $1.name ? $0.path < $1.path : $0.name < $1.name
        }

        var index = 0
        while index < packages.count {
            var end = index + 1
            while end < packages.count,
                  packages[end].name == packages[index].name {
                end += 1
            }
            if end - index > 1 {
                throw DuplicatePackageIdentityError(
                    code: "DUPLICATE_PACKAGE_IDENTITY",
                    package: packages[index].name,
                    paths: packages[index ..< end].map {
                        repositoryPackagePath(root: root, path: $0.path)
                    }
                )
            }
            index = end
        }

        return packages
    }

    public static func inferLanguage(path: String) -> String {
        let parts = path
            .replacingOccurrences(of: "\\", with: "/")
            .split(separator: "/")
            .map(String.init)
        for index in parts.indices
            where parts[index] == "packages" || parts[index] == "programs" {
            guard parts.indices.contains(index + 1) else {
                return "unknown"
            }
            let language = parts[index + 1]
            return languages.contains(language) ? language : "unknown"
        }
        return "unknown"
    }

    public static func inferPackageName(path: String, language: String) -> String {
        let parts = path
            .replacingOccurrences(of: "\\", with: "/")
            .split(separator: "/")
            .map(String.init)
        let kind = parts.contains("programs") ? "/programs" : ""
        return "\(language)\(kind)/\((path as NSString).lastPathComponent)"
    }

    public static func getBuildFile(directory: String, platformOverride: String? = nil) -> String? {
        let platform = platformOverride ?? currentPlatform()
        let fm = FileManager.default

        func existing(_ filename: String) -> String? {
            let path = (directory as NSString).appendingPathComponent(filename)
            return fm.fileExists(atPath: path) ? path : nil
        }

        if platform == "darwin", let path = existing("BUILD_mac") {
            return path
        }

        if platform == "linux", let path = existing("BUILD_linux") {
            return path
        }

        if platform == "windows", let path = existing("BUILD_windows") {
            return path
        }

        if (platform == "darwin" || platform == "linux"), let path = existing("BUILD_mac_and_linux") {
            return path
        }

        return existing("BUILD")
    }

    public static func readLines(filePath: String) -> [String] {
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }
        return content
            .split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty && !$0.hasPrefix("#") }
    }

    private static func repositoryPackagePath(root: String, path: String) -> String {
        let normalizedPath = path.replacingOccurrences(of: "\\", with: "/")
        let parts = normalizedPath.split(separator: "/").map(String.init)
        var canonicalStart: Int?
        if parts.count >= 2 {
            for index in 0 ..< parts.count - 1
                where parts[index] == "code"
                    && (parts[index + 1] == "packages" || parts[index + 1] == "programs") {
                canonicalStart = index
            }
        }
        if let canonicalStart {
            return parts[canonicalStart...].joined(separator: "/")
        }

        let normalizedRoot = root.replacingOccurrences(of: "\\", with: "/")
        if normalizedPath.hasPrefix(normalizedRoot + "/") {
            return String(normalizedPath.dropFirst(normalizedRoot.count + 1))
        }
        return (path as NSString).lastPathComponent
    }

    private static func walk(directory: String, packages: inout [BuildPackage]) {
        let fm = FileManager.default
        let normalizedDirectory = URL(fileURLWithPath: directory).standardizedFileURL.path
        let directoryName = (normalizedDirectory as NSString).lastPathComponent
        if skipDirectories.contains(directoryName) {
            return
        }

        if let buildFile = getBuildFile(directory: normalizedDirectory) {
            let commands = readLines(filePath: buildFile)
            let content = (try? String(contentsOfFile: buildFile, encoding: .utf8)) ?? ""
            let language = inferLanguage(path: normalizedDirectory)
            let name = inferPackageName(path: normalizedDirectory, language: language)
            packages.append(
                BuildPackage(
                    name: name,
                    path: normalizedDirectory,
                    buildCommands: commands,
                    language: language,
                    buildContent: content
                )
            )
            return
        }

        guard let entries = try? fm.contentsOfDirectory(
            atPath: normalizedDirectory
        ) else {
            return
        }

        for entry in entries.sorted() {
            let path = (normalizedDirectory as NSString).appendingPathComponent(entry)
            var isDirectory: ObjCBool = false
            if fm.fileExists(atPath: path, isDirectory: &isDirectory), isDirectory.boolValue {
                walk(directory: path, packages: &packages)
            }
        }
    }

    private static func currentPlatform() -> String {
        #if os(macOS)
        return "darwin"
        #elseif os(Linux)
        return "linux"
        #elseif os(Windows)
        return "windows"
        #else
        return "unknown"
        #endif
    }
}
