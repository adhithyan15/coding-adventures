### Added — default-off Flutter startup diagnostic

The generated Flutter phone `main()` can record whether application-support
directory lookup returned a usable path or threw, for the opt-in Android
failure investigation. Timing collection is disabled by default. The existing
startup and persistence behavior is unchanged.
