import Foundation

public enum Validator {
    public static let ciManagedToolchainLanguages: Set<String> = [
        "python",
        "ruby",
        "typescript",
        "rust",
        "elixir",
        "lua",
        "perl",
    ]

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
}
