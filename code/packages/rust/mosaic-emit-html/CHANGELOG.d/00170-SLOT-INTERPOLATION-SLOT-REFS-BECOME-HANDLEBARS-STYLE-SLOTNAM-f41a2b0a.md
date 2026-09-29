- **Slot interpolation:** slot refs become Handlebars-style `{{slotName}}`
  template tokens. The host either pre-substitutes them server-side or pipes
  the output through a downstream JS hydrator (out of scope).
