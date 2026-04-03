import Foundation

public enum Reporter {
    private static let statusDisplay: [BuildStatus: String] = [
        .built: "BUILT",
        .failed: "FAILED",
        .skipped: "SKIPPED",
        .depSkipped: "DEP-SKIP",
        .wouldBuild: "WOULD-BUILD",
    ]

    public static func formatReport(results: [String: PackageBuildResult]) -> String {
        var output = "\nBuild Report\n============\n"

        guard !results.isEmpty else {
            return output + "No packages processed.\n"
        }

        let maxNameLength = max("Package".count, results.keys.map(\.count).max() ?? 0)
        output += "\(pad("Package", to: maxNameLength))   \(pad("Status", to: 12)) Duration\n"

        for name in results.keys.sorted() {
            let result = results[name]!
            let status = statusDisplay[result.status] ?? result.status.rawValue.uppercased()
            let duration = result.status == .depSkipped ? "- (dep failed)" : formatDuration(result.duration)
            output += "\(pad(name, to: maxNameLength))   \(pad(status, to: 12)) \(duration)\n"
        }

        let total = results.count
        let built = results.values.filter { $0.status == .built }.count
        let skipped = results.values.filter { $0.status == .skipped }.count
        let failed = results.values.filter { $0.status == .failed }.count
        let depSkipped = results.values.filter { $0.status == .depSkipped }.count
        let wouldBuild = results.values.filter { $0.status == .wouldBuild }.count

        output += "\nTotal: \(total) packages"
        if built > 0 {
            output += " | \(built) built"
        }
        if skipped > 0 {
            output += " | \(skipped) skipped"
        }
        if failed > 0 {
            output += " | \(failed) failed"
        }
        if depSkipped > 0 {
            output += " | \(depSkipped) dep-skipped"
        }
        if wouldBuild > 0 {
            output += " | \(wouldBuild) would-build"
        }
        output += "\n"

        return output
    }

    private static func formatDuration(_ duration: TimeInterval) -> String {
        if duration < 0.01 {
            return "-"
        }
        return String(format: "%.1fs", duration)
    }

    private static func pad(_ value: String, to width: Int) -> String {
        if value.count >= width {
            return value
        }
        return value + String(repeating: " ", count: width - value.count)
    }
}
