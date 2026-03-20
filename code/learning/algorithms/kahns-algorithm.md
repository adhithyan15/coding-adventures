# Kahn's Algorithm

Kahn's algorithm is a classic way to perform a **topological sort** on a directed acyclic graph (DAG).

That sentence is compact, so let us unpack it carefully.

## The Problem It Solves

Imagine you have a set of things with dependencies:

- package `arithmetic` depends on `logic-gates`
- package `cpu-simulator` depends on `arithmetic`
- package `pipeline` depends on `cache`, `branch-predictor`, and `hazard-detection`

You cannot build or run these in arbitrary order. You need an order where every dependency appears before the thing that needs it.

That is exactly what topological sorting gives you.

## What Is A Directed Graph?

A directed graph has:

- **nodes**: the things we care about
- **edges**: arrows between them

In this repository, a node might be a package and an edge means:

```text
A -> B
```

which we read as:

```text
B depends on A
```

That means `A` must come before `B`.

## What Is A Topological Order?

A topological order is a list of nodes such that for every edge:

```text
A -> B
```

`A` appears before `B`.

Example:

```text
logic-gates -> arithmetic -> cpu-simulator
```

Valid topological order:

```text
logic-gates, arithmetic, cpu-simulator
```

Invalid order:

```text
cpu-simulator, arithmetic, logic-gates
```

because `cpu-simulator` appears before the thing it depends on.

## Why The Graph Must Be A DAG

Kahn's algorithm only works if the graph is acyclic.

That means there must not be a cycle like:

```text
A -> B -> C -> A
```

Why is that a problem?

Because then:

- `A` must come before `B`
- `B` must come before `C`
- `C` must come before `A`

There is no possible ordering that satisfies all three constraints.

In build systems, cycles usually mean the dependency structure is broken.

## Core Idea Of Kahn's Algorithm

The algorithm repeatedly finds nodes with **in-degree 0**.

### What Is In-Degree?

The in-degree of a node is the number of arrows coming into it.

If edges mean "dependency comes first," then:

- in-degree 0 means "nothing depends on this node being later"
- equivalently, it means "this node has no unmet prerequisites"

Example graph:

```text
logic-gates -> arithmetic -> cpu-simulator
logic-gates -> cache
clock       -> cache
```

In-degrees:

- `logic-gates`: 0
- `clock`: 0
- `arithmetic`: 1
- `cpu-simulator`: 1
- `cache`: 2

The nodes with in-degree 0 are the ones we are allowed to schedule first.

## The Algorithm In Plain English

1. Compute the in-degree of every node.
2. Put all nodes with in-degree 0 into a queue.
3. Repeatedly:
   - remove one node from the queue
   - append it to the output order
   - for every node it points to, reduce that node's in-degree by 1
   - if any neighbor's in-degree becomes 0, add it to the queue
4. If you processed every node, you have a valid topological order.
5. If some nodes remain unprocessed, the graph had a cycle.

## Worked Example

Let us use a slightly richer graph:

```text
logic-gates -> arithmetic
logic-gates -> clock
arithmetic  -> cpu-simulator
clock       -> pipeline
cache       -> pipeline
branch-predictor -> pipeline
hazard-detection -> pipeline
```

We can visualize it like this:

```text
logic-gates ----> arithmetic ----> cpu-simulator
      |
      +---------> clock ---------+
                                 |
cache --------------------------> pipeline
branch-predictor ---------------> pipeline
hazard-detection ---------------> pipeline
```

### Step 1: Compute in-degrees

| Node | In-degree |
|------|-----------|
| `logic-gates` | 0 |
| `cache` | 0 |
| `branch-predictor` | 0 |
| `hazard-detection` | 0 |
| `arithmetic` | 1 |
| `clock` | 1 |
| `cpu-simulator` | 1 |
| `pipeline` | 4 |

### Step 2: Initialize queue with zero in-degree nodes

Queue:

```text
[logic-gates, cache, branch-predictor, hazard-detection]
```

Output:

```text
[]
```

### Step 3: Remove `logic-gates`

Output:

```text
[logic-gates]
```

Its outgoing edges go to:

- `arithmetic`
- `clock`

Reduce their in-degrees:

- `arithmetic`: 1 -> 0
- `clock`: 1 -> 0

Add them to the queue.

Queue:

```text
[cache, branch-predictor, hazard-detection, arithmetic, clock]
```

### Step 4: Remove `cache`

Output:

```text
[logic-gates, cache]
```

`cache -> pipeline`, so reduce:

- `pipeline`: 4 -> 3

Queue:

```text
[branch-predictor, hazard-detection, arithmetic, clock]
```

### Step 5: Remove `branch-predictor`

Output:

```text
[logic-gates, cache, branch-predictor]
```

Reduce:

- `pipeline`: 3 -> 2

### Step 6: Remove `hazard-detection`

Output:

```text
[logic-gates, cache, branch-predictor, hazard-detection]
```

Reduce:

- `pipeline`: 2 -> 1

### Step 7: Remove `arithmetic`

Output:

```text
[logic-gates, cache, branch-predictor, hazard-detection, arithmetic]
```

Reduce:

- `cpu-simulator`: 1 -> 0

Queue now includes `cpu-simulator`.

### Step 8: Remove `clock`

Output:

```text
[logic-gates, cache, branch-predictor, hazard-detection, arithmetic, clock]
```

Reduce:

- `pipeline`: 1 -> 0

Now `pipeline` can be queued too.

### Final result

One valid topological order is:

```text
logic-gates
cache
branch-predictor
hazard-detection
arithmetic
clock
cpu-simulator
pipeline
```

Notice something important:

