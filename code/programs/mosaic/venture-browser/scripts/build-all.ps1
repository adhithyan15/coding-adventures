param(
    [string]$Output = "target/mosaic-venture-browser",
    [switch]$Release,
    [switch]$EmitOnly,
    [switch]$Strict
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$packageRoot = (Resolve-Path (Join-Path $scriptRoot "..")).Path
$rustWorkspace = (Resolve-Path (Join-Path $packageRoot "..\..\..\packages\rust")).Path
$outputRoot = if ([System.IO.Path]::IsPathRooted($Output)) {
    [System.IO.Path]::GetFullPath($Output)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $packageRoot $Output))
}
$backends = @("react", "electron", "swiftui", "qt", "webcomponent", "html", "xaml", "flutter", "compose")
$skipped = [System.Collections.Generic.List[string]]::new()
$deferred = [System.Collections.Generic.List[string]]::new()

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

function Test-Command {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Skip-Backend {
    param(
        [Parameter(Mandatory = $true)][string]$Backend,
        [Parameter(Mandatory = $true)][string]$Reason
    )

    Write-Host "==> Skipping ${Backend}: $Reason"
    $skipped.Add("$Backend ($Reason)")
}

function Defer-Backend {
    param(
        [Parameter(Mandatory = $true)][string]$Backend,
        [Parameter(Mandatory = $true)][string]$Reason
    )

    Write-Host "==> Deferring ${Backend}: $Reason"
    $deferred.Add("$Backend ($Reason)")
}

foreach ($backend in $backends) {
    Write-Host "==> Emitting $backend"
    $cargoArgs = @("run", "-q", "-p", "mosaic-compile")
    if ($Release) {
        $cargoArgs += "--release"
    }
    $cargoArgs += @(
        "--",
        "pkg",
        $packageRoot,
        "--backend",
        $backend,
        "--output",
        $outputRoot,
        "--emit-project",
        "--theme",
        "light"
    )
    Push-Location $rustWorkspace
    try {
        Invoke-Checked -Command "cargo" -Arguments $cargoArgs
    } finally {
        Pop-Location
    }
}

if ($EmitOnly) {
    Write-Host "Emitted $($backends.Count) Venture backend projects under $outputRoot"
    exit 0
}

foreach ($backend in @("react", "electron")) {
    if (-not (Test-Command "npm")) {
        Skip-Backend -Backend $backend -Reason "npm is not installed"
        continue
    }
    Write-Host "==> Building $backend"
    Push-Location (Join-Path $outputRoot $backend)
    try {
        Invoke-Checked -Command "npm" -Arguments @("install", "--ignore-scripts")
        Invoke-Checked -Command "npm" -Arguments @("run", "build")
    } finally {
        Pop-Location
    }
}

if (Test-Command "node") {
    Write-Host "==> Checking html"
    Push-Location (Join-Path $outputRoot "html")
    try {
        Invoke-Checked -Command "node" -Arguments @("--check", "main.js")
    } finally {
        Pop-Location
    }

    Write-Host "==> Checking webcomponent"
    Push-Location (Join-Path $outputRoot "webcomponent")
    try {
        foreach ($source in @("VentureChrome.js", "index.js", "main.js")) {
            Invoke-Checked -Command "node" -Arguments @("--check", $source)
        }
    } finally {
        Pop-Location
    }
} else {
    Skip-Backend -Backend "html" -Reason "node is not installed"
    Skip-Backend -Backend "webcomponent" -Reason "node is not installed"
}

$isWindows = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)
$isMacOS = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::OSX)
$isLinux = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Linux)

if ($isMacOS -and (Test-Command "swift")) {
    Write-Host "==> Building Venture macOS native bridge"
    $bridgeArgs = @("build", "-p", "venture-browser-macos")
    $bridgeProfile = "debug"
    if ($Release) {
        $bridgeArgs += "--release"
        $bridgeProfile = "release"
    }
    Push-Location $rustWorkspace
    try {
        Invoke-Checked -Command "cargo" -Arguments $bridgeArgs
    } finally {
        Pop-Location
    }
    Copy-Item -Force `
        (Join-Path $rustWorkspace "target/$bridgeProfile/libventure_browser_macos.dylib") `
        (Join-Path $outputRoot "swiftui/libventure_browser_macos.dylib")
    Write-Host "==> Building swiftui"
    Push-Location (Join-Path $outputRoot "swiftui")
    try {
        Invoke-Checked -Command "swift" -Arguments @("build")
    } finally {
        Pop-Location
    }
} elseif (-not $isMacOS) {
    Defer-Backend -Backend "swiftui" -Reason "SwiftUI builds require macOS"
} else {
    Skip-Backend -Backend "swiftui" -Reason "swift is not installed"
}

