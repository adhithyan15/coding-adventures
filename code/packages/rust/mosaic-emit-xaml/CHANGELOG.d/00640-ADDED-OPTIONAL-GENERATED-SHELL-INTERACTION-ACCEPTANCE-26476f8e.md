### Added - optional generated-shell interaction acceptance

Generated WinUI applications now invoke an optional package-host interaction
hook after wiring the Mosaic component's dispatch event. Package owners can
exercise emitted native controls and shared dispatch in direct launch
acceptance without adding application-specific behavior to the shell.

