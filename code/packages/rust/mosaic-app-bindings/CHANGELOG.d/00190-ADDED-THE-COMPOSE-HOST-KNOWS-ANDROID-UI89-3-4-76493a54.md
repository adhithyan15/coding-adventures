### Added — the Compose host knows Android (UI89 §3.4)

`mosaicPlatform()` reports `android` on Android's runtime, and
`MosaicRuntimeHost.stateDirectory` lets the platform say where state lives
(the Android activity sets its `filesDir`); without it, Android keeps no
state rather than guessing a path. Desktop is unchanged.

