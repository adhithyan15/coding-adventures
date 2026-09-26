- **Scripting is off by default.** `HtmlParseOptions::default()` now uses
  `HtmlScriptingMode::Disabled`. Nothing parsing through the defaults, Venture
  included, runs scripts, and with scripting on a `<noscript>` fallback became
  raw text instead of markup. Conformance cases keep their `#script-on` /
  `#script-off` flags (unflagged cases still run scripting-on); tests that pin
  scripting-on behaviour opt in explicitly.
