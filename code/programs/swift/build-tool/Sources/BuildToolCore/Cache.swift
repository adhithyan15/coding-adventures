import Foundation

public final class BuildCache {
    public private(set) var entries: [String: CacheEntry] = [:]

    public init() {}

    public func load(from path: String) {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: path)) else {
            entries = [:]
            return
        }

        let decoder = JSONDecoder()
        guard let decoded = try? decoder.decode([String: CacheEntry].self, from: data) else {
            entries = [:]
            return
        }
        entries = decoded
    }

    public func save(to path: String) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(entries)
        let url = URL(fileURLWithPath: path)
        let tempURL = url.deletingLastPathComponent().appendingPathComponent(url.lastPathComponent + ".tmp")
        try data.write(to: tempURL)
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
        try FileManager.default.moveItem(at: tempURL, to: url)
    }

    public func needsBuild(name: String, packageHash: String, depsHash: String) -> Bool {
        guard let entry = entries[name] else {
            return true
        }
        return entry.status == "failed"
            || entry.packageHash != packageHash
            || entry.depsHash != depsHash
    }

    public func record(name: String, packageHash: String, depsHash: String, status: String) {
        entries[name] = CacheEntry(
            packageHash: packageHash,
            depsHash: depsHash,
            lastBuilt: ISO8601DateFormatter().string(from: Date()),
            status: status
        )
    }
}
