# Truncated child fixtures must end at the intended framing boundary

When testing a parent DER cursor's framing error, end the outer value immediately after the truncated child bytes. Appending later schema fields can accidentally satisfy the child's declared length, turning the intended parent-cursor failure into a nested semantic error. Construct the smallest outer container containing only the incomplete child, then assert the whole-container offset separately.
