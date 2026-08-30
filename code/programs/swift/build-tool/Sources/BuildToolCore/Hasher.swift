import Foundation
#if canImport(CryptoKit)
import CryptoKit
#endif

public enum Hasher {
    private static let sourceExtensions: [String: Set<String>] = [
        "python": [".py", ".toml", ".cfg"],
        "ruby": [".rb", ".gemspec"],
        "go": [".go"],
        "typescript": [".ts", ".tsx", ".js", ".mjs", ".cjs", ".json"],
        "rust": [".rs", ".toml"],
        "elixir": [".ex", ".exs"],
        "lua": [".lua", ".rockspec"],
        "perl": [".pl", ".pm", ".t", ".xs"],
        "swift": [".swift"],
        "haskell": [".hs", ".cabal"],
    ]

    private static let specialFilenames: [String: Set<String>] = [
        "python": [],
        "ruby": ["Gemfile", "Rakefile"],
        "go": ["go.mod", "go.sum"],
        "typescript": ["package.json", "package-lock.json", "tsconfig.json", "vitest.config.ts"],
        "rust": ["Cargo.toml", "Cargo.lock"],
        "elixir": ["mix.exs", "mix.lock"],
        "lua": [],
        "perl": ["Makefile.PL", "Build.PL", "cpanfile", "MANIFEST", "META.json", "META.yml"],
        "swift": ["Package.swift"],
        "haskell": ["cabal.project"],
    ]

    public static func hashPackage(_ package: BuildPackage) -> String {
        let files = collectSourceFiles(package)
        if files.isEmpty {
            return hash(data: Data())
        }

        let combined = files.compactMap { try? Data(contentsOf: URL(fileURLWithPath: $0)) }
            .map(hash(data:))
            .joined()
        return hash(string: combined)
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

    public static func collectSourceFiles(_ package: BuildPackage) -> [String] {
        let root = package.path
        let fm = FileManager.default
        var files: [String] = []

        guard let enumerator = fm.enumerator(atPath: root) else {
            return []
        }

        let extensions = sourceExtensions[package.language] ?? []
        let specials = specialFilenames[package.language] ?? []

        while let entry = enumerator.nextObject() as? String {
            let normalized = entry.replacingOccurrences(of: "\\", with: "/")
            let fullPath = (root as NSString).appendingPathComponent(entry)
            var isDirectory: ObjCBool = false
            if fm.fileExists(atPath: fullPath, isDirectory: &isDirectory), isDirectory.boolValue {
                if Discovery.skipDirectories.contains((normalized as NSString).lastPathComponent) {
                    enumerator.skipDescendants()
                }
                continue
            }

            let filename = (normalized as NSString).lastPathComponent
            if isBuildFile(filename) {
                files.append(fullPath)
                continue
            }

            if package.isStarlark, !package.declaredSrcs.isEmpty {
                if package.declaredSrcs.contains(where: { GlobMatch.matchPath($0, normalized) }) {
                    files.append(fullPath)
                }
                continue
            }

            if extensions.contains((filename as NSString).pathExtension.isEmpty ? "" : ".\((filename as NSString).pathExtension)") {
                files.append(fullPath)
                continue
            }

            if specials.contains(filename) {
                files.append(fullPath)
            }
        }

        return files.sorted {
            relativePath($0, root: root) < relativePath($1, root: root)
        }
    }

    private static func isBuildFile(_ filename: String) -> Bool {
        ["BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows", "BUILD_mac_and_linux"].contains(filename)
    }

    private static func relativePath(_ path: String, root: String) -> String {
        let normalizedPath = path.replacingOccurrences(of: "\\", with: "/")
        let normalizedRoot = root.replacingOccurrences(of: "\\", with: "/")
        if normalizedPath.hasPrefix(normalizedRoot + "/") {
            return String(normalizedPath.dropFirst(normalizedRoot.count + 1))
        }
        return normalizedPath
    }

    private static func hash(string: String) -> String {
        hash(data: Data(string.utf8))
    }

    private static func hash(data: Data) -> String {
        #if canImport(CryptoKit)
        if #available(macOS 10.15, *) {
            let digest = SHA256.hash(data: data)
            return digest.map { String(format: "%02x", $0) }.joined()
        }
        #else
        #endif
        var value: UInt64 = 14_695_981_039_346_656_037
        for byte in data {
            value ^= UInt64(byte)
            value &*= 1_099_511_628_211
        }
        return String(format: "%016llx", value)
    }
}
