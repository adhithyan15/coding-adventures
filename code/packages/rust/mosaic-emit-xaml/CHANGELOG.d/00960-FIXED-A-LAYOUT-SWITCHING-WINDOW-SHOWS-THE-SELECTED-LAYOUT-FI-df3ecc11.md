### Fixed — a layout-switching window shows the selected layout first, and registry names are checked

- A native-complete window with layout variants starts its runtime only
  once the window has been laid out. A start queued before the first layout
  pass waits for the content's first `SizeChanged` (`StartRuntimeOnceLaidOut`,
  unsubscribed as it fires), so the first root mounted is the one the
  window selects; the loading view shows until then. Before, a window that
  opened already narrow could show the default layout for one frame and
  then switch (UI48 §7.11 known gap). A retried start, already laid out,
  starts at once. A package without variants is unchanged.
- Layout-variant names are checked against a `ComponentRegistry`'s
  components in the generated namespace too
  (`ComponentRegistry::components_in_namespace`), in `from_pipeline` and
  `from_pipeline_variant`: a variant may not spell such a component, its
  `…Event` union or its `…Mosaic…` support names. `mosaic-compile`'s
  single-file mode registers the package's sibling exports there; before,
  their names were not checked (UI48 §7.11 known gap).
