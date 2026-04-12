-- test_document_manager.lua — DocumentManager tests
-- ==================================================
--
-- The DocumentManager tracks all files currently open in the editor.
-- It handles open/change/close operations and maintains the current text
-- of each file. These tests verify:
--
--   1. Opening a document stores its text and version
--   2. Getting a non-existent document returns nil
--   3. Closing a document removes it
--   4. Full replacement changes work
--   5. Incremental (range-based) changes work
--   6. Applying changes to a non-open document returns an error
--   7. Incremental changes with Unicode (emoji) work correctly

local ls00 = require("coding_adventures.ls00")

describe("DocumentManager", function()
    it("stores opened documents", function()
        local dm = ls00.DocumentManager:new()
        dm:open("file:///test.txt", "hello world", 1)

        local doc = dm:get("file:///test.txt")
        assert.is_not_nil(doc)
        assert.are.equal("hello world", doc.text)
        assert.are.equal(1, doc.version)
    end)

    it("returns nil for non-existent documents", function()
        local dm = ls00.DocumentManager:new()
        local doc = dm:get("file:///nonexistent.txt")
        assert.is_nil(doc)
    end)

    it("removes documents on close", function()
        local dm = ls00.DocumentManager:new()
        dm:open("file:///test.txt", "hello", 1)
        dm:close("file:///test.txt")

        local doc = dm:get("file:///test.txt")
        assert.is_nil(doc)
    end)

    it("applies full replacement changes", function()
        local dm = ls00.DocumentManager:new()
        dm:open("file:///test.txt", "hello world", 1)

        local err = dm:apply_changes("file:///test.txt", {
            { range = nil, text = "goodbye world" },
        }, 2)
        assert.is_nil(err)

        local doc = dm:get("file:///test.txt")
        assert.are.equal("goodbye world", doc.text)
        assert.are.equal(2, doc.version)
    end)

    it("applies incremental changes", function()
        local dm = ls00.DocumentManager:new()
        dm:open("file:///test.txt", "hello world", 1)

        -- Replace "world" (chars 6-11) with "Go".
        local err = dm:apply_changes("file:///test.txt", {
            {
                range = ls00.Range(
                    ls00.Position(0, 6),
                    ls00.Position(0, 11)
                ),
                text = "Go",
            },
        }, 2)
        assert.is_nil(err)

        local doc = dm:get("file:///test.txt")
        assert.are.equal("hello Go", doc.text)
    end)

    it("returns error for changes to non-open document", function()
        local dm = ls00.DocumentManager:new()
        local err = dm:apply_changes("file:///notopen.txt", {
            { range = nil, text = "x" },
        }, 1)
        assert.is_not_nil(err)
        assert.is_truthy(err:find("not open"))
    end)

    it("handles incremental changes with emoji", function()
        -- "A🎸B" — emoji is 4 UTF-8 bytes, 2 UTF-16 code units.
        -- Replace "B" (UTF-16 char 3, after the surrogate pair) with "X".
        local dm = ls00.DocumentManager:new()
        dm:open("file:///test.txt", "A\xF0\x9F\x8E\xB8B", 1)

        local err = dm:apply_changes("file:///test.txt", {
            {
                range = ls00.Range(
                    ls00.Position(0, 3),   -- UTF-16 char 3 = after 🎸
                    ls00.Position(0, 4)    -- UTF-16 char 4 = after B
                ),
                text = "X",
            },
        }, 2)
        assert.is_nil(err)

        local doc = dm:get("file:///test.txt")
        assert.are.equal("A\xF0\x9F\x8E\xB8X", doc.text)
    end)

    it("handles multi-change sequences", function()
        local dm = ls00.DocumentManager:new()
        dm:open("uri", "hello world", 1)

        -- Change "hello" to "hi".
        local err = dm:apply_changes("uri", {
            {
                range = ls00.Range(
                    ls00.Position(0, 0),
                    ls00.Position(0, 5)
                ),
                text = "hi",
            },
        }, 2)
        assert.is_nil(err)

        local doc = dm:get("uri")
        assert.are.equal("hi world", doc.text)
    end)
end)
