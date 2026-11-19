### Improved — Flutter Android diagnostic log delivery

The opt-in Flutter phone diagnostics now emit bounded records through Flutter's
Android log path and mark entry into `main()` and the start of the app-support
lookup. Ordinary builds remain silent; the existing Android persistence gate
and its deadline are unchanged.
