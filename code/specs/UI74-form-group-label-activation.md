# UI74: Form-group disabledness and label activation

## Goal

Venture must apply HTML form-group and label behavior once in the shared
browser pipeline. Native and generated hosts forward pointer input; they do not
reimplement disabledness, association, focus, or activation policy.

## Effective disabledness

- A disabled `fieldset` disables descendant controls except controls inside its
  first direct `legend` child.
- An outer disabled fieldset remains authoritative inside nested fieldsets.
- Effectively disabled controls cannot focus, activate, validate, or contribute
  successful form entries.
- The HTML render projection carries this effective state so layout, paint,
  interaction, accessibility, and submission consume one answer.

## Label association

- `label[for]` targets the first rendered labelable control with the matching
  authored id. A missing target leaves the label inactive.
- A label without `for` targets its first descendant rendered control.
- Label geometry becomes a semantic control hit region using the same clips,
  transforms, fixed positioning, and scroll policy as other interactive areas.
- A nested control's direct region remains above its wrapping label region.
- Label activation focuses text-like controls and runs the existing semantic
  activation reducer for choice, button, and picker controls. Image-submit
  activation uses keyboard coordinates `(0, 0)` because label geometry is not
  image-local input.

## Acceptance

The deterministic form-group fixture covers explicit and implicit labels,
first-legend exemption, blocked required controls, successful-control ordering,
and shared hit-region metadata. Venture package acceptance verifies that every
host inherits the behavior through the existing page-input ABI.
