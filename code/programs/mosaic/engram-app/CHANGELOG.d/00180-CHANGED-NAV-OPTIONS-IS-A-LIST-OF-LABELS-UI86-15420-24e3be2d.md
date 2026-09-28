### Changed — `nav-options` is a list of labels (UI86, #15420)

The screen switcher's slot type follows toolkit 0.15: `list<text>` instead of
`list<list<text>>`. The selected screen is announced by the platform's own
selected state, not by a name the engine wrote. The toolkit dependency moves
to 0.15.0, and `engram_engine.wasm` is rebuilt.

