defmodule CodingAdventures.Barcode1DTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.Barcode1D
  alias CodingAdventures.PixelContainer

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

  test "accepts string names for additional symbologies" do
    cases = [
      {"codabar", "40156"},
      {"code128", "Code 128"},
      {"ean-13", "400638133393"},
      {"itf", "123456"},
      {"upc-a", "03600029145"}
    ]

    Enum.each(cases, fn {symbology, data} ->
      assert {:ok, scene} = Barcode1D.build_scene(data, symbology: symbology)
      assert scene.width > 0
    end)
  end

  test "builds scenes for additional symbologies" do
    cases = [
      {:codabar, "40156"},
      {:code128, "Code 128"},
      {:ean13, "400638133393"},
      {:itf, "123456"},
      {:upca, "03600029145"}
    ]

    Enum.each(cases, fn {symbology, data} ->
      assert {:ok, scene} = Barcode1D.build_scene(data, symbology: symbology)
      assert scene.width > 0
      assert scene.metadata.symbology
    end)
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

  test "renders pixels with injected executor hooks" do
    assert {:ok, pixels} =
             Barcode1D.render_pixels("HELLO-123",
               symbology: :code39,
               backend_result: {:ok, :metal},
               scene_executor: fn scene, :metal ->
                 {:ok,
                  %PixelContainer{
                    width: scene.width,
                    height: scene.height,
                    data: :binary.copy(<<0, 0, 0, 255>>, scene.width * scene.height)
                  }}
               end
             )

    assert pixels.width > 0
    assert pixels.height > 0
    assert byte_size(pixels.data) == pixels.width * pixels.height * 4
  end

  test "renders png with injected codec hooks" do
    assert {:ok, png} =
             Barcode1D.render_png("HELLO-123",
               symbology: :code39,
               backend_result: {:ok, :metal},
               scene_executor: fn scene, :metal ->
                 {:ok,
                  %PixelContainer{
                    width: scene.width,
                    height: scene.height,
                    data: :binary.copy(<<255, 255, 255, 255>>, scene.width * scene.height)
                  }}
               end,
               png_encoder: fn %PixelContainer{data: data} -> {:ok, "PNG:" <> data} end
             )

    assert String.starts_with?(png, "PNG:")
  end

  test "returns a clean error for unsupported symbologies" do
    assert Barcode1D.render_png("HELLO-123", symbology: :qr) == {:error, :unsupported_symbology}
  end

  test "rejects non-binary barcode payloads" do
    assert Barcode1D.build_scene(12_345, symbology: :code39) == {:error, :unsupported_symbology}
  end
end
