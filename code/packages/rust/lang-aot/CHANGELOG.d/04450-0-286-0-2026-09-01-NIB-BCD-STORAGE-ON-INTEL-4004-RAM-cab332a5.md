## 0.286.0 - 2026-09-01 (Nib BCD storage on Intel 4004 RAM)

Nib's one-nibble `bcd` statics now execute on both the seven standard LANG
backends and the in-tree Intel 4004 simulator. The 4004 AOT path builds one
module-wide global-slot map and lowers typed global stores/loads to the chip's
real `DCL`/`FIM`/`SRC` plus RAM data instructions.

