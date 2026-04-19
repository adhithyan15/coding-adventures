package gitdiff

import "testing"

func TestAnalyzeCIWorkflowPatchAllowsToolchainScopedDotnetChanges(t *testing.T) {
	patch := `
@@ -312,0 +313,6 @@
+      - name: Set up .NET
+        if: needs.detect.outputs.needs_dotnet == 'true'
+        uses: actions/setup-dotnet@v4
+        with:
+          dotnet-version: '9.0.x'
`

	change := AnalyzeCIWorkflowPatch(patch)
	if change.RequiresFullRebuild {
		t.Fatalf("expected toolchain-scoped dotnet change to stay incremental")
	}

	got := SortedToolchains(change.Toolchains)
	if len(got) != 1 || got[0] != "dotnet" {
		t.Fatalf("expected dotnet toolchain only, got %v", got)
	}
}

func TestAnalyzeCIWorkflowPatchAllowsToolchainScopedSwiftChanges(t *testing.T) {
	patch := `
@@ -300,0 +301,54 @@
+      - name: Set up Swift (Windows)
+        if: needs.detect.outputs.needs_swift == 'true' && runner.os == 'Windows'
+        shell: pwsh
+        run: |
+          winget install --id Swift.Toolchain --exact --accept-package-agreements --accept-source-agreements --source winget
+          $wingetExitCode = $LASTEXITCODE
+          if ($wingetExitCode -ne 0) {
+            Write-Warning "winget install exited with code $wingetExitCode; checking whether Swift is available anyway"
+          }
+          $currentPathEntries = @($env:Path -split ';')
+          $currentPathSet = @{}
+          foreach ($entry in $currentPathEntries) {
+            if (-not [string]::IsNullOrWhiteSpace($entry)) {
+              $currentPathSet[$entry] = $true
+            }
+          }
+          $registryPathEntries = @(
+            [System.Environment]::GetEnvironmentVariable('Path', 'User')
+            [System.Environment]::GetEnvironmentVariable('Path', 'Machine')
+          ) | Where-Object { $_ } | ForEach-Object { $_ -split ';' } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
+          $newPathEntries = @()
+          foreach ($entry in $registryPathEntries) {
+            if (-not $currentPathSet.ContainsKey($entry)) {
+              $currentPathSet[$entry] = $true
+              $newPathEntries += $entry
+            }
+          }
+          foreach ($entry in $newPathEntries) {
+            $entry | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
+          }
+          if ($newPathEntries.Count -gt 0) {
+            $env:Path = "$env:Path;$($newPathEntries -join ';')"
+          }
+          $swiftRoots = @(
+            (Join-Path $env:LOCALAPPDATA 'Programs\Swift')
+            (Join-Path $env:ProgramFiles 'Swift')
+          ) | Where-Object { Test-Path $_ }
+          $swiftExe = @(
+            (Get-Command swift -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue)
+            ($swiftRoots | ForEach-Object {
+              Get-ChildItem -Path $_ -Filter swift.exe -Recurse -File -ErrorAction SilentlyContinue |
+                Sort-Object FullName -Descending |
+                Select-Object -First 1 -ExpandProperty FullName
+            })
+          ) | Where-Object { $_ } | Select-Object -First 1
+          if (-not $swiftExe) {
+            if ($wingetExitCode -ne 0) {
+              throw "winget install exited with code $wingetExitCode and swift.exe was not found"
+            }
+            throw 'swift.exe not found after winget install'
+          }
+          $toolchainBin = Split-Path -Parent $swiftExe
+          if (-not $currentPathSet.ContainsKey($toolchainBin)) {
+            $toolchainBin | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
+            $env:Path = "$env:Path;$toolchainBin"
+            $currentPathSet[$toolchainBin] = $true
+          }
+          Write-Host "Using Swift from $swiftExe"
+          $sdkRoot = [System.Environment]::GetEnvironmentVariable('SDKROOT', 'User')
+          if (-not $sdkRoot) {
+            $sdkRoot = [System.Environment]::GetEnvironmentVariable('SDKROOT', 'Machine')
+          }
+          if (-not $sdkRoot) {
+            $sdkRoot = $swiftRoots | ForEach-Object {
+            Get-ChildItem -Path $_ -Filter Windows.sdk -Directory -Recurse -ErrorAction SilentlyContinue |
+              Sort-Object FullName -Descending |
+              Select-Object -First 1 -ExpandProperty FullName
+            } | Where-Object { $_ } | Select-Object -First 1
+          }
+          if ($sdkRoot) {
+            "SDKROOT=$sdkRoot" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
+            $env:SDKROOT = $sdkRoot
+            Write-Host "Using SDKROOT=$sdkRoot"
+          }
+          where.exe swift
`

	change := AnalyzeCIWorkflowPatch(patch)
	if change.RequiresFullRebuild {
		t.Fatalf("expected swift toolchain change to stay incremental")
	}

	got := SortedToolchains(change.Toolchains)
	if len(got) != 1 || got[0] != "swift" {
		t.Fatalf("expected swift toolchain only, got %v", got)
	}
}

