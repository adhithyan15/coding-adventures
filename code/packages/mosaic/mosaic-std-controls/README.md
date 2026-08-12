# mosaic-std-controls

Accessible controls with coherent Foundation defaults for native Mosaic apps.
Version 0.2 provides four controls common to forms and settings screens:

- `Button`: native activation, focus, disabled, and accessibility semantics;
- `Input`: native single-line editing, placeholder, disabled/read-only, change,
  and commit semantics;
- `Checkbox`: native two-state selection, label, focus, keyboard, and change
  semantics;
- `NumberInput`: native numeric editing, mobile keyboard, disabled, and commit
  semantics.

```toml
[dependencies]
mosaic-std-controls = "0.2.0"
```

```mll
layout SignIn {
  Column {
    pkg::mosaic-std-controls::Input (
      placeholder: "Email address",
      onChange: emit: onEmailChange,
      onCommit: emit: onSubmit
    )
    pkg::mosaic-std-controls::NumberInput (
      placeholder: "Team size",
      onChange: emit: onTeamSizeChange
    )
    pkg::mosaic-std-controls::Checkbox (
      label: "Remember this device",
      onChange: emit: onRememberChange
    )
    pkg::mosaic-std-controls::Button (
      label: "Continue",
      onClick: emit: onSubmit
    )
  }
}
```

Both controls delegate interaction and accessibility to Mosaic's native host
primitives. Their public slots have honest defaults, so callers do not need to
pass toolkit-specific variant or size values. The package carries fallback
values for the public Foundation token contract, providing the light/dark
palette, spacing, radius, and type scale without requiring per-app setup.
Application token overrides still take precedence everywhere.

The package reuses `mosaic-pkg-toolkit` as an implementation dependency while
presenting a smaller stable standard-library contract. All-five-native package
and consuming-app tests guard the facade. Radio remains intentionally absent
until every backend implements native group mutual exclusion; select/picker,
switch, and slider are tracked follow-ups.
