- **Qt control fill widths (#17338).** Journal's exact `width: 100%` search,
  title, and body fields now lower to `Layout.fillWidth: true`. The stale Qt
  `width` degradation pin is removed, while unsupported geometry and the
  measured Qt and Flutter `min-height` gaps remain explicit.
