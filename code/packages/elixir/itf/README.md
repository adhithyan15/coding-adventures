# coding_adventures_itf

Interleaved 2 of 5 encoding with shared 1D barcode layout and backend-neutral
paint scenes. Validation follows `barcode-symbologies-v1`: at most 4,096
Unicode scalars, then non-empty even length, then ASCII digits. Failures expose
stable language-neutral identifiers.

Run `mix test` from this directory.
