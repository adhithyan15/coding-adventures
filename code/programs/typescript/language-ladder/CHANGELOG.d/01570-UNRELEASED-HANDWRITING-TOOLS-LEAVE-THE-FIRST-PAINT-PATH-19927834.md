## Unreleased — handwriting tools leave the first-paint path

Opening a verified letter now dynamically loads the authored pen-path registry,
TrueType parser, and live filmstrip renderer. The lightweight script inventory
still names letters at first paint, while unopened handwriting details no longer
preload a chunk that had grown to 500,505 bytes. The bundle gate now proves the
named handwriting chunk stays outside the preload set instead of raising the
500 kB ceiling whenever another cited letter lands.
