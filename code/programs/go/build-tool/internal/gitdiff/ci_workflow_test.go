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

func TestAnalyzeCIWorkflowPatchAllowsSharedJVMToolchainChanges(t *testing.T) {
	patch := `
@@ -314,0 +315,11 @@
+      - name: Set up JDK 21
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: actions/setup-java@v4
+        with:
+          distribution: 'temurin'
+          java-version: '21'
+      - name: Set up Gradle
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: gradle/actions/setup-gradle@v4
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
