---
category: Rust
---

# Audit automatic merge bodies for duplicate public fields

Independent whole-main review found that the virtual diagram-ir merge duplicated
TimelineConfig.font_size and font_family and their Default initializers. Both
branches added the same semantic fields at different positions, so Git reported
no textual conflict. This is a static compile-contract defect; no incoming code
was compiled or claimed accepted. The complete twenty-overlap audit found it
beyond the eleven paths with reported conflict markers.

Review complete base/owned/main/virtual bodies for every overlap, not just marker
regions. Resolve to one exact owned field/default per semantic name, preserving
field order, and union genuinely new architecture fields with a complete affected
constructor census. Close typed/serialization products and actual builds before
adoption. A marker-free virtual merge is a candidate, never validation evidence.
