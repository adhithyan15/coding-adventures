import { describe, it, expect } from "vitest";
import {
  analyzeCIWorkflowPatch,
  sortedToolchains,
} from "../src/ci-workflow.js";

describe("analyzeCIWorkflowPatch", () => {
  it("allows toolchain-scoped dotnet changes", () => {
    const change = analyzeCIWorkflowPatch(`
@@ -312,0 +313,6 @@
+      - name: Set up .NET
+        if: needs.detect.outputs.needs_dotnet == 'true'
+        uses: actions/setup-dotnet@v4
+        with:
+          dotnet-version: '9.0.x'
`);

    expect(change.requiresFullRebuild).toBe(false);
    expect(sortedToolchains(change.toolchains)).toEqual(["dotnet"]);
  });

  it("ignores comment-only changes", () => {
    const change = analyzeCIWorkflowPatch(`
@@ -316,2 +316,2 @@
-          # .NET 8 is the current LTS release.
+          # .NET 9 is the current LTS release.
`);

    expect(change.requiresFullRebuild).toBe(false);
    expect(sortedToolchains(change.toolchains)).toEqual([]);
  });

  it("requires a full rebuild for build command changes", () => {
    const change = analyzeCIWorkflowPatch(`
@@ -404,1 +404,1 @@
-          $BT -root . -validate-build-files -language all
+          $BT -root . -force -validate-build-files -language all
`);

    expect(change.requiresFullRebuild).toBe(true);
  });

  it("requires a full rebuild for unknown workflow changes", () => {
    const change = analyzeCIWorkflowPatch(`
@@ -170,0 +171,1 @@
+      timeout-minutes: 45
`);

    expect(change.requiresFullRebuild).toBe(true);
  });
});
