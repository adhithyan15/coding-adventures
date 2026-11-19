### Added — early Flutter phone startup markers

When the opt-in Mosaic native diagnostics define is enabled, the generated
phone entry point records entry into `main()` and the app-support lookup before
awaiting the platform plugin. Default builds remain unchanged.
