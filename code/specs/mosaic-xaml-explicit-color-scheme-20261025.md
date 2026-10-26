# Explicit XAML color schemes

For #18262 and #14273, translate authored color-scheme light/dark into FrameworkElement.RequestedTheme Light/Dark. Apply through the reusable style path, including container wrappers and host controls. This aligns inherited native template resources with an explicitly authored surface, rather than setting global app/OS preferences or hard-coding placeholder brushes.

Only single light/dark keywords, case-insensitive and whitespace-trimmed, are accepted. Other CSS values (normal, light dark, only dark, cascade keywords) remain unsupported and must not emit invalid enum literals. No app-specific or generated-output changes. High-contrast behavior remains controlled by WinUI.

Validate property/value mapping, unsupported values and generated nested container/input output. Build and launch a generated native VisiCalc candidate; inspect blank placeholder, focused editor, typed text and keyboard navigation. Keep the issue open if the visual defect persists. Microsoft documents inheritance and template resource lookup at https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.frameworkelement.requestedtheme .

## Native validation (2026-10-25)

Generated unmodified VisiCalc sources from base `d790f9a4080` plus this change.
The workbook Grid now has `RequestedTheme="Dark"`; no app-specific or generated
source edits were made. Release x64 `dotnet build` succeeded with zero errors and
two existing CS0067 warnings. All 395 emitter unit tests passed; security review
passed.

Using the real WinUI window through Computer Use, selected blank G6: the
placeholder was visibly light against the dark surface. Clicking the formula
field retained a dark focused surface (the preceding build showed a white one).
UI Automation identified focus as `edit ... ID: formula-field`. Typed `=20+22`,
pressed Enter and observed G6 = 42. Clicked G6, pressed Right and observed H6 plus
the readable blank placeholder. Closed normally and confirmed process exit.
Screenshots were inspected in this chat, not committed as binary artifacts.

Local build/generation/test logs are under
`C:/Users/adhit/.codex/worktrees/visicalc-filtered-measurement-20261025-output/` as
`theme-release-build.log`, `theme-generation.log`, and `theme-unit-tests.log`.
Native candidate: `theme-xaml/xaml/bin/x64/Release/net9.0-windows10.0.19041.0/win-x64/VisiCalc.exe`.
The unchanged Oct16 runtime DLL was reused, SHA256
`A18198012F2985EC5CEA6E10A2151388D2E9093F567A59F46A72E45C28BC2124`.

This is a visual and interaction regression receipt, not measured WCAG contrast
or release acceptance. Light-theme, disabled-state, high-contrast and broader
accessibility/design checks remain open in #18262 / #14278 / #14273. Performance
percentiles and distributable release acceptance remain separate open gates.
