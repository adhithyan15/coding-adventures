# Tiny Message Passing V1 Fixtures

This corpus pins one synchronous message-passing round on a three-node
undirected path. Each edge expands into two directed messages, every message
uses the original source feature, each target sums its inbox, and one shared
affine-plus-ReLU rule updates the nodes.

Validate from the repository root:

```text
python code/scripts/validate_tiny_message_passing_labs.py
```
