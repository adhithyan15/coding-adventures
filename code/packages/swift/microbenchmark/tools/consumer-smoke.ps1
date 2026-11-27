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
'@
[System.IO.File]::WriteAllText((Join-Path $scratch 'Sources\Consumer\main.swift'), $consumerSource, $utf8)

Push-Location $microRoot
try {
    swift build -c release --product array-microbenchmark
    if ($LASTEXITCODE -ne 0) { throw 'release array-microbenchmark build failed' }
    $binPath = (& swift build -c release --show-bin-path | Select-Object -Last 1).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'could not locate release binary directory' }
    $executable = Join-Path $binPath 'array-microbenchmark.exe'
    swift build --package-path $scratch -c release
    if ($LASTEXITCODE -ne 0) { throw 'external SwiftPM consumer build failed' }
    $consumerBinPath = (& swift build --package-path $scratch -c release --show-bin-path | Select-Object -Last 1).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'could not locate external consumer binary directory' }
    $consumer = Join-Path $consumerBinPath 'Consumer.exe'
    $savedGitRevision = [Environment]::GetEnvironmentVariable('GIT_REVISION', 'Process')
    $savedGithubSha = [Environment]::GetEnvironmentVariable('GITHUB_SHA', 'Process')
    $newline = [Environment]::NewLine
    try {
        $env:GIT_REVISION = 'git-revision-smoke'
        $env:GITHUB_SHA = 'github-sha-smoke'
        $reportLines = & $executable --pairs 40 --target-ns 1000
        if ($LASTEXITCODE -ne 0) { throw 'native primary-revision CLI smoke failed' }
        $reportPath = Join-Path $scratch 'native-report.json'
        [System.IO.File]::WriteAllText($reportPath, (($reportLines -join $newline) + $newline), $utf8)
        & $consumer $reportPath git-revision-smoke
        if ($LASTEXITCODE -ne 0) { throw 'external consumer rejected primary revision' }

        Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue
        Remove-Item Env:GITHUB_SHA -ErrorAction SilentlyContinue
        $reportLines = & $executable --pairs 40 --target-ns 1000
        if ($LASTEXITCODE -ne 0) { throw 'native no-environment CLI smoke failed' }
        $noEnvReportPath = Join-Path $scratch 'no-env-report.json'
        [System.IO.File]::WriteAllText($noEnvReportPath, (($reportLines -join $newline) + $newline), $utf8)
        & $consumer $noEnvReportPath unknown
        if ($LASTEXITCODE -ne 0) { throw 'external consumer rejected missing-environment default' }

        # Build this child environment explicitly. PowerShell versions before
        # 7.5 may interpret assigning '' through Env: as removal; an explicit
        # process environment block preserves the present-empty case.
        $emptyPrimaryStart = New-Object System.Diagnostics.ProcessStartInfo
        $emptyPrimaryStart.FileName = $executable
        $emptyPrimaryStart.Arguments = '--pairs 40 --target-ns 1000'
        $emptyPrimaryStart.UseShellExecute = $false
        $emptyPrimaryStart.RedirectStandardOutput = $true
        $emptyPrimaryStart.RedirectStandardError = $true
        $emptyPrimaryStart.EnvironmentVariables['GIT_REVISION'] = ''
        $emptyPrimaryStart.EnvironmentVariables['GITHUB_SHA'] = 'github-sha-smoke'
        $emptyPrimaryProcess = New-Object System.Diagnostics.Process
        $emptyPrimaryProcess.StartInfo = $emptyPrimaryStart
        if (-not $emptyPrimaryProcess.Start()) { throw 'could not start empty-primary CLI smoke' }
        $emptyPrimaryOutput = $emptyPrimaryProcess.StandardOutput.ReadToEnd()
        $emptyPrimaryError = $emptyPrimaryProcess.StandardError.ReadToEnd()
        $emptyPrimaryProcess.WaitForExit()
        if ($emptyPrimaryProcess.ExitCode -ne 0) {
            throw "native empty-primary CLI smoke failed: $emptyPrimaryError"
        }
        if ($emptyPrimaryOutput -notmatch '"revision":"unknown"' -or
            $emptyPrimaryOutput -match '"revision":"github-sha-smoke"') {
            throw 'present-empty GIT_REVISION did not suppress the nonempty fallback'
        }
        $emptyPrimaryReportPath = Join-Path $scratch 'empty-primary-report.json'
        [System.IO.File]::WriteAllText($emptyPrimaryReportPath, $emptyPrimaryOutput, $utf8)
        & $consumer $emptyPrimaryReportPath unknown
        if ($LASTEXITCODE -ne 0) { throw 'external consumer rejected present-empty primary handling' }

        Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue
        $env:GITHUB_SHA = 'github-sha-smoke'
        $reportLines = & $executable --pairs 40 --target-ns 1000
        if ($LASTEXITCODE -ne 0) { throw 'native fallback-revision CLI smoke failed' }
        $fallbackReportPath = Join-Path $scratch 'fallback-report.json'
        [System.IO.File]::WriteAllText($fallbackReportPath, (($reportLines -join $newline) + $newline), $utf8)
        & $consumer $fallbackReportPath github-sha-smoke
        if ($LASTEXITCODE -ne 0) { throw 'external consumer rejected fallback revision' }

        $unicodeRevision = 'revision-π-🚀'
        $reportLines = & $executable --pairs 40 --target-ns 1000 --revision $unicodeRevision
        if ($LASTEXITCODE -ne 0) { throw 'native CLI override smoke failed' }
        $overrideReportPath = Join-Path $scratch 'override-report.json'
        [System.IO.File]::WriteAllText($overrideReportPath, (($reportLines -join $newline) + $newline), $utf8)
        & $consumer $overrideReportPath $unicodeRevision
        if ($LASTEXITCODE -ne 0) { throw 'external consumer rejected CLI revision override' }
    } finally {
        if ($null -eq $savedGitRevision) { Remove-Item Env:GIT_REVISION -ErrorAction SilentlyContinue }
        else { $env:GIT_REVISION = $savedGitRevision }
        if ($null -eq $savedGithubSha) { Remove-Item Env:GITHUB_SHA -ErrorAction SilentlyContinue }
        else { $env:GITHUB_SHA = $savedGithubSha }
    }
} finally {
    Pop-Location
}
