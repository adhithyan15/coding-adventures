# Explicit XAML color schemes

For #18262 and #14273, translate authored color-scheme light/dark into FrameworkElement.RequestedTheme Light/Dark. Apply through the reusable style path, including container wrappers and host controls. This aligns inherited native template resources with an explicitly authored surface, rather than setting global app/OS preferences or hard-coding placeholder brushes.

Only single light/dark keywords, case-insensitive and whitespace-trimmed, are accepted. Other CSS values (normal, light dark, only dark, cascade keywords) remain unsupported and must not emit invalid enum literals. No app-specific or generated-output changes. High-contrast behavior remains controlled by WinUI.

Validate property/value mapping, unsupported values and generated nested container/input output. Build and launch a generated native VisiCalc candidate; inspect blank placeholder, focused editor, typed text and keyboard navigation. Keep the issue open if the visual defect persists. Microsoft documents inheritance and template resource lookup at https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.frameworkelement.requestedtheme .
