# Attention QKV V1 Fixtures

This corpus pins one three-token self-attention projection and query-key score
trace for NN12. It stops before softmax, masking, or value mixing.

```text
python code/scripts/validate_attention_qkv_labs.py
```

See [`NN12-attention-qkv-labs.md`](../../NN12-attention-qkv-labs.md).
