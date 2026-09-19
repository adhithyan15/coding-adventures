## Unreleased - CLR01 encoded wide immediate refusal

Encoded CLR compilation now refuses wide integer literals instead of silently
truncating them. Source-level tests verify refusal of 4294967296 and execute
both signed 32-bit boundaries on the encoded simulator. Textual CoreCLR remains
separate; no full-width encoded arithmetic or input support is claimed.
