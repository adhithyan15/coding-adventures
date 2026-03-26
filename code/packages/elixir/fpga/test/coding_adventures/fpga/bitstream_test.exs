defmodule CodingAdventures.FPGA.BitstreamTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.Bitstream

  describe "from_map/1" do
    test "parses full config" do
      config = %{
        "clbs" => %{"0_0" => %{"slice_0" => %{"lut_a" => [0, 0, 0, 1]}}},
        "routing" => %{"0_0" => %{"out_0" => "in_1"}},
        "io" => %{"pin_0" => %{"direction" => "input"}}
      }

      bs = Bitstream.from_map(config)
      assert Map.has_key?(bs.clb_configs, "0_0")
      assert Map.has_key?(bs.routing_configs, "0_0")
      assert Map.has_key?(bs.io_configs, "pin_0")
    end

    test "defaults missing keys to empty maps" do
      bs = Bitstream.from_map(%{})
      assert bs.clb_configs == %{}
      assert bs.routing_configs == %{}
      assert bs.io_configs == %{}
    end
  end

  describe "clb_config/2" do
    test "returns config for existing key" do
      bs = Bitstream.from_map(%{"clbs" => %{"0_0" => %{"data" => 42}}})
      assert Bitstream.clb_config(bs, "0_0") == %{"data" => 42}
    end

    test "returns nil for missing key" do
      bs = Bitstream.from_map(%{})
      assert Bitstream.clb_config(bs, "0_0") == nil
    end
  end

  describe "routing_config/2" do
    test "returns config for existing key" do
      bs = Bitstream.from_map(%{"routing" => %{"1_1" => %{"out_0" => "in_2"}}})
      assert Bitstream.routing_config(bs, "1_1") == %{"out_0" => "in_2"}
    end

    test "returns nil for missing key" do
      bs = Bitstream.from_map(%{})
      assert Bitstream.routing_config(bs, "1_1") == nil
    end
  end

  describe "io_config/2" do
    test "returns config for existing pin" do
      bs = Bitstream.from_map(%{"io" => %{"pin_3" => %{"direction" => "output"}}})
      assert Bitstream.io_config(bs, "pin_3") == %{"direction" => "output"}
    end

    test "returns nil for missing pin" do
      bs = Bitstream.from_map(%{})
      assert Bitstream.io_config(bs, "pin_3") == nil
    end
  end
end
