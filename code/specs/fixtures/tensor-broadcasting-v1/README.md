# Tensor Broadcasting V1 Fixtures

This NN26 corpus makes right-aligned tensor broadcasting and its reverse
gradient reduction concrete. It covers two-sided expansion, leading-rank
padding, scalar expansion, and one deterministic shape mismatch. Compatible
cases pin every output-to-input coordinate mapping and central finite-
difference audit.

Validate it with:

```text
python code/scripts/validate_tensor_broadcasting_labs.py
```
