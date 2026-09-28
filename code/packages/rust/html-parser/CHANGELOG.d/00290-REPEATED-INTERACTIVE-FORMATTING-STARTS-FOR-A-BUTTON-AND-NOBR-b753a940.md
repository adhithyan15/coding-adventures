- Repeated interactive formatting starts for `a`, `button`, and `nobr` now
  close the previous open element before inserting the next one, avoiding
  impossible nested interactive DOMs for common omitted-end-tag markup.
