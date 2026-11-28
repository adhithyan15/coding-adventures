# Changelog

## Unreleased

- Route the historical Unix file-read fixture to the existing Ruby simulator
  under its discovered package identity `ruby/unix_history_simulator`. The
  package test reads only the canonical fixture, so no replay helper, adapter,
  or driver path is added to the Ruby input hash.
