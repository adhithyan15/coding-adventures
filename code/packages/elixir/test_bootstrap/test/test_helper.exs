ExUnit.start(timeout: 120_000)

defmodule Fx do
  @moduledoc "Shared helpers: the COMMITTED pbt-v1 fixture is read by walking up, never copied."

  alias CodingAdventures.TestBootstrap.Pbt.Corpus

  def repo_root do
    walk(Path.expand(__DIR__))
  end

  defp walk(dir) do
    cond do
      File.regular?(Path.join(dir, "code/specs/fixtures/pbt-v1/cases.json")) -> dir
      Path.dirname(dir) == dir -> raise "could not find code/specs/fixtures/pbt-v1/cases.json above #{__DIR__}"
      true -> walk(Path.dirname(dir))
    end
  end

  def fixture_path(name \\ "cases.json"), do: Path.join([repo_root(), "code/specs/fixtures/pbt-v1", name])
  def raw, do: File.read!(fixture_path())
  # A fresh, independent parse of the committed corpus.
  def cases, do: Corpus.parse(raw())

  def int(n), do: %{"kind" => "int", "value" => Integer.to_string(n)}

  # -- the fuzz-v1 corpus (VER06n) -------------------------------------------------

  def fuzz_path(name \\ "cases.json"), do: Path.join([repo_root(), "code/specs/fixtures/fuzz-v1", name])
  def fuzz_raw, do: File.read!(fuzz_path())
  def fuzz_cases, do: CodingAdventures.TestBootstrap.Fuzz.Corpus.parse(fuzz_raw())

  def go_vectors_path,
    do: Path.join(repo_root(), "code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go")

  def hex(text), do: Base.decode16!(text, case: :lower)
  def unhex(bytes), do: Base.encode16(bytes, case: :lower)
  def bytes_node(bytes), do: %{"kind" => "bytes", "hex" => unhex(bytes)}

  # A seed whose FIRST next_u64 output is exactly `out` (the SplitMix64 finalizer
  # inverted), to force a chosen word, a rejected draw or a threshold draw.
  import Bitwise
  @mask 0xFFFF_FFFF_FFFF_FFFF
  @m1 0xBF58_476D_1CE4_E5B9
  @m2 0x94D0_49BB_1331_11EB
  @gamma 0x9E37_79B9_7F4A_7C15
  defp unxor(y, s), do: Enum.reduce(1..(div(64, s) + 1), y, fn _, acc -> bxor(y, acc >>> s) end)
  defp mul_inv(m), do: Enum.reduce(1..7, m, fn _, x -> band(x * (2 - band(m * x, @mask)), @mask) end)

  def seed_for(out) do
    z2 = unxor(out, 31)
    z1 = unxor(band(z2 * mul_inv(@m2), @mask), 27)
    s = unxor(band(z1 * mul_inv(@m1), @mask), 30)
    band(s - @gamma, @mask)
  end
end
