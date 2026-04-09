# coding_adventures_graph

An undirected graph data structure implementation from scratch.

## Where it fits in the stack

This package provides a foundational undirected graph data structure for use across the coding-adventures project.

## Installation

```bash
gem install coding_adventures_graph
```

For development:

```bash
bundle install
bundle exec rake test
```

## Quick Start

```ruby
require "coding_adventures_graph"

g = CodingAdventures::Graph::Graph.new
g.add_edge("A", "B")
g.add_edge("B", "C")

puts g.nodes    # ["A", "B", "C"]
puts g.edges    # [["A", "B"], ["B", "C"]]
```

## Running Tests

```bash
bundle exec rake test
```

Tests require 95%+ coverage.
