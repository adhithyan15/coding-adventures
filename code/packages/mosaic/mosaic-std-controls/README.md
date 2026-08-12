# mosaic-std-controls

Accessible controls with coherent Foundation defaults for native Mosaic apps.
Version 0.3 provides five controls common to forms and settings screens:

- `Button`: native activation, focus, disabled, and accessibility semantics;
- `Input`: native single-line editing, placeholder, disabled/read-only, change,
  and commit semantics;
- `Checkbox`: native two-state selection, label, focus, keyboard, and change
  semantics;
- `NumberInput`: native numeric editing, mobile keyboard, disabled, and commit
  semantics;
- `Slider`: required human-readable label, native adjustable range semantics,
  optional formatted visible value, disabled state, and separate live-change
  and commit events.

```toml
[dependencies]
mosaic-std-controls = "0.3.0"
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
    pkg::mosaic-std-controls::Slider (
      label: "Notification volume",
      value: 65,
      min: 0,
      max: 100,
      step: 5,
      display-value: "65%",
      onChange: emit: onVolumeChange,
      onCommit: emit: onVolumeCommit
    )
    pkg::mosaic-std-controls::Button (
      label: "Continue",
      onClick: emit: onSubmit
    )
  }
}
```

The controls delegate interaction and accessibility to Mosaic's native host
primitives. Their public slots have honest defaults, so callers do not need to
pass toolkit-specific variant or size values. The package carries fallback
values for the public Foundation token contract, providing the light/dark
palette, spacing, radius, and type scale without requiring per-app setup.
Application token overrides still take precedence everywhere.

The package reuses `mosaic-pkg-toolkit` as an implementation dependency while
presenting a smaller stable standard-library contract. All-five-native package
and consuming-app tests guard the facades. `Slider` keeps its visible formatted
value out of the accessibility tree because each native adjustable control
already announces its numeric range value. Radio remains intentionally absent
until every backend implements native group mutual exclusion; select/picker and
switch are tracked follow-ups.