func TestAnalyzeCIWorkflowPatchAllowsWindowsSwiftOnlyBuildStep(t *testing.T) {
	patch := `
@@ -430,0 +431,9 @@
+      - name: Build and test affected Swift packages on Windows
+        if: runner.os == 'Windows' && needs.detect.outputs.needs_swift == 'true'
+        shell: pwsh
+        run: |
+          $planFlag = @()
+          if (Test-Path 'build-plan.json') {
+            Write-Host 'Using pre-computed build plan'
+            $planFlag = @('-plan-file', 'build-plan.json')
+          }
+          & ./build-tool.exe -root . @planFlag -validate-build-files -language swift
@@ -440,0 +450,1 @@
+        if: runner.os != 'Windows' || (needs.detect.outputs.needs_swift == 'true' && false)
`

	change := AnalyzeCIWorkflowPatch(patch)
	if change.RequiresFullRebuild {
		t.Fatalf("expected windows swift-only build step to stay incremental")
	}

	got := SortedToolchains(change.Toolchains)
	if len(got) != 1 || got[0] != "swift" {
		t.Fatalf("expected swift toolchain only, got %v", got)
	}
}

func TestAnalyzeCIWorkflowPatchAllowsSharedJVMToolchainChanges(t *testing.T) {
	patch := `
@@ -314,0 +315,17 @@
+      - name: Set up JDK 21
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: actions/setup-java@v4
+        with:
+          distribution: 'temurin'
+          java-version: '21'
+      - name: Set up Gradle
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: gradle/actions/setup-gradle@v4
+      - name: Disable long-lived Gradle services on Windows CI
+        if: (needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true') && runner.os == 'Windows'
+        shell: bash
+        run: |
+          {
+            echo 'GRADLE_OPTS=-Dorg.gradle.daemon=false -Dorg.gradle.vfs.watch=false'
+          } >> "$GITHUB_ENV"
`

	change := AnalyzeCIWorkflowPatch(patch)
	if change.RequiresFullRebuild {
		t.Fatalf("expected JVM toolchain change to stay incremental")
	}

	got := SortedToolchains(change.Toolchains)
	if len(got) != 2 || got[0] != "java" || got[1] != "kotlin" {
		t.Fatalf("expected java and kotlin toolchains, got %v", got)
	}
}

func TestAnalyzeCIWorkflowPatchAllowsPatchedLuaRocksBootstrap(t *testing.T) {
	patch := `
@@ -260,0 +261,5 @@
+          tmpdir="$(mktemp -d)"
+          curl -L --fail https://luarocks.org/manifests/hoelzro/lua-term-0.8-1.rockspec -o "$tmpdir/lua-term-0.8-1.rockspec"
+          sed -i.bak 's|archive/0\.08\.tar\.gz|archive/refs/tags/0.08.tar.gz|' "$tmpdir/lua-term-0.8-1.rockspec"
+          luarocks install "$tmpdir/lua-term-0.8-1.rockspec"
+          rm -rf "$tmpdir"
`

	change := AnalyzeCIWorkflowPatch(patch)
	if change.RequiresFullRebuild {
		t.Fatalf("expected LuaRocks bootstrap change to stay incremental")
	}

	got := SortedToolchains(change.Toolchains)
	if len(got) != 1 || got[0] != "lua" {
		t.Fatalf("expected lua toolchain only, got %v", got)
	}
}

func TestAnalyzeCIWorkflowPatchIgnoresCommentOnlyChanges(t *testing.T) {
	patch := `
@@ -316,2 +316,2 @@
-          # .NET 8 is the current LTS release.
+          # .NET 9 is the current LTS release.
`

	change := AnalyzeCIWorkflowPatch(patch)
	if change.RequiresFullRebuild {
		t.Fatalf("expected comment-only change to stay safe")
	}
	if len(change.Toolchains) != 0 {
		t.Fatalf("expected no toolchains for comment-only change, got %v", change.Toolchains)
	}
}

func TestAnalyzeCIWorkflowPatchRequiresFullRebuildForBuildCommandChanges(t *testing.T) {
	patch := `
@@ -404,1 +404,1 @@
-          $BT -root . -validate-build-files -language all
+          $BT -root . -force -validate-build-files -language all
`

	change := AnalyzeCIWorkflowPatch(patch)
	if !change.RequiresFullRebuild {
		t.Fatalf("expected build command change to require a full rebuild")
	}
}

func TestAnalyzeCIWorkflowPatchRequiresFullRebuildForUnknownWorkflowChanges(t *testing.T) {
	patch := `
@@ -170,0 +171,1 @@
+      timeout-minutes: 45
`

	change := AnalyzeCIWorkflowPatch(patch)
	if !change.RequiresFullRebuild {
		t.Fatalf("expected unknown workflow change to require a full rebuild")
	}
}
