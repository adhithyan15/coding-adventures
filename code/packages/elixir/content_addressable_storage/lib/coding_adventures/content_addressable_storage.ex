defmodule CodingAdventures.ContentAddressableStorage do
  @moduledoc """
  Content-Addressable Storage (CAS) for the coding-adventures monorepo.

  ## What Is Content-Addressable Storage?

  Ordinary storage maps a *name* to content: you ask for `photo.jpg`, you get
  that photo. CAS flips the relationship — you ask for the *hash of the content*,
  and you get that content back. The hash is both the address and the integrity
  check.

  ```
  Traditional:  name  ──►  content           (name can be reused; content can change)
  CAS:          hash  ──►  content           (hash is derived from content; cannot lie)
  ```

  The defining property: if you know the hash, you know the content. If the
  stored bytes don't hash to the address you asked for, the store is corrupt.
  This makes CAS *self-authenticating* — trust the hash, trust the data.

  Git's entire object model is built on CAS. Every file snapshot (blob),
  directory listing (tree), commit, and tag is stored by the SHA-1 hash of its
  serialized bytes. Two identical files → one object. A renamed file → zero new
  storage. The history graph is an immutable DAG of hashes pointing to hashes.

  ## Package Structure

  | Module                                    | Role                                          |
  |-------------------------------------------|-----------------------------------------------|
  | `CodingAdventures.ContentAddressableStorage.BlobStore`          | Behaviour (interface) for storage backends    |
  | `CodingAdventures.ContentAddressableStorage.Store`              | CAS logic: hashing, integrity, prefix search  |
  | `CodingAdventures.ContentAddressableStorage.LocalDiskStore`     | Filesystem backend with Git 2/38 layout       |
  | `CodingAdventures.ContentAddressableStorage.Hex`               | Hex encoding/decoding utilities               |
  | `CodingAdventures.ContentAddressableStorage.Error`              | Typed error reason catalogue                  |

  ## Quick Start

      backend = CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!("/tmp/objects")
      store   = CodingAdventures.ContentAddressableStorage.Store.new(backend)

      {:ok, key} = CodingAdventures.ContentAddressableStorage.Store.put(store, "hello, world")
      {:ok, data} = CodingAdventures.ContentAddressableStorage.Store.get(store, key)
      # data == "hello, world"

      # Abbreviated lookup (like `git show a3f4b2`)
      short = String.slice(CodingAdventures.ContentAddressableStorage.Hex.key_to_hex(key), 0, 8)
      {:ok, full_key} = CodingAdventures.ContentAddressableStorage.Store.find_by_prefix(store, short)
      # full_key == key

  ## SHA-1

  This package uses the project's own SHA-1 implementation
  (`CodingAdventures.Sha1`) rather than `:crypto.hash(:sha, data)`. Every step
  of the digest computation is visible and explained in that module.
  """
end
