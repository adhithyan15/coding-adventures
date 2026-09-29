- **A drag cannot be stranded.** `pointercancel` is handled (the browser reclaims
  touch gestures it decides are scrolls), and capture covers release outside the
  root. A stuck drag is not benign: the next Space would release instead of
  grabbing, and the next press would emit a second `onDragStart` with no
  intervening `onDragEnd`.
