## [Unreleased]

### Changed

- Test-only: `RippleCarryAdderValidatesInputs` uses `Assert.ThrowsAny<ArgumentOutOfRangeException>` because logic-gates now throws `InvalidBitException`, a subclass of `ArgumentOutOfRangeException`. No library behavior change.
