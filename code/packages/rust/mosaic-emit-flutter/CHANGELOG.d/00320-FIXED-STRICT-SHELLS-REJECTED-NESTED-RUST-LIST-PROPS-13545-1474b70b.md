### Fixed - strict shells rejected nested Rust list props (#13545)

Native-complete Flutter shells now rebuild nested JSON lists at every declared
generic depth before passing them to generated widgets. Dart's JSON decoder
produces `List<dynamic>` values, so the previous exact
`value is List<List<String>>` check rejected valid Rust projections such as
TaskApp's `project-rows` during the first real render. The recursive decoder
preserves the declared `List<List<T>>` shape, keeps flat-list behavior intact,
and reports indexed paths when a nested value has the wrong shape.

Verified by regenerating the real TaskApp with its Rust runtime and driving the
Flutter controls through create, due-date scheduling, complete/reopen/delete,
and persisted restart widget tests.

