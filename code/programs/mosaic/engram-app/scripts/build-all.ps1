param(
    [string]$Output = "target/mosaic-engram-app",
    [switch]$Release
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$packageRoot = (Resolve-Path (Join-Path $scriptRoot "..")).Path
$rustWorkspace = (Resolve-Path (Join-Path $packageRoot "..\..\..\packages\rust")).Path
$engramWasmRoot = (Resolve-Path (Join-Path $rustWorkspace "engram-wasm")).Path
$nativeProfile = if ($Release) { "release" } else { "debug" }
$nativeLibraryName = if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
    "engram_capi.dll"
} elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::OSX)) {
    "libengram_capi.dylib"
} else {
    "libengram_capi.so"
}
$hostCliName = if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
    "engram-host-cli.exe"
} else {
    "engram-host-cli"
}
$staticLibraryName = if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
    "engram_capi.lib"
} else {
    "libengram_capi.a"
}
$engramCapiLibrary = Join-Path $rustWorkspace "target\$nativeProfile\$nativeLibraryName"
$engramCapiStaticLibrary = Join-Path $rustWorkspace "target\$nativeProfile\$staticLibraryName"
$engramHostCli = Join-Path $rustWorkspace "target\$nativeProfile\$hostCliName"

if ([System.IO.Path]::IsPathRooted($Output)) {
    $outputRoot = [System.IO.Path]::GetFullPath($Output)
} else {
    $outputRoot = [System.IO.Path]::GetFullPath((Join-Path $packageRoot $Output))
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

if (-not $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER) {
    $sysroot = (& rustc --print sysroot).Trim()
    $rustLld = Join-Path $sysroot "lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe"
    if (Test-Path -LiteralPath $rustLld) {
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $rustLld
    }
}

Write-Host "==> Building Engram WASM Mosaic host"
& (Join-Path $engramWasmRoot "build-wasm.ps1")
if ($LASTEXITCODE -ne 0) {
    throw "engram-wasm build failed with exit code $LASTEXITCODE"
}

Write-Host "==> Building Engram native C API host"
$capiCargoArgs = @("build", "-p", "engram-capi")
if ($Release) {
    $capiCargoArgs += "--release"
}
Push-Location $rustWorkspace
try {
    & cargo @capiCargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "engram-capi build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$engramWasmFile = Join-Path $engramWasmRoot "pkg\engram_engine.wasm"
$engramWasmLoader = Join-Path $engramWasmRoot "js\engram-mosaic-host-wasm.mjs"
$engramCapiHeader = Join-Path $rustWorkspace "engram-capi\include\engram.h"

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Content
    )
    $encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

function Install-EngramReactHost {
    param([Parameter(Mandatory = $true)][string]$ReactRoot)

    if (-not (Test-Path -LiteralPath $ReactRoot)) {
        return
    }
    $publicDir = Join-Path $ReactRoot "public"
    $srcDir = Join-Path $ReactRoot "src"
    New-Item -ItemType Directory -Force -Path $publicDir, $srcDir | Out-Null
    Copy-Item -LiteralPath $engramWasmFile -Destination (Join-Path $publicDir "engram_engine.wasm") -Force
    Copy-Item -LiteralPath $engramWasmLoader -Destination (Join-Path $srcDir "engram-mosaic-host-wasm.mjs") -Force
}

function Install-EngramHtmlHost {
    param([Parameter(Mandatory = $true)][string]$HtmlRoot)

    if (-not (Test-Path -LiteralPath $HtmlRoot)) {
        return
    }
    Copy-Item -LiteralPath $engramWasmFile -Destination (Join-Path $HtmlRoot "engram_engine.wasm") -Force
    Copy-Item -LiteralPath $engramWasmLoader -Destination (Join-Path $HtmlRoot "engram-mosaic-host-wasm.mjs") -Force
}

function Install-EngramWebComponentHost {
    param([Parameter(Mandatory = $true)][string]$WebComponentRoot)

    Install-EngramHtmlHost -HtmlRoot $WebComponentRoot
}

function Install-EngramElectronHost {
    param([Parameter(Mandatory = $true)][string]$ElectronRoot)

    if (-not (Test-Path -LiteralPath $ElectronRoot)) {
        return
    }
    $electronDir = Join-Path $ElectronRoot "electron"
    New-Item -ItemType Directory -Force -Path $electronDir | Out-Null
    Copy-Item -LiteralPath $engramWasmFile -Destination (Join-Path $electronDir "engram_engine.wasm") -Force
    Copy-Item -LiteralPath $engramWasmLoader -Destination (Join-Path $electronDir "engram-mosaic-host-wasm.mjs") -Force
    Copy-Item -LiteralPath $engramHostCli -Destination (Join-Path $electronDir $hostCliName) -Force
}

function Install-EngramQtHost {
    param([Parameter(Mandatory = $true)][string]$QtRoot)

    if (-not (Test-Path -LiteralPath $QtRoot)) {
        return
    }
    if (-not (Test-Path -LiteralPath $engramCapiLibrary)) {
        throw "expected Engram native library missing: $engramCapiLibrary"
    }

    Copy-Item -LiteralPath $engramCapiLibrary -Destination (Join-Path $QtRoot $nativeLibraryName) -Force
}

function Install-EngramXamlHost {
    param([Parameter(Mandatory = $true)][string]$XamlRoot)

    if (-not (Test-Path -LiteralPath $XamlRoot)) {
        return
    }
    if (-not (Test-Path -LiteralPath $engramCapiLibrary)) {
        throw "expected Engram native library missing: $engramCapiLibrary"
    }

    Copy-Item -LiteralPath $engramCapiLibrary -Destination (Join-Path $XamlRoot $nativeLibraryName) -Force
}

function Add-EngramSwiftUIPackageBridge {
    param([Parameter(Mandatory = $true)][string]$PackagePath)

    if (-not (Test-Path -LiteralPath $PackagePath)) {
        return
    }
    $content = Get-Content -LiteralPath $PackagePath -Raw
    if ($content.Contains('name: "CEngram"')) {
        return
    }

    $content = [regex]::Replace(
        $content,
        'targets: \[\r?\n',
        "targets: [`r`n    .systemLibrary(`r`n      name: ""CEngram"",`r`n      path: ""Sources/CEngram""`r`n    ),`r`n",
        1
    )
    $content = [regex]::Replace(
        $content,
        '\.executableTarget\(\r?\n\s+name: "App",\r?\n\s+path: "Sources/App"\r?\n\s+\)',
        ".executableTarget(`r`n      name: ""App"",`r`n      dependencies: [""CEngram""],`r`n      path: ""Sources/App"",`r`n      linkerSettings: [`r`n        .unsafeFlags([""-L"", ""Sources/CEngram/lib"", ""-lengram_capi""])`r`n      ]`r`n    )",
        1
    )
    Write-Utf8NoBom -Path $PackagePath -Content $content
}

function Install-EngramSwiftUIHost {
    param([Parameter(Mandatory = $true)][string]$SwiftUIRoot)

    if (-not (Test-Path -LiteralPath $SwiftUIRoot)) {
        return
    }
    if (-not (Test-Path -LiteralPath $engramCapiStaticLibrary)) {
        throw "expected Engram static native library missing: $engramCapiStaticLibrary"
    }

    $appDir = Join-Path $SwiftUIRoot "Sources\App"
    $moduleDir = Join-Path $SwiftUIRoot "Sources\CEngram"
    $includeDir = Join-Path $moduleDir "include"
    $libDir = Join-Path $moduleDir "lib"
    New-Item -ItemType Directory -Force -Path $appDir, $includeDir, $libDir | Out-Null

    Copy-Item -LiteralPath $engramCapiHeader -Destination (Join-Path $includeDir "engram.h") -Force
    Copy-Item -LiteralPath $engramCapiStaticLibrary -Destination (Join-Path $libDir $staticLibraryName) -Force
    Write-Utf8NoBom -Path (Join-Path $moduleDir "module.modulemap") -Content @"
module CEngram {
  header "include/engram.h"
  export *
}
"@

    Add-EngramSwiftUIPackageBridge -PackagePath (Join-Path $SwiftUIRoot "Package.swift")
}

function Add-EngramComposeHostDependencies {
    param([Parameter(Mandatory = $true)][string]$BuildGradlePath)

    if (-not (Test-Path -LiteralPath $BuildGradlePath)) {
        return
    }
    $content = Get-Content -LiteralPath $BuildGradlePath -Raw
    if ($content.Contains('net.java.dev.jna:jna')) {
        return
    }

    $content = $content.Replace(
        '    implementation(compose.desktop.currentOs)',
        "    implementation(compose.desktop.currentOs)`r`n    implementation(""net.java.dev.jna:jna:5.19.1"")`r`n    implementation(""org.json:json:20260522"")"
    )
    Write-Utf8NoBom -Path $BuildGradlePath -Content $content
}

function Install-EngramComposeHost {
    param([Parameter(Mandatory = $true)][string]$ComposeRoot)

    if (-not (Test-Path -LiteralPath $ComposeRoot)) {
        return
    }
    if (-not (Test-Path -LiteralPath $engramCapiLibrary)) {
        throw "expected Engram native library missing: $engramCapiLibrary"
    }

    Copy-Item -LiteralPath $engramCapiLibrary -Destination (Join-Path $ComposeRoot $nativeLibraryName) -Force
    Add-EngramComposeHostDependencies -BuildGradlePath (Join-Path $ComposeRoot "build.gradle.kts")
}

function Add-EngramFlutterHostDependencies {
    param([Parameter(Mandatory = $true)][string]$PubspecPath)

    if (-not (Test-Path -LiteralPath $PubspecPath)) {
        return
    }
    $content = Get-Content -LiteralPath $PubspecPath -Raw
    $lineEnding = if ($content.Contains("`r`n")) { "`r`n" } else { "`n" }
    if (-not $content.Contains("  ffi:")) {
        $content = $content.Replace(
            "dependencies:`n  flutter:`n    sdk: flutter",
            "dependencies:`n  flutter:`n    sdk: flutter`n  ffi: ^2.1.3"
        )
        $content = $content.Replace(
            "dependencies:`r`n  flutter:`r`n    sdk: flutter",
            "dependencies:`r`n  flutter:`r`n    sdk: flutter`r`n  ffi: ^2.1.3"
        )
    }
    if (-not $content.Contains("  file_selector:")) {
        if ($content.Contains("  ffi: ^2.1.3")) {
            $content = $content.Replace(
                "  ffi: ^2.1.3",
                "  ffi: ^2.1.3${lineEnding}  file_selector: ^1.0.3"
            )
        } else {
            $content = $content.Replace(
                "dependencies:`n  flutter:`n    sdk: flutter",
                "dependencies:`n  flutter:`n    sdk: flutter`n  file_selector: ^1.0.3"
            )
            $content = $content.Replace(
                "dependencies:`r`n  flutter:`r`n    sdk: flutter",
                "dependencies:`r`n  flutter:`r`n    sdk: flutter`r`n  file_selector: ^1.0.3"
            )
        }
    }
    Write-Utf8NoBom -Path $PubspecPath -Content $content
}

function Install-EngramFlutterHost {
    param([Parameter(Mandatory = $true)][string]$FlutterRoot)

    if (-not (Test-Path -LiteralPath $FlutterRoot)) {
        return
    }
    if (-not (Test-Path -LiteralPath $engramCapiLibrary)) {
        throw "expected Engram native library missing: $engramCapiLibrary"
    }

    Copy-Item -LiteralPath $engramCapiLibrary -Destination (Join-Path $FlutterRoot $nativeLibraryName) -Force
    Add-EngramFlutterHostDependencies -PubspecPath (Join-Path $FlutterRoot "pubspec.yaml")
}

$backends = @(
    "html",
    "webcomponent",
    "react",
    "electron",
    "swiftui",
    "qt",
    "xaml",
    "flutter",
    "compose"
)

foreach ($backend in $backends) {
    Write-Host "==> Emitting EngramApp $backend project"
    $cargoArgs = @("run")
    if ($Release) {
        $cargoArgs += "--release"
    }
    $cargoArgs += @(
        "-p",
        "mosaic-compile",
        "--",
        "pkg",
        $packageRoot,
        "--backend",
        $backend,
        "--output",
        $outputRoot,
        "--emit-project"
    )

    Push-Location $rustWorkspace
    try {
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "mosaic-compile failed for backend '$backend' with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

Install-EngramHtmlHost -HtmlRoot (Join-Path $outputRoot "html")
Install-EngramWebComponentHost -WebComponentRoot (Join-Path $outputRoot "webcomponent")
Install-EngramReactHost -ReactRoot (Join-Path $outputRoot "react")
Install-EngramElectronHost -ElectronRoot (Join-Path $outputRoot "electron")
Install-EngramQtHost -QtRoot (Join-Path $outputRoot "qt")
Install-EngramSwiftUIHost -SwiftUIRoot (Join-Path $outputRoot "swiftui")
Install-EngramXamlHost -XamlRoot (Join-Path $outputRoot "xaml")
Install-EngramFlutterHost -FlutterRoot (Join-Path $outputRoot "flutter")
Install-EngramComposeHost -ComposeRoot (Join-Path $outputRoot "compose")

Write-Host "Engram Mosaic host shells written to $outputRoot"
