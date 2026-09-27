### Security - validate literal HostLink.href's URI scheme (#13052)

Follow-up to #12038 (the identical XAML gap). Added `has_disallowed_uri_scheme`
(new `PipelineEmitError::UnsafeUriScheme` variant), checked when `external`
is not `false` — the path a real `launchUrl` call will eventually use.
Today that path is still a `/* TODO: launchUrl(Uri.parse(...)) */` comment,
never real code, so there's no live navigation vulnerability yet; this is a
preventative fix ahead of that landing rather than a response to a
currently-reachable gap. A relative reference with no scheme at all (`"#"`,
`"/about"`) is unaffected, and `external: false` (dispatch-only, never
reaching `launchUrl`) is unaffected either way — matching every other
backend's #13052 scoping. No runtime (slot-bound) guard was added, for the
same reason: there is no real navigation call yet to guard.

