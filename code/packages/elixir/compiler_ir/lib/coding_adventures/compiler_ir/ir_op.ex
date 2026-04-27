defmodule CodingAdventures.CompilerIr.IrOp do
  @moduledoc """
  IR Opcode constants for the general-purpose intermediate representation.

  ## Design

  Opcodes are represented as Elixir atoms, which is idiomatic Elixir
  and avoids the need for an integer enum. Atoms are:

  - Fast to compare (pointer equality)
  - Self-documenting in pattern matches
  - Internable — the atom table grows by at most N atoms total

  ## Opcode groups

  ### Constants
  - `:load_imm`   — load integer literal into register
  - `:load_addr`  — load address of data label into register

  ### Memory
  - `:load_byte`  — load byte from memory (zero-extended)
  - `:store_byte` — store byte to memory
  - `:load_word`  — load machine word from memory
  - `:store_word` — store machine word to memory

  ### Arithmetic
  - `:add`        — register + register
  - `:add_imm`    — register + immediate
  - `:sub`        — register - register
  - `:and`        — register & register (bitwise AND)
  - `:and_imm`    — register & immediate (bitwise AND)

  ### Comparison (produce 0 or 1)
  - `:cmp_eq`     — 1 if lhs == rhs
  - `:cmp_ne`     — 1 if lhs != rhs
  - `:cmp_lt`     — 1 if lhs < rhs (signed)
  - `:cmp_gt`     — 1 if lhs > rhs (signed)

  ### Control Flow
  - `:label`      — define a label (no machine code)
  - `:jump`       — unconditional jump
  - `:branch_z`   — jump if register == 0
  - `:branch_nz`  — jump if register != 0
  - `:call`       — call subroutine
  - `:ret`        — return from subroutine

  ### System
  - `:syscall`    — system call (platform ABI)
  - `:halt`       — terminate execution

  ### Meta
  - `:nop`        — no operation
  - `:comment`    — human-readable comment (no machine code)

  ## Text names

  The `to_string/1` and `parse/1` functions provide roundtrip-safe
  conversion between atoms and their UPPER_SNAKE_CASE text names (e.g.
  `:load_imm` ↔ `"LOAD_IMM"`). These names are the canonical form used
  in `.ir` text files.
  """

  # ── All valid opcodes ────────────────────────────────────────────────────────

  @type t ::
          :load_imm
          | :load_addr
          | :load_byte
          | :store_byte
          | :load_word
          | :store_word
          | :add
          | :add_imm
          | :sub
          | :and
          | :and_imm
          | :cmp_eq
          | :cmp_ne
          | :cmp_lt
          | :cmp_gt
          | :label
          | :jump
          | :branch_z
          | :branch_nz
          | :call
          | :ret
          | :syscall
          | :halt
          | :nop
          | :comment

  # ── Opcode ↔ text name mapping ───────────────────────────────────────────────
  #
  # This map drives both the printer (atom → string) and the parser
  # (string → atom). Keeping a single source of truth prevents drift.

  @op_names %{
    load_imm: "LOAD_IMM",
    load_addr: "LOAD_ADDR",
    load_byte: "LOAD_BYTE",
    store_byte: "STORE_BYTE",
    load_word: "LOAD_WORD",
    store_word: "STORE_WORD",
    add: "ADD",
    add_imm: "ADD_IMM",
    sub: "SUB",
    and: "AND",
    and_imm: "AND_IMM",
    cmp_eq: "CMP_EQ",
    cmp_ne: "CMP_NE",
    cmp_lt: "CMP_LT",
    cmp_gt: "CMP_GT",
    label: "LABEL",
    jump: "JUMP",
    branch_z: "BRANCH_Z",
    branch_nz: "BRANCH_NZ",
    call: "CALL",
    ret: "RET",
    syscall: "SYSCALL",
    halt: "HALT",
    nop: "NOP",
    comment: "COMMENT"
  }

  # Inverted map: "LOAD_IMM" → :load_imm
  @name_to_op Map.new(@op_names, fn {atom, name} -> {name, atom} end)

  @doc """
  Return all valid opcodes as a list.

  ## Examples

      iex> IrOp.all() |> Enum.member?(:load_imm)
      true
  """
  @spec all() :: [t()]
  def all, do: Map.keys(@op_names)

  @doc """
  Convert an opcode atom to its canonical UPPER_SNAKE_CASE text name.

  Returns `"UNKNOWN"` if the opcode is not recognised.

  ## Examples

      iex> IrOp.to_string(:load_imm)
      "LOAD_IMM"

      iex> IrOp.to_string(:branch_nz)
      "BRANCH_NZ"

      iex> IrOp.to_string(:not_an_opcode)
      "UNKNOWN"
  """
  @spec to_string(t() | atom()) :: String.t()
  def to_string(op), do: Map.get(@op_names, op, "UNKNOWN")

  @doc """
  Parse an UPPER_SNAKE_CASE opcode name into an atom.

  Returns `{:ok, atom}` on success, `{:error, :unknown_opcode}` if the
  name is not recognised.

  ## Examples

      iex> IrOp.parse("LOAD_IMM")
      {:ok, :load_imm}

      iex> IrOp.parse("BRANCH_NZ")
      {:ok, :branch_nz}

      iex> IrOp.parse("BOGUS")
      {:error, :unknown_opcode}
  """
  @spec parse(String.t()) :: {:ok, t()} | {:error, :unknown_opcode}
  def parse(name) do
    case Map.get(@name_to_op, name) do
      nil -> {:error, :unknown_opcode}
      op -> {:ok, op}
    end
  end
end
