#!/usr/bin/env bash
set -euo pipefail

micro_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rank_root="$(cd "$micro_root/../rank-statistics" && pwd)"
scratch="$(mktemp -d)"
mkdir -p "$scratch/Sources/Consumer"

cat > "$scratch/Package.swift" <<EOF
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Consumer",
    dependencies: [
        .package(path: "$rank_root"),
        .package(path: "$micro_root"),
    ],
    targets: [
        .executableTarget(name: "Consumer", dependencies: [
            .product(name: "RankStatistics", package: "rank-statistics"),
            .product(name: "Microbenchmark", package: "microbenchmark"),
        ]),
    ]
)
EOF

cat > "$scratch/Sources/Consumer/main.swift" <<'SWIFT'
import Foundation
import Microbenchmark
import RankStatistics

@main
struct Consumer {
    static func main() throws {
        guard try median([1.0, 2.0, 3.0]) == 2.0 else {
            throw NSError(domain: "Consumer", code: 1)
        }
        let json = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
        let report = try replayReport(json)
        let expectedRevision = CommandLine.arguments[2] == "--expect-empty"
            ? ""
            : CommandLine.arguments[2]
        guard report.pairs.count == 40,
              report.revision == expectedRevision,
              report.pairs.allSatisfy({ $0.baselineNs > 0 && $0.candidateNs > 0 }) else {
            throw NSError(domain: "Consumer", code: 2)
        }
        print("external consumer replayed 40 positive pairs")
    }
}
SWIFT

swift build --package-path "$micro_root" -c release --product array-microbenchmark
bin_path="$(swift build --package-path "$micro_root" -c release --show-bin-path | tail -n 1)"
report="$scratch/native-report.json"
env GIT_REVISION=git-revision-smoke GITHUB_SHA=github-sha-smoke \
  "$bin_path/array-microbenchmark" --pairs 40 --target-ns 1000 > "$report"
swift build --package-path "$scratch" -c release
consumer_bin_path="$(swift build --package-path "$scratch" -c release --show-bin-path | tail -n 1)"
no_env_report="$scratch/no-env-report.json"
env -u GIT_REVISION -u GITHUB_SHA \
  "$bin_path/array-microbenchmark" --pairs 40 --target-ns 1000 > "$no_env_report"
"$consumer_bin_path/Consumer" "$no_env_report" unknown
"$consumer_bin_path/Consumer" "$report" git-revision-smoke

fallback_report="$scratch/fallback-report.json"
env -u GIT_REVISION GITHUB_SHA=github-sha-smoke \
  "$bin_path/array-microbenchmark" --pairs 40 --target-ns 1000 > "$fallback_report"
"$consumer_bin_path/Consumer" "$fallback_report" github-sha-smoke

empty_primary_report="$scratch/empty-primary-report.json"
env GIT_REVISION= GITHUB_SHA=github-sha-smoke \
  "$bin_path/array-microbenchmark" --pairs 40 --target-ns 1000 > "$empty_primary_report"
"$consumer_bin_path/Consumer" "$empty_primary_report" unknown

override_report="$scratch/override-report.json"
env GIT_REVISION=git-revision-smoke GITHUB_SHA=github-sha-smoke \
  "$bin_path/array-microbenchmark" --pairs 40 --target-ns 1000 --revision 'revision-π-🚀' > "$override_report"
"$consumer_bin_path/Consumer" "$override_report" 'revision-π-🚀'
