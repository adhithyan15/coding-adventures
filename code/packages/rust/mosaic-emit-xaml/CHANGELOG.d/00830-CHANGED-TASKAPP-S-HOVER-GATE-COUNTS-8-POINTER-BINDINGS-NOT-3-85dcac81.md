### Changed — TaskApp's hover gate counts 8 pointer bindings, not 38 (#14016)

`tests/task_app_hover_compiles_to_xaml.rs` compiles TaskApp's own light
stylesheet. Thirty of the 38 bindings it counted were the inline view
switcher's `seg-*` "off" buttons, which are gone now that TaskApp uses the
toolkit SegmentedControl, so the count is 8. The `Seg*` names are also
removed from its target list. No emitter behaviour changed.

