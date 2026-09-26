### MSBuild csproj details that required experimentation

- `<UseRidGraph>true</UseRidGraph>` — WindowsAppSDK uses legacy
  `win10-*` RIDs that .NET 8+ removed from the default graph.
- `<AppxGeneratePriEnabled>false</AppxGeneratePriEnabled>` +
  `<EnableDefaultPriItems>false</EnableDefaultPriItems>` — bypass
  most AppxPackage MSBuild plumbing that requires Visual Studio.
  One cosmetic MSB4062 still fires at the very end of build; the
  .exe + dependencies are produced first.
- `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` bundles the
  pinned 1.8 WinUI framework libraries so generated hosts do not depend on a
  separately registered machine runtime. The .NET runtime remains
  framework-dependent.

