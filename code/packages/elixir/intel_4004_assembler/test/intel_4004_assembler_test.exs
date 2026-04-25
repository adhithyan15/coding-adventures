defmodule CodingAdventures.Intel4004AssemblerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Intel4004Assembler
  alias CodingAdventures.Intel4004Assembler.{AssemblerError, Encoder, ParsedLine}

  test "lexes labels, mnemonics, operands, and comments" do
    parsed = Intel4004Assembler.lex_line("loop: JCN 0x4, done ; comment")

    assert %ParsedLine{
             label: "loop",
             mnemonic: "JCN",
             operands: ["0x4", "done"],
             source: "loop: JCN 0x4, done ; comment"
           } = parsed
  end

  test "assembles a small program" do
    assert {:ok, <<0xD5, 0xB2, 0x01>>} =
             Intel4004Assembler.assemble("""
             ORG 0x000
             _start:
               LDM 5
               XCH R2
               HLT
             """)
  end

  test "assembles labels, padding, and current pc operands" do
    binary =
      Intel4004Assembler.assemble!("""
      ORG 0x002
      JUN done
      JCN 0x4, $
      done:
      HLT
      """)

    assert binary == <<0x00, 0x00, 0x40, 0x06, 0x14, 0x04, 0x01>>
  end

  test "encodes register, register-pair, and immediate families" do
    assert Encoder.encode_instruction("FIM", ["P0", "0xAB"], %{}, 0) == [0x20, 0xAB]
    assert Encoder.encode_instruction("SRC", ["P2"], %{}, 0) == [0x25]
    assert Encoder.encode_instruction("FIN", ["P2"], %{}, 0) == [0x34]
    assert Encoder.encode_instruction("JIN", ["P2"], %{}, 0) == [0x35]
    assert Encoder.encode_instruction("ADD", ["R3"], %{}, 0) == [0x83]
    assert Encoder.encode_instruction("SUB", ["R4"], %{}, 0) == [0x94]
    assert Encoder.encode_instruction("LD", ["R5"], %{}, 0) == [0xA5]
    assert Encoder.encode_instruction("INC", ["R6"], %{}, 0) == [0x66]
    assert Encoder.encode_instruction("BBL", ["9"], %{}, 0) == [0xC9]
    assert Encoder.encode_instruction("ADD_IMM", ["R0", "R2", "7"], %{}, 0) == [0xD7, 0x82]
  end

  test "encodes jumps, subroutine calls, and indexed skips" do
    symbols = %{"target" => 0x123, "near" => 0x23}

    assert Encoder.encode_instruction("JUN", ["target"], symbols, 0) == [0x41, 0x23]
    assert Encoder.encode_instruction("JMS", ["target"], symbols, 0) == [0x51, 0x23]
    assert Encoder.encode_instruction("ISZ", ["R2", "near"], symbols, 0) == [0x72, 0x23]
  end

  test "rejects bad operands and out-of-range values" do
    assert {:error, %AssemblerError{message: "Undefined label: 'missing'"}} =
             Intel4004Assembler.assemble("JUN missing")

    assert_raise AssemblerError, fn ->
      Encoder.encode_instruction("FIM", ["P0", "0x1FF"], %{}, 0)
    end

    assert_raise AssemblerError, fn -> Encoder.encode_instruction("ADD", ["R16"], %{}, 0) end
    assert_raise AssemblerError, fn -> Encoder.encode_instruction("SRC", ["P8"], %{}, 0) end
    assert_raise AssemblerError, fn -> Encoder.encode_instruction("NOP", ["R0"], %{}, 0) end
    assert_raise AssemblerError, fn -> Encoder.instruction_size("BOGUS") end
    assert_raise AssemblerError, fn -> Intel4004Assembler.assemble!("ORG 0x1000\nHLT") end
  end
end
