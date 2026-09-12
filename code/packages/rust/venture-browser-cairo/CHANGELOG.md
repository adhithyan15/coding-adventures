# Changelog

## Unreleased

- Expose shared live output and normalized meter/progress accessibility state
  to Qt, Flutter, and Compose hosts.

- Expose shared bounded datalist state and semantic picker action ABI seams to
  Qt, Flutter, and Compose hosts.

- Expose shared file picker request and selected-byte ABI seams for Qt,
  Flutter, and Compose adapters.

- Route form submit/reset activation through the shared validation and
  transactional GET/POST navigation pipeline.

- Route Qt, Flutter, and Compose surface clicks through shared form-control
  activation before link navigation.

- Report native session startup errors to stderr before returning a null handle
  to Qt, Flutter, or Compose, so failing navigation can be diagnosed.

## 0.9.1

- Coordinate the package with Venture's first immutable pre-1.0 release.

## 0.9.0

- Drain completion-discovered stylesheet imports through the same shared
  lifecycle used by the Qt, Flutter, and Compose host family.
- Expose the shared document-first navigation and incremental image completion
  seam for the Qt, Flutter, and Compose host family.
- Serialize shared View Source auxiliary-document effects for Qt, Flutter, and
  Compose hosts without moving source construction into toolkit adapters.

- Load and atomically persist the shared bookmark catalog for generated Qt,
  Flutter, and Compose hosts, with durable restart acceptance in the common
  Cairo bridge.
- Extract the shared Venture browser controller, Cairo renderer, and native C
  ABI compatibility exports from the Qt adapter into a backend-neutral crate.
