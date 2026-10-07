## Added native table pointer focus restoration

Native data-cell container taps now capture logical coordinates before authored activation and reacquire pointer focus afterwards, so adapter row replacement does not leave keyboard input in the formula field. Non-table taps retain their existing behavior. Native interaction acceptance remains required (#14278).
