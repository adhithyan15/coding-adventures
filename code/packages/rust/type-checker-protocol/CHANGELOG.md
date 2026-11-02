# Changelog

## Unreleased — #18474 security review: bounded diagnostics

- New `bounded` module. `push_bounded` keeps at most `MAX_DIAGNOSTICS`
  (256) diagnostics and `MAX_DIAGNOSTIC_BYTES` (1 MiB) of messages, cuts a
  message at `MAX_DIAGNOSTIC_MESSAGE_BYTES` (4 KiB), and appends the
  `DIAGNOSTICS_TRUNCATED` note once when one is dropped.
  `diagnostics_closed` lets a checker skip work whose only product would be
  dropped. `name_list` / `name_list_with` name at most `MAX_LISTED_NAMES`
  (8) items, then "… and K more". Unit tests cover each limit, at the limit
  and one past it.

## Initial release

- Initial Rust port of the shared type-checker protocol and hook dispatcher.
