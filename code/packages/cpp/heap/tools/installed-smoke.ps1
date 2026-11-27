Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$self = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$repo = $self
while ($repo -and -not (Test-Path -LiteralPath (Join-Path $repo '.git'))) {
    $repo = Split-Path -Parent $repo
}
if (-not $repo) { throw "Git repository root not found above $self" }

$licenseBlob = '4a4dc9a35858c625d7a0833d0a63bb2a7e05563c'
$license = Join-Path $self 'LICENSE'
$licenseHash = (& git -C $repo hash-object --no-filters -- $license).Trim()
if ($LASTEXITCODE -ne 0 -or $licenseHash -ne $licenseBlob) {
    throw 'Package LICENSE does not match the pinned repository license blob'
}

$tempPath = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$repoFull = [IO.Path]::GetFullPath($repo).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$selfFull = [IO.Path]::GetFullPath($self).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if ($tempPath.StartsWith($repoFull, [StringComparison]::OrdinalIgnoreCase) -or
    $tempPath.StartsWith($selfFull, [StringComparison]::OrdinalIgnoreCase)) {
    throw "System temp path must be outside the repository: $tempPath"
}
$runRoot = Join-Path $tempPath ("ca-heap-installed-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runRoot | Out-Null
$prefix = Join-Path $runRoot 'install-prefix'
$relocated = Join-Path $runRoot 'relocated-prefix'
$unavailable = Join-Path $runRoot 'original-prefix-temporarily-unavailable'
$packageBuild = Join-Path $runRoot 'package-build'
$consumerSource = Join-Path $self 'tests\installed'
$consumerBuild = Join-Path $runRoot 'consumer-build'
$log = Join-Path $runRoot 'commands.log'

function Invoke-CheckedNative {
    param([Parameter(Mandatory)][string]$Executable,
          [AllowEmptyCollection()][string[]]$Arguments = @())
    Add-Content -LiteralPath $log -Value "`n`$ $Executable $($Arguments -join ' ')"
    if (-not (Get-Command -Name $Executable -ErrorAction SilentlyContinue)) {
        throw "Required executable was not found: $Executable"
    }
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $lines = & $Executable @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorAction
    }
    if ($null -ne $lines) {
        foreach ($line in $lines) {
            if ($line -is [System.Management.Automation.ErrorRecord]) {
                Add-Content -LiteralPath $log -Value $line.Exception.Message
            }
            else {
                Add-Content -LiteralPath $log -Value ([string]$line)
            }
        }
    }
    if ($exitCode -ne 0) {
        throw "Command failed with exit ${exitCode}: $Executable $($Arguments -join ' '); output: $log"
    }
}

function Assert-PrivateRunPath {
    param([Parameter(Mandatory)][string]$Candidate)
    $candidateFull = [IO.Path]::GetFullPath($Candidate)
    $rootFull = [IO.Path]::GetFullPath($runRoot)
    $rootPrefix = $rootFull.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $candidateFull.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Move/copy path is outside the private run directory: $candidateFull"
    }
}

$envNames = @('CMAKE_PREFIX_PATH','CMAKE_MODULE_PATH','CMAKE_FIND_ROOT_PATH','CMAKE_TOOLCHAIN_FILE')
$savedEnv = @{}
foreach ($name in $envNames) { $savedEnv[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
$movedOriginal = $false
try {
    if ($runRoot.StartsWith($repoFull, [StringComparison]::OrdinalIgnoreCase) -or
        $runRoot.StartsWith($selfFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Private run directory must be outside the repository: $runRoot"
    }
    $consumerSource = Join-Path $runRoot 'consumer-source'
    New-Item -ItemType Directory -Path $consumerSource | Out-Null
    Copy-Item -LiteralPath (Join-Path $self 'tests\installed\CMakeLists.txt') -Destination $consumerSource
    Copy-Item -LiteralPath (Join-Path $self 'tests\installed\main.cpp') -Destination $consumerSource

    Invoke-CheckedNative 'cmake' @('-S', $self, '-B', $packageBuild,
        "-DCMAKE_INSTALL_PREFIX=$prefix",
        '-DCMAKE_FIND_PACKAGE_NO_PACKAGE_REGISTRY=ON',
        '-DCMAKE_FIND_PACKAGE_NO_SYSTEM_PACKAGE_REGISTRY=ON')
    Invoke-CheckedNative 'cmake' @('--build', $packageBuild)
    Invoke-CheckedNative 'cmake' @('--install', $packageBuild)

    $installedLicense = Join-Path $prefix 'share\licenses\ca_heap\LICENSE'
    if (-not (Test-Path -LiteralPath $installedLicense -PathType Leaf)) {
        throw "Installed LICENSE is missing: $installedLicense"
    }
    $installedHash = (& git -C $repo hash-object --no-filters -- $installedLicense).Trim()
    if ($LASTEXITCODE -ne 0 -or $installedHash -ne $licenseBlob) {
        throw 'Installed LICENSE does not match the pinned repository blob'
    }
    Assert-PrivateRunPath $prefix
    Assert-PrivateRunPath $relocated
    Copy-Item -LiteralPath $prefix -Destination $relocated -Recurse
    Assert-PrivateRunPath $prefix
    Assert-PrivateRunPath $unavailable
    Move-Item -LiteralPath $prefix -Destination $unavailable
    $movedOriginal = $true
    foreach ($name in $envNames) { [Environment]::SetEnvironmentVariable($name, '', 'Process') }

    Invoke-CheckedNative 'cmake' @('-S', $consumerSource, '-B', $consumerBuild,
        "-DCA_HEAP_RELOCATED_PREFIX=$relocated",
        "-DCA_HEAP_SOURCE_ROOT=$self",
        "-DCA_HEAP_ORIGINAL_PREFIX=$prefix",
        '-DCMAKE_PREFIX_PATH=',
        '-DCMAKE_FIND_PACKAGE_NO_PACKAGE_REGISTRY=ON',
        '-DCMAKE_FIND_PACKAGE_NO_SYSTEM_PACKAGE_REGISTRY=ON',
        '-DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF',
        '-DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF')
    Invoke-CheckedNative 'cmake' @('--build', $consumerBuild, '--config', 'Release')
    Invoke-CheckedNative 'cmake' @('-E', 'chdir', $consumerBuild, 'ctest', '-C', 'Release', '--output-on-failure')
}
finally {
    if ($movedOriginal -and (Test-Path -LiteralPath $unavailable) -and -not (Test-Path -LiteralPath $prefix)) {
        Assert-PrivateRunPath $unavailable
        Assert-PrivateRunPath $prefix
        Move-Item -LiteralPath $unavailable -Destination $prefix
    }
    foreach ($name in $envNames) {
        [Environment]::SetEnvironmentVariable($name, $savedEnv[$name], 'Process')
    }
}

Write-Output "Installed-prefix consumer passed. Evidence: $runRoot"
