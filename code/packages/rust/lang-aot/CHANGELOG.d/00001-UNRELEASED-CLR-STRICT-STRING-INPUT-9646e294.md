## Unreleased — strict CLR encoded string input (CLR14)

Execute an opt-in strict scalar artifact that reads exact string bytes, moves
the string through a direct call, and returns the same simulator arena handle.
The proof covers reserved MemberRef dispatch and `string` metadata without
changing default source routing or enabling broader encoded string operations.
