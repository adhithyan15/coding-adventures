# Recount wide and reference slots before accepting a stack-bound oracle

The first JVM mixed-width regression expected thirteen slots for one array
reference, four wide values and three other one-slot values. The correct sum
is twelve: 1 + 2 + 2 + 1 + 1 + 1 + 2 + 2. The first pre-repair log is retained
as an actual failure, but its thirteen-slot oracle is not acceptance evidence.
Correct the independent count and rerun the red control before implementation.
Runtime verification of the original frontend programs is a separate gate.
