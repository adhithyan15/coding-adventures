# Native depth tokens must retire only their non-inset web-shadow companions

When a stylesheet authors both a portable elevation token and a CSS shadow,
the native emitter may render the depth from the elevation token rather than
parsing the web shadow. The degradation reporter should therefore count the
non-inset shadow as consumed only when the same part has a supported elevation
value. Standalone shadows, inset drawing effects, and unsupported elevation
values must remain visible debt.