if (Test-Command "cmake") {
    Write-Host "==> Building qt"
    Push-Location (Join-Path $outputRoot "qt")
    try {
        if (Test-Command "qt-cmake") {
            Invoke-Checked -Command "qt-cmake" -Arguments @("-S", ".", "-B", "build")
        } else {
            Invoke-Checked -Command "cmake" -Arguments @("-S", ".", "-B", "build")
        }
        Invoke-Checked -Command "cmake" -Arguments @("--build", "build")
    } finally {
        Pop-Location
    }
} else {
    Skip-Backend -Backend "qt" -Reason "cmake is not installed"
}

if ($isWindows -and (Test-Command "dotnet")) {
    Write-Host "==> Building Venture Windows native bridge"
    $bridgeArgs = @("build", "-p", "venture-browser-windows")
    $bridgeProfile = "debug"
    if ($Release) {
        $bridgeArgs += "--release"
        $bridgeProfile = "release"
    }
    Push-Location $rustWorkspace
    try {
        Invoke-Checked -Command "cargo" -Arguments $bridgeArgs
    } finally {
        Pop-Location
    }
    Copy-Item -Force `
        (Join-Path $rustWorkspace "target/$bridgeProfile/venture_browser_windows.dll") `
        (Join-Path $outputRoot "xaml/venture_browser_windows.dll")
    Write-Host "==> Building xaml"
    Push-Location (Join-Path $outputRoot "xaml")
    try {
        Invoke-Checked -Command "dotnet" -Arguments @("build", "VentureChrome.csproj", "-p:Platform=x64")
    } finally {
        Pop-Location
    }
} elseif (-not $isWindows) {
    Defer-Backend -Backend "xaml" -Reason "WinUI builds require Windows"
} else {
    Skip-Backend -Backend "xaml" -Reason "dotnet is not installed"
}

$flutterPlatform = if ($isMacOS) { "macos" } elseif ($isWindows) { "windows" } elseif ($isLinux) { "linux" } else { $null }
if ($null -ne $flutterPlatform -and (Test-Command "flutter")) {
    Write-Host "==> Building flutter ($flutterPlatform)"
    Push-Location (Join-Path $outputRoot "flutter")
    try {
        Invoke-Checked -Command "flutter" -Arguments @("pub", "get")
        Invoke-Checked -Command "flutter" -Arguments @("analyze", "lib")
        if (-not (Test-Path -LiteralPath $flutterPlatform)) {
            Invoke-Checked -Command "flutter" -Arguments @("create", "--platforms=$flutterPlatform", ".")
        }
        Invoke-Checked -Command "flutter" -Arguments @("build", $flutterPlatform)
    } finally {
        Pop-Location
    }
} elseif ($null -eq $flutterPlatform) {
    Defer-Backend -Backend "flutter" -Reason "unsupported host platform"
} else {
    Skip-Backend -Backend "flutter" -Reason "flutter is not installed"
}

$javaMajor = $null
if (Test-Command "java") {
    $javaVersion = (& java -version 2>&1 | Out-String)
    if ($javaVersion -match 'version "([0-9]+)') {
        $javaMajor = [int]$Matches[1]
    }
}
if (-not (Test-Command "gradle")) {
    Skip-Backend -Backend "compose" -Reason "gradle is not installed"
} elseif ($null -eq $javaMajor -or $javaMajor -lt 21) {
    Skip-Backend -Backend "compose" -Reason "JDK 21 or newer is not installed"
} else {
    Write-Host "==> Building compose"
    Push-Location (Join-Path $outputRoot "compose")
    try {
        Invoke-Checked -Command "gradle" -Arguments @("--no-daemon", "build")
    } finally {
        Pop-Location
    }
}

Write-Host "Built or checked $($backends.Count - $skipped.Count - $deferred.Count) of $($backends.Count) Venture backend projects."
if ($deferred.Count -gt 0) {
    Write-Host "Deferred to native hosts: $($deferred -join ', ')"
}
if ($skipped.Count -gt 0) {
    Write-Host "Skipped: $($skipped -join ', ')"
    if ($Strict) {
        throw "Strict mode requires every host-applicable backend gate to run."
    }
}
