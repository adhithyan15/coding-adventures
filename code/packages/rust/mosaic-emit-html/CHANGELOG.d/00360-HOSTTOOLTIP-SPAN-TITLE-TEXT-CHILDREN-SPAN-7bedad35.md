- **`HostTooltip` → `<span title="text">{children}</span>`**:
  - `text: str|slot` → real `title=` attribute (string literal or `{{slot}}` marker)
  - Single child wraps inside; multi-child case walks recursively
  - Plain-text only in v1 per UI29-4 §3.2

