$ErrorActionPreference = 'Stop'

$microRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rankRoot = (Resolve-Path (Join-Path $microRoot '..\rank-statistics')).Path.Replace('\', '/')
$microPackage = $microRoot.Replace('\', '/')
$scratch = Join-Path $env:TEMP ("swift-microbenchmark-consumer-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
New-Item -ItemType Directory -Path (Join-Path $scratch 'Sources\Consumer') -Force | Out-Null

$manifest = @"
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Consumer",
    dependencies: [
        .package(path: "$rankRoot"),
        .package(path: "$microPackage"),
    ],
    targets: [
        .executableTarget(name: "Consumer", dependencies: [
            .product(name: "RankStatistics", package: "rank-statistics"),
            .product(name: "Microbenchmark", package: "microbenchmark"),
        ]),
    ]
)
"@
$utf8 = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText((Join-Path $scratch 'Package.swift'), $manifest, $utf8)

$consumerSource = @'
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
        guard report.pairs.count == 40,
              report.pairs.allSatisfy({ $0.baselineNs > 0 && $0.candidateNs > 0 }) else {
            throw NSError(domain: "Consumer", code: 2)
        }
        print("external consumer replayed 40 positive pairs")
    }
}
'@
[System.IO.File]::WriteAllText((Join-Path $scratch 'Sources\Consumer\main.swift'), $consumerSource, $utf8)

Push-Location $microRoot
try {
    swift build -c release --product array-microbenchmark
    if ($LASTEXITCODE -ne 0) { throw 'release array-microbenchmark build failed' }
    $binPath = (& swift build -c release --show-bin-path | Select-Object -Last 1).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'could not locate release binary directory' }
    $executable = Join-Path $binPath 'array-microbenchmark.exe'
    $reportPath = Join-Path $scratch 'native-report.json'
    $reportLines = & $executable --pairs 40 --target-ns 1000 --revision swift-consumer-smoke
    if ($LASTEXITCODE -ne 0) { throw 'native 40-pair CLI smoke failed' }
    [System.IO.File]::WriteAllText($reportPath, (($reportLines -join "`n") + "`n"), $utf8)

    $parsed = Get-Content -Raw -LiteralPath $reportPath | ConvertFrom-Json
    if ($parsed.pairs.Count -ne 40) { throw 'native report did not contain 40 pairs' }
    foreach ($pair in $parsed.pairs) {
        if ($pair.baseline_ns -le 0 -or $pair.candidate_ns -le 0) {
            throw 'native report contained a nonpositive duration'
        }
    }

    swift build --package-path $scratch -c release
    if ($LASTEXITCODE -ne 0) { throw 'external SwiftPM consumer build failed' }
    $consumerBinPath = (& swift build --package-path $scratch -c release --show-bin-path | Select-Object -Last 1).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'could not locate external consumer binary directory' }
    & (Join-Path $consumerBinPath 'Consumer.exe') $reportPath
    if ($LASTEXITCODE -ne 0) { throw 'external consumer failed to replay native report' }
} finally {
    Pop-Location
}
