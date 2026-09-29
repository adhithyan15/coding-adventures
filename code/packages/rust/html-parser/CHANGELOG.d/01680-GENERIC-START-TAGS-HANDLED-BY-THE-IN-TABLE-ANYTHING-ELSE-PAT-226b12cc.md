- Generic start tags handled by the in-table "anything else" path, including
  paragraph-boundary elements, `br`, `p`, and `plaintext`, now report the
  required parse error before retaining their foster-parented DOM placement.
