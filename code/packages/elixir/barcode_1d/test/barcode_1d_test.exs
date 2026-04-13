defmodule CodingAdventures.Barcode1DTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.Barcode1D

  test "renders a code39 barcode to PNG when Metal is available" do
    case Barcode1D.current_backend() do
      {:ok, :metal} ->
        assert {:ok, png} = Barcode1D.render_png("HELLO-123", symbology: :code39)
        assert binary_part(png, 0, 8) == <<137, 80, 78, 71, 13, 10, 26, 10>>
        assert byte_size(png) > 100

      {:error, reason} ->
        assert Barcode1D.render_png("HELLO-123", symbology: :code39) == {:error, reason}
      end
  end

  test "builds a code39 paint scene" do
    assert {:ok, scene} = Barcode1D.build_scene("HELLO-123", symbology: :code39)
    assert scene.width > 0
    assert scene.height > 0
    assert scene.background == "#ffffff"
    assert is_list(scene.instructions)
    assert length(scene.instructions) > 0
  end

  test "accepts string symbology names" do
    assert {:ok, scene} = Barcode1D.build_scene("HELLO-123", symbology: "code39")
    assert scene.width > 0
  end

  test "resolves backend selection for supported and future platforms" do
    assert Barcode1D.current_backend({:unix, :darwin}, "aarch64-apple-darwin") == {:ok, :metal}

    assert Barcode1D.current_backend({:unix, :darwin}, "x86_64-apple-darwin") ==
             {:error, :metal_requires_apple_silicon}

    assert Barcode1D.current_backend({:win32, :nt}, "x86_64-pc-windows-msvc") ==
             {:error, :direct2d_not_implemented}

    assert Barcode1D.current_backend({:unix, :linux}, "x86_64-unknown-linux-gnu") ==
             {:error, :cairo_not_implemented}

    assert Barcode1D.current_backend({:unix, :freebsd}, "amd64-unknown-freebsd") ==
             {:error, :unsupported_os}
  end

  test "renders a code39 barcode to pixels when Metal is available" do
    case Barcode1D.current_backend() do
      {:ok, :metal} ->
        assert {:ok, pixels} = Barcode1D.render_pixels("HELLO-123", symbology: :code39)
        assert pixels.width > 0
        assert pixels.height > 0
        assert byte_size(pixels.data) == pixels.width * pixels.height * 4

      {:error, reason} ->
        assert Barcode1D.render_pixels("HELLO-123", symbology: :code39) == {:error, reason}
      end
  end

  test "returns a clean error for unsupported symbologies" do
    assert Barcode1D.render_png("HELLO-123", symbology: :ean_13) == {:error, :unsupported_symbology}
  end

  test "rejects non-binary barcode payloads" do
    assert Barcode1D.build_scene(12_345, symbology: :code39) == {:error, :unsupported_symbology}
  end
end
