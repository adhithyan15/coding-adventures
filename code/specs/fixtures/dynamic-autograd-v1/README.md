# Dynamic Autograd V1 Fixtures

This NN27 corpus makes an executed reverse-mode graph inspectable. It pins one
complete multiply-add-square graph, one runtime branch, and one post-forward
mutation that demonstrates immutable saved snapshots.

Validate it with:

```text
python code/scripts/validate_dynamic_autograd_labs.py
```
