---
category: QR / format-marker / file-format specifics
---

# OpenType `head` table is exactly 54 bytes

Missing the `xMin/yMin/xMax/yMax` quartet (8 bytes) makes it 46, mis-aligning every subsequent table offset.