There is not just one correct answer.

`cache` and `branch-predictor` can swap places.
`clock` and `arithmetic` can swap places.
Any ordering is valid as long as dependencies come first.

## Why Kahn's Algorithm Is So Good For Build Systems

The repository's build tools do not just need "some order." They need a practical order for execution.

Kahn's algorithm is a great fit because it naturally exposes:

- which packages are ready now
- which packages become ready after another finishes
- whether the dependency graph is broken

That aligns directly with how a build tool thinks.

### Connection To Parallel Execution

Suppose the queue currently contains:

```text
[cache, branch-predictor, hazard-detection]
```

Those nodes all have in-degree 0 at the same time.

That means they are independent at this moment and can be built in parallel.

This is how the repository's graph utilities derive **independent groups** or **parallel levels**.

Example:

```text
Level 0: logic-gates, cache, branch-predictor, hazard-detection
Level 1: arithmetic, clock
Level 2: cpu-simulator, pipeline
```

Kahn's algorithm is not just about ordering. It also gives a natural way to find batches of work that can run concurrently.

## Pseudocode

```text
kahns_topological_sort(graph):
    in_degree = count_incoming_edges(graph)
    queue = all_nodes_with_in_degree_zero(in_degree)
    order = []

    while queue is not empty:
        node = remove_one(queue)
        order.append(node)

        for neighbor in outgoing_edges(node):
            in_degree[neighbor] -= 1
            if in_degree[neighbor] == 0:
                queue.add(neighbor)

    if len(order) != number_of_nodes(graph):
        error "graph has a cycle"

    return order
```

## Why It Detects Cycles

This is one of the nicest parts of the algorithm.

If a graph contains a cycle, then every node in that cycle always has at least one incoming edge from another node in the same cycle.

That means:

- none of those nodes ever reaches in-degree 0
- none of them ever enters the queue
- the queue eventually becomes empty before all nodes are processed

So cycle detection falls out naturally:

```text
processed node count < total node count
=> cycle exists
```

## Complexity

Let:

- `V` = number of nodes
- `E` = number of edges

Time complexity:

```text
O(V + E)
```

Why?

- every node is enqueued and dequeued at most once
- every edge is examined once when its source node is processed

Space complexity:

```text
O(V)
```

for the queue, in-degree table, and output order.

That is exactly what you want from a graph traversal in infrastructure code: simple, linear, predictable.

## Comparison With DFS-Based Topological Sort

Another common way to topologically sort a graph is depth-first search (DFS).

Both approaches are valid, but they feel different.

### DFS-based topological sort

- recurse through dependencies
- append nodes in postorder
- reverse the result

### Kahn's algorithm

- count in-degrees
- repeatedly remove currently available nodes

### Why Kahn often feels better for tooling

Kahn's algorithm matches operational questions more directly:

- what can I build right now?
- what becomes available after this finishes?
- can I run a level in parallel?

That makes it especially natural for build systems, schedulers, and dependency planners.

## A Small Realistic Build Example

Imagine these internal packages:

```text
go/directed-graph
go/build-tool
python/directed-graph
python/build-tool
typescript/logic-gates
typescript/arithmetic
typescript/cpu-simulator
```

Dependencies:

```text
go/directed-graph -> go/build-tool
python/directed-graph -> python/build-tool
typescript/logic-gates -> typescript/arithmetic
typescript/arithmetic -> typescript/cpu-simulator
```

Kahn's algorithm would let a build planner start with:

```text
go/directed-graph
python/directed-graph
typescript/logic-gates
```

because all of them have no unmet prerequisites.

Then later:

```text
go/build-tool
python/build-tool
typescript/arithmetic
```

and finally:

```text
typescript/cpu-simulator
```

This is exactly the kind of reasoning the repository's directed-graph package is supporting.

## Common Mistakes When Implementing It

### 1. Mixing up edge direction

If you accidentally encode edges as:

```text
dependent -> dependency
```

instead of:

```text
dependency -> dependent
```

your topological order will come out backwards for build planning.

Always decide what an edge means and stick to it.

In this repository, the most useful convention is:

```text
dependency -> dependent
```

because then the produced order is already a valid build order.

### 2. Forgetting isolated nodes

A node with no incoming and no outgoing edges must still appear in the result.

It has in-degree 0, so it belongs in the initial queue.

### 3. Mutating the original graph incorrectly

Many implementations reduce in-degrees in a side table rather than deleting edges from the graph directly.

That keeps the graph reusable and makes the algorithm easier to reason about.

### 4. Non-deterministic output

If multiple nodes have in-degree 0 at once and you use an unordered set, your output order may change between runs.

For tooling, deterministic output is usually better.

That is why many implementations sort the ready nodes before processing them.

## Mental Model

The cleanest mental model is:

Kahn's algorithm is a repeated "unlocking" process.

- Nodes start locked if they still have unmet prerequisites.
- A node becomes unlocked when all incoming dependencies have been satisfied.
- Once unlocked, it can be scheduled.
- Scheduling it may unlock more nodes.

That is why it feels so natural for builds, course prerequisites, task planning, and package dependency graphs.

## Where It Shows Up In This Repository

- `directed-graph` libraries
- monorepo build tools
- parallel build grouping
- dependency validation

The algorithm is not just an academic exercise here. It is part of how the repository figures out what can happen next.

## If You Want To Go Further

Once Kahn's algorithm feels intuitive, the next useful topics are:

- cycle detection in directed graphs
- transitive closure
- reachability and affected-node propagation
- strongly connected components
- scheduling with resource constraints

But Kahn's algorithm is the right first stop because it turns dependency graphs from static diagrams into executable plans.
