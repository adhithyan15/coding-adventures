---
category: Testing & coverage
---

# Long division over little-endian limbs must process them from most significant to least significant

For a little-endian limb array, small-divisor long division must reverse the
limbs first and carry the remainder from the most significant limb toward the
least significant one. Recursing to the tail before computing the current
limb propagates the remainder in the wrong direction. Test wide rank vectors
beyond machine integer width so an incorrect limb order cannot hide in small
samples.
