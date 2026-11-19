### Fixed — strict Flutter sample host diagnostics

The native-required sample shell includes opt-in diagnostic call sites. Its
placeholder `mosaic_host.dart` now defines the matching inert helper, so the
generated main and placeholder host remain a compilable pair before the
package builder installs the standard runtime binding.
