- **`HostNumberInput` → `<input type="number" inputmode="numeric" ...>`**:
  - `value: slot|number` → real `value=` attribute (slot template or literal)
  - `min` / `max` / `step` numeric literals → matching HTML attribute values
  - `placeholder: str|slot` → real `placeholder=` attribute
  - `disabled: slot|bool` → bare `disabled` keyword OR `data-disabled` marker
  - `onChange: emit: onX` → `data-on-change="onX"` marker
  - `inputmode="numeric"` always set — triggers mobile numeric keyboard

7 new tests pin: HostLink href+label rendering, the target=_blank security pin (rel="noopener noreferrer" paired emission), the external+onActivate data-* markers, HostTooltip span+title wrapping, HostNumberInput inputmode=numeric default, min/max/step numeric literal pass-through, and the onChange data marker.

### Added — U29-2-K-html — `HostCheckbox` + `HostRadio` kernel primitive lowerings

Both new UI29-2 primitives lower to native HTML form controls:

