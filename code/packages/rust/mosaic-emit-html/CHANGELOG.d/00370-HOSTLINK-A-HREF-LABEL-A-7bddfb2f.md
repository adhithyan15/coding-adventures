- **`HostLink` → `<a href ...>label</a>`**:
  - `href: str|slot` → real `href=` attribute (string literal or `{{slot}}` template marker)
  - `label: str|slot` → text body
  - `target: new-tab` → `target="_blank"` paired with `rel="noopener noreferrer"` (security default — prevents reverse-tabnabbing; eslint react/jsx-no-target-blank parity)
  - `target: parent|top|same` → standard HTML `target=` values
  - `external: false` keyword → `data-external="false"` marker for hydration to intercept
  - `onActivate: emit: onX` → `data-on-activate="onX"` marker

