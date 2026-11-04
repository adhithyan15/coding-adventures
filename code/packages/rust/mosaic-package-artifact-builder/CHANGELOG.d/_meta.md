# Changelog

## Unreleased

- Add an explicit default-off deferred XAML package experiment, routed through all exports, layout variants and project roots; reject it for other backends before emission. Existing builder calls remain eager.

