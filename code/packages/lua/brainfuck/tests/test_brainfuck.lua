-- ============================================================================
-- Tests for brainfuck — interpreter, compiler, and validator
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. validate() — balanced and unbalanced brackets.
-- 2. compile_to_opcodes() — correct opcode mapping, jump targets.
-- 3. run_opcodes() — execution of all 8 commands.
-- 4. interpret() — end-to-end integration tests.
-- 5. Cell wrapping — 255+1=0, 0-1=255.
-- 6. Input / EOF — reads bytes, sets 0 on EOF.
-- 7. Classic programs — "H" (72 +'s), echo, hello world excerpt.
-- ============================================================================

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local bf = require("coding_adventures.brainfuck")

describe("Brainfuck", function()

    -- -----------------------------------------------------------------------
    -- validate()
    -- -----------------------------------------------------------------------
    describe("validate()", function()

        it("accepts an empty program", function()
            local ok, err = bf.validate("")
            assert.is_true(ok)
            assert.is_nil(err)
        end)

        it("accepts a program with no brackets", function()
            local ok, _ = bf.validate("+++--->>><<<")
            assert.is_true(ok)
        end)

        it("accepts properly nested brackets", function()
            assert.is_true((bf.validate("[[][]]")))
            assert.is_true((bf.validate("[[[]]]")))
            assert.is_true((bf.validate("[]")))
        end)

        it("rejects a lone [", function()
            local ok, err = bf.validate("[")
            assert.is_false(ok)
            assert.is_not_nil(err)
        end)

        it("rejects a lone ]", function()
            local ok, err = bf.validate("]")
            assert.is_false(ok)
            assert.is_not_nil(err)
        end)

        it("rejects extra ] after matching pairs", function()
            local ok, err = bf.validate("[][]]]")
            assert.is_false(ok)
            assert.is_not_nil(err)
        end)

        it("rejects mismatched count [[ ]", function()
            local ok, _ = bf.validate("[[+]")
            assert.is_false(ok)
        end)

        it("ignores non-command characters in bracket counting", function()
            local ok, _ = bf.validate("Hello [World]!")
            assert.is_true(ok)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- compile_to_opcodes()
    -- -----------------------------------------------------------------------
    describe("compile_to_opcodes()", function()

        it("maps each command to the correct opcode", function()
            local ops, err = bf.compile_to_opcodes("><+-.,")
            assert.is_nil(err)
            -- 6 commands + 1 HALT
            assert.are.equal(7, #ops)
            assert.are.equal(bf.OP_RIGHT,  ops[1].op)
            assert.are.equal(bf.OP_LEFT,   ops[2].op)
            assert.are.equal(bf.OP_INC,    ops[3].op)
            assert.are.equal(bf.OP_DEC,    ops[4].op)
            assert.are.equal(bf.OP_OUTPUT, ops[5].op)
            assert.are.equal(bf.OP_INPUT,  ops[6].op)
            assert.are.equal(bf.OP_HALT,   ops[7].op)
        end)

        it("ignores non-command characters", function()
            -- "Hello" is a comment; only + and . are commands
            local ops, err = bf.compile_to_opcodes("Hello+World.")
            assert.is_nil(err)
            assert.are.equal(3, #ops)  -- +, ., HALT
        end)

        it("sets correct jump targets for []", function()
            -- Program: [+]
            -- Index:    1 2 3(HALT)
            --           [ + ] HALT
            local ops, err = bf.compile_to_opcodes("[+]")
            assert.is_nil(err)
            -- [ is at index 1: if cell==0, jump to instruction after ] = index 4
            assert.are.equal(4, ops[1].operand)
            -- ] is at index 3: if cell!=0, jump back to [ = index 1
            assert.are.equal(1, ops[3].operand)
        end)

        it("handles nested brackets", function()
            -- Program: [[]]
            -- Index:    1 2 3 4 5(HALT)
            --           [ [ ] ] HALT
            local ops, err = bf.compile_to_opcodes("[[]]")
            assert.is_nil(err)
            -- outer [  (idx 1): jumps to instruction after outer ] (idx 5)
            assert.are.equal(5, ops[1].operand)
            -- inner [  (idx 2): jumps to instruction after inner ] (idx 4)
            assert.are.equal(4, ops[2].operand)
            -- inner ]  (idx 3): jumps back to inner [ (idx 2)
            assert.are.equal(2, ops[3].operand)
            -- outer ]  (idx 4): jumps back to outer [ (idx 1)
            assert.are.equal(1, ops[4].operand)
        end)

        it("returns error on unbalanced brackets", function()
            local ops, err = bf.compile_to_opcodes("[")
            assert.is_nil(ops)
            assert.is_not_nil(err)
        end)

        it("appends HALT at the end", function()
            local ops, _ = bf.compile_to_opcodes("+++")
            assert.are.equal(bf.OP_HALT, ops[#ops].op)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- interpret() — basic operations
    -- -----------------------------------------------------------------------
    describe("interpret()", function()

        it("+++++. outputs char(5) (ENQ)", function()
            local out, err = bf.interpret("+++++.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(5), out)
        end)

        it("72 increments produce 'H' (ASCII 72)", function()
            local prog = string.rep("+", 72) .. "."
            local out, err = bf.interpret(prog, "")
            assert.is_nil(err)
            assert.are.equal("H", out)
        end)

        it("produces multiple output characters", function()
            -- ++ . + . = char(2) char(3)
            local out, err = bf.interpret("++.+.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(2) .. string.char(3), out)
        end)

        it("handles empty program with no output", function()
            local out, err = bf.interpret("", "")
            assert.is_nil(err)
            assert.are.equal("", out)
        end)

        it("non-command characters are treated as comments", function()
            -- "Hello" comment surrounds the actual ++ . command
            local out, err = bf.interpret("Hello ++. World", "")
            assert.is_nil(err)
            assert.are.equal(string.char(2), out)
        end)

        it("returns error for unbalanced brackets", function()
            local out, err = bf.interpret("[", "")
            assert.is_nil(out)
            assert.is_not_nil(err)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Cell wrapping
    -- -----------------------------------------------------------------------
    describe("cell wrapping", function()

        it("incrementing 255 wraps to 0", function()
            -- Set cell to 255: 255 increments. Then increment one more, output.
            local prog = string.rep("+", 255) .. "+."
            local out, err = bf.interpret(prog, "")
            assert.is_nil(err)
            assert.are.equal(string.char(0), out)
        end)

        it("decrementing 0 wraps to 255", function()
            -- Start at 0, decrement, output → char(255)
            local out, err = bf.interpret("-.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(255), out)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Loops
    -- -----------------------------------------------------------------------
    describe("loops", function()

        it("[+] loop is skipped when cell is 0", function()
            -- Cell starts at 0, so [+] body never executes. Output: char(0)
            local out, err = bf.interpret("[+].", "")
            assert.is_nil(err)
            assert.are.equal(string.char(0), out)
        end)

        it("loop decrements cell to zero: +++[-]", function()
            -- Set cell to 3, loop decrementing until 0, then output.
            -- Result: char(0)
            local out, err = bf.interpret("+++[-].", "")
            assert.is_nil(err)
            assert.are.equal(string.char(0), out)
        end)

        it("loop copies cell: [->+<]", function()
            -- Classic copy: cell[0] = 3, loop moves value to cell[1]
            -- After: cell[0]=0, cell[1]=3.  Output cell[1]: char(3)
            local out, err = bf.interpret("+++[->+<]>.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(3), out)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Input / EOF
    -- -----------------------------------------------------------------------
    describe("input and EOF", function()

        it("reads input byte into cell", function()
            -- , reads 'A' (65), then . outputs it
            local out, err = bf.interpret(",.", "A")
            assert.is_nil(err)
            assert.are.equal("A", out)
        end)

        it(",[.,] echoes the entire input string", function()
            -- cat program: read-while-not-zero loop
            -- Note: relies on EOF setting cell to 0
            local out, err = bf.interpret(",[.,]", "hello")
            assert.is_nil(err)
            assert.are.equal("hello", out)
        end)

        it("sets cell to 0 on EOF", function()
            -- Read past end of input, then output — should be char(0)
            local out, err = bf.interpret(",.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(0), out)
        end)

        it("reads multiple input bytes in sequence", function()
            -- Two reads, two outputs
            local out, err = bf.interpret(",.,.","AB")
            assert.is_nil(err)
            assert.are.equal("AB", out)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Data pointer movement
    -- -----------------------------------------------------------------------
    describe("data pointer", function()

        it("> moves to cell 2; . outputs its initial value 0", function()
            local out, err = bf.interpret(">.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(0), out)
        end)

        it("cells are independent: set cell[0]=1, cell[1]=2, output both", function()
            -- +.>++.
            local out, err = bf.interpret("+.>++.", "")
            assert.is_nil(err)
            assert.are.equal(string.char(1) .. string.char(2), out)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Hello World (abbreviated)
    -- -----------------------------------------------------------------------
    describe("Hello World excerpt", function()

        it("produces 'H' using multiplication loop", function()
            -- Classic: +++++++++[>++++++++<-]>. = 9*8 = 72 = 'H'
            local out, err = bf.interpret("+++++++++[>++++++++<-]>.", "")
            assert.is_nil(err)
            assert.are.equal("H", out)
        end)

    end)

end)
