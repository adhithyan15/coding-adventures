### Changed — backend lists are derived, not copied

- `Backend::NATIVE` (the native backends in `Backend::ALL` order), and `Backend::is_native`, `Backend::dir_name` and `Backend::from_dir_name` are public, so package gates, the CLI and build scripts derive their backend lists instead of copying them. `native_is_all_filtered_by_is_native` pins `NATIVE`; `every_backend_round_trips_through_its_dir_name` pins the names.
