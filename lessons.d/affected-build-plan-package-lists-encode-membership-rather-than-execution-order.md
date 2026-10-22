---
category: BUILD files & dependency management
---

# Affected build-plan package lists encode membership rather than execution order

A new TE04 fixture-routing test compared the affected package array to a fixed
producer-before-consumer order. The first full test run passed, but coverage
exposed the reverse order: `affectedListFromSet` deliberately projects a Go map.
The plan stores dependency ordering separately in `dependency_edges`.

Assert affected membership using a sorted copy or a set, while checking the
actual dependency relation independently. Preserve null versus empty array
semantics. Repeat the focused test to expose accidental dependence on map order;
do not change production scheduling to satisfy an assertion about an unordered
projection.
