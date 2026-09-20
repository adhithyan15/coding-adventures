# Inspect JSON field types before sorting reporter data

A diagnostic tried to sort the package-parity report's `coverage` value as if
it were a string-keyed mapping, but the schema supplies a list of objects.
Before performing summary operations on structured CLI output, inspect each
field's runtime type and one representative value. Select explicit scalar keys
from object rows instead of relying on a guessed container shape.
