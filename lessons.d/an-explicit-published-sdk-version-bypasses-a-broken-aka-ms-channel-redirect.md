---
category: CI & GitHub Actions
---

# An explicit published SDK version bypasses a broken aka.ms channel redirect

On PR #18126, two Windows XAML jobs failed inside `actions/setup-dotnet@v6`
before compiling or testing. A targeted retry reproduced HTTP 409 from
`dotnetorigin.blob.core.windows.net` for the floating 9.0 SDK and LTS runtime.
The same SDK archive on Microsoft's `builds.dotnet.microsoft.com` feed answered
HTTP 200, so this was a download-route failure, not a product build failure.

The bundled installer uses aka.ms only when no exact version is supplied.
Pinning the two failing jobs to SDK 9.0.319, published October 13 and older
than the repository's seven-day release floor, selects the direct feed while
preserving the .NET 9 target. Verify the version and Windows archive against
Microsoft's release metadata and run the actual installer in dry-run mode.
The action's separate floating LTS-runtime probe may still warn; its SDK
installation is the required step and includes the matching runtime. Keep
CI pending until the native compile and test steps actually finish.
