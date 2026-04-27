import Foundation

public enum Discovery {
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
        "Pods",
        ".build",
    ]

    public static func discoverPackages(root: String) -> [BuildPackage] {
        var packages: [BuildPackage] = []
        walk(directory: root, packages: &packages)
        return packages.sorted { $0.name < $1.name }
    }

    public static func inferLanguage(path: String) -> String {
        let parts = path
            .replacingOccurrences(of: "\\", with: "/")
            .split(separator: "/")
            .map(String.init)
        for language in allPackageLanguages where parts.contains(language) {
            return language
        }
        return "unknown"
    }

    public static func inferPackageName(path: String, language: String) -> String {
        "\(language)/\((path as NSString).lastPathComponent)"
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
