"""
================================================================
EMBEDDING — LOOKUP TABLE FOR DISCRETE TOKENS
================================================================

An Embedding layer converts discrete integer tokens (like word IDs)
into dense vector representations. It's essentially a lookup table:

    Token ID  →  Vector
    0         →  [0.12, -0.34, 0.56, ...]
    1         →  [-0.78, 0.91, 0.23, ...]
    ...

=== How It Works ===

The embedding is stored as a weight matrix of shape (num_embeddings, embedding_dim).
Looking up token i is simply extracting row i from this matrix.

    vocab_size = 10000      # number of unique tokens
    embed_dim = 256         # size of each embedding vector

    embed = Embedding(vocab_size, embed_dim)
    # embed.weight has shape (10000, 256)

    token_ids = [5, 42, 7]  # three tokens
    vectors = embed(token_ids)  # shape: (3, 256)

=== Why Embeddings Are Learned ===

The embedding vectors are Parameters — they're updated by backprop
during training. The network learns to place similar words close
together in the embedding space:

    king - man + woman ≈ queen   (the famous Word2Vec example)

================================================================
"""

from __future__ import annotations

from ml_framework_core import Parameter, Tensor

from .module import Module


class Embedding(Module):
    """Lookup table mapping integer indices to dense vectors.

    Args:
        num_embeddings: Size of the vocabulary (number of unique tokens)
        embedding_dim: Size of each embedding vector

    Input: list of integer token IDs (or a 1-D Tensor of indices)
    Output: Tensor of shape (len(indices), embedding_dim)

    Example:
        embed = Embedding(1000, 64)  # 1000 tokens, 64-dim vectors
        # Look up embeddings for tokens [5, 10, 15]
        indices = Tensor.from_list([5.0, 10.0, 15.0])
        vectors = embed(indices)  # shape: (3, 64)
    """

    def __init__(self, num_embeddings: int, embedding_dim: int) -> None:
        super().__init__()
        object.__setattr__(self, "num_embeddings", num_embeddings)
        object.__setattr__(self, "embedding_dim", embedding_dim)

        # The embedding weight matrix: each row is one token's vector
        # Initialized with random normal values (standard practice)
        self.weight = Parameter(Tensor.randn(num_embeddings, embedding_dim))

    def forward(self, indices: Tensor) -> Tensor:
        """Look up embedding vectors for the given token indices.

        Args:
            indices: 1-D Tensor of integer token IDs

        Returns:
            2-D Tensor of shape (num_indices, embedding_dim)

        Each index i selects row i from self.weight:
            output[k] = self.weight[indices[k]]
        """
        if indices.ndim != 1:
            raise ValueError(f"Embedding expects 1-D indices, got {indices.ndim}-D")

        num_indices = indices.shape[0]
        embed_dim = self.embedding_dim
        result = [0.0] * (num_indices * embed_dim)

        for k in range(num_indices):
            # Get the token index (convert float to int)
            idx = int(indices.data[k])
            if idx < 0 or idx >= self.num_embeddings:
                raise IndexError(
                    f"Token index {idx} out of range [0, {self.num_embeddings})"
                )
            # Copy the embedding vector for this token
            row_start = idx * embed_dim
            for j in range(embed_dim):
                result[k * embed_dim + j] = self.weight.data[row_start + j]

        return Tensor(result, (num_indices, embed_dim), device=indices.device)

    def __repr__(self) -> str:
        return f"Embedding({self.num_embeddings}, {self.embedding_dim})"
