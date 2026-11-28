# Changelog

## Unreleased

- Add the fixed-geometry immutable write-back cache with sector flush cuts and
  explicit power-loss behavior.
- Add ascending-LBA whole-set dirty barriers with ordered sector cuts and an
  acknowledgement only after every dirty sector reaches durable media.
