package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local board_vm = require("coding_adventures.board_vm_native")

describe("coding_adventures.board_vm_native", function()
    it("loads the Rust native extension", function()
        assert.is_true(board_vm.available())
    end)

    it("builds blink upload/run frames through Rust-owned protocol builders", function()
        local session = board_vm.session({ next_request_id = 7, program_id = 9 })
        local frames = session:blink_upload_run_frames({
            pin = 13,
            high_ms = 125,
            low_ms = 250,
            max_stack = 4,
            instruction_budget = 777,
            time_budget_ms = 50,
        })

        assert.are.equal(4, #frames)
        assert.are.equal(11, session.next_request_id)
        for _, frame in ipairs(frames) do
            assert.is_string(frame)
            assert.is_true(#frame > 0)
            assert.are.equal(0, frame:byte(#frame))
        end
    end)

    it("builds HELLO and CAPS_QUERY frames with session request ids", function()
        local session = board_vm.session({ next_request_id = 3 })
        local hello = session:hello_wire("lua-host", 0x1234)
        local caps = session:caps_query_wire()

        assert.is_string(hello)
        assert.is_string(caps)
        assert.are.equal(5, session.next_request_id)
        assert.are.equal(0, hello:byte(#hello))
        assert.are.equal(0, caps:byte(#caps))
    end)
end)
