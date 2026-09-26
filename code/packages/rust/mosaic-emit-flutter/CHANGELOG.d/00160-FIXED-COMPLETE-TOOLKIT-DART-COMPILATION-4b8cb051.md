### Fixed - complete toolkit Dart compilation

Flutter control lowerings now avoid generated component-name collisions with
Material's `Checkbox`, `Radio`, and `Tooltip`, derive callback fields from the
declared event payload, carry indexed `HostLink` events through their loop
shadow, preserve decimal numeric payloads as `num`, and omit unused truthiness
and loop-index bindings. The complete 23-component toolkit is analyzer-clean
and builds as a native Flutter desktop application.

