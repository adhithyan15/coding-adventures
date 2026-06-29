param(
    [string]$Output = "target/mosaic-engram-app",
    [switch]$Release
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$packageRoot = (Resolve-Path (Join-Path $scriptRoot "..")).Path
$rustWorkspace = (Resolve-Path (Join-Path $packageRoot "..\..\..\packages\rust")).Path
$engramWasmRoot = (Resolve-Path (Join-Path $rustWorkspace "engram-wasm")).Path
$hostRoot = Join-Path $packageRoot "host"
$nativeProfile = if ($Release) { "release" } else { "debug" }
$nativeLibraryName = if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
    "engram_capi.dll"
} elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::OSX)) {
    "libengram_capi.dylib"
} else {
    "libengram_capi.so"
}
$engramCapiLibrary = Join-Path $rustWorkspace "target\$nativeProfile\$nativeLibraryName"

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
$engramWasmTypes = Join-Path $hostRoot "web\engram-mosaic-host-wasm.d.ts"
$engramWebHost = Join-Path $hostRoot "web\engram-host.ts"
$engramElectronHost = Join-Path $hostRoot "electron\host.js"
$engramXamlHost = Join-Path $hostRoot "xaml\MosaicHost.cs"

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Content
    )
    $encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

function Add-EngramHostImport {
    param([Parameter(Mandatory = $true)][string]$MainPath)

    if (-not (Test-Path -LiteralPath $MainPath)) {
        return
    }
    $content = Get-Content -LiteralPath $MainPath -Raw
    $importLine = 'import "./engram-host";'
    if ($content.Contains($importLine)) {
        return
    }
    $match = [regex]::Match($content, 'import \{ EngramApp \} from "\.\./EngramApp";\r?\n')
    if ($match.Success) {
        $insertAt = $match.Index + $match.Length
        $content = $content.Insert($insertAt, "$importLine`r`n")
    } else {
        $content = "$importLine`r`n$content"
    }
    Write-Utf8NoBom -Path $MainPath -Content $content
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
    Copy-Item -LiteralPath $engramWasmTypes -Destination (Join-Path $srcDir "engram-mosaic-host-wasm.d.ts") -Force
    Copy-Item -LiteralPath $engramWebHost -Destination (Join-Path $srcDir "engram-host.ts") -Force
    Add-EngramHostImport -MainPath (Join-Path $srcDir "main.tsx")
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
    Copy-Item -LiteralPath $engramElectronHost -Destination (Join-Path $electronDir "host.js") -Force
}

function Add-EngramXamlNativeContent {
    param(
        [Parameter(Mandatory = $true)][string]$CsprojPath,
        [Parameter(Mandatory = $true)][string]$NativeLibraryName
    )

    if (-not (Test-Path -LiteralPath $CsprojPath)) {
        return
    }
    $content = Get-Content -LiteralPath $CsprojPath -Raw
    $includeLine = "<Content Include=""$NativeLibraryName"" CopyToOutputDirectory=""PreserveNewest"" />"
    if ($content.Contains($includeLine)) {
        return
    }
    $itemGroup = "  <ItemGroup>`r`n    $includeLine`r`n  </ItemGroup>`r`n"
    $content = $content.Replace("</Project>", "$itemGroup</Project>")
    Write-Utf8NoBom -Path $CsprojPath -Content $content
}

function Install-EngramXamlHost {
    param([Parameter(Mandatory = $true)][string]$XamlRoot)

    if (-not (Test-Path -LiteralPath $XamlRoot)) {
        return
    }
    if (-not (Test-Path -LiteralPath $engramCapiLibrary)) {
        throw "expected Engram native library missing: $engramCapiLibrary"
    }

    Copy-Item -LiteralPath $engramXamlHost -Destination (Join-Path $XamlRoot "MosaicHost.cs") -Force
    Copy-Item -LiteralPath $engramCapiLibrary -Destination (Join-Path $XamlRoot $nativeLibraryName) -Force

    $csproj = Get-ChildItem -LiteralPath $XamlRoot -Filter "*.csproj" | Select-Object -First 1
    if ($null -ne $csproj) {
        Add-EngramXamlNativeContent -CsprojPath $csproj.FullName -NativeLibraryName $nativeLibraryName
    }
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

Install-EngramReactHost -ReactRoot (Join-Path $outputRoot "react")
Install-EngramElectronHost -ElectronRoot (Join-Path $outputRoot "electron")
Install-EngramXamlHost -XamlRoot (Join-Path $outputRoot "xaml")

Write-Host "Engram Mosaic host shells written to $outputRoot"
