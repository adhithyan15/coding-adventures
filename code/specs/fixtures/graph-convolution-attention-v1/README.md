# Graph Convolution and Attention V1 Fixtures

This corpus compares degree-normalized graph convolution with stable-softmax
graph attention on the same self-looped three-node path. It pins every GCN
coefficient and contribution plus every GAT score, shift, exponential,
attention weight, contribution, and output.

```text
python code/scripts/validate_graph_convolution_attention_labs.py
```
