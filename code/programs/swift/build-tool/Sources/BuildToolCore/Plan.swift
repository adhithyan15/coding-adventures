import Foundation

public enum PlanIO {
    public static let currentSchemaVersion = 1

    public static func writePlan(_ plan: BuildPlan, to path: String) throws {
        var mutablePlan = plan
        mutablePlan.schemaVersion = currentSchemaVersion

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(mutablePlan)

        let url = URL(fileURLWithPath: path)
        let tempURL = url.deletingLastPathComponent().appendingPathComponent(url.lastPathComponent + ".tmp")
        try data.write(to: tempURL)
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
        try FileManager.default.moveItem(at: tempURL, to: url)
    }

    public static func readPlan(from path: String) throws -> BuildPlan {
        let url = URL(fileURLWithPath: path)
        guard let data = try? Data(contentsOf: url) else {
            throw BuildToolError.invalidPlan("build plan not found: \(path)")
        }
        let decoder = JSONDecoder()
        let plan = try decoder.decode(BuildPlan.self, from: data)
        if plan.schemaVersion > currentSchemaVersion {
            throw BuildToolError.unsupportedPlanVersion(plan.schemaVersion, currentSchemaVersion)
        }
        return plan
    }
}
