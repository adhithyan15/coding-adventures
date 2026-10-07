---
category: Repo policy / workflow reminders
---

# Project JSON fields before serialization instead of truncating a serialized string

While inspecting the CI plan, `ConvertTo-Json | Select-Object -First 1` still
printed a multi-megabyte object. The serializer returns one complete string;
selecting its first pipeline object does not select its first line or field.

Inspect property names and project the required platform, package and fields
before serializing. Serialize only that small object; do not rely on output
truncation to make a broad read useful. This keeps CI evidence readable and
avoids hiding relevant verdicts among unrelated package records.
