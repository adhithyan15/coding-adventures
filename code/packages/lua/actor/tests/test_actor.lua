-- Tests for coding_adventures.actor
--
-- Covers: ActorResult, ActorSystem, spawn, send, run, get_state,
--         is_stopped, dead letters, cascading messages, actor creation
--         from behavior, and stop semantics.
--
-- Lua 5.4 busted test suite.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local Actor = require("coding_adventures.actor")

-- Convenience shorthand
local Result = Actor.ActorResult
local System = Actor.ActorSystem

describe("actor", function()

    -- -----------------------------------------------------------------------
    -- Version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(Actor.VERSION)
        assert.equals("0.1.0", Actor.VERSION)
    end)

    -- -----------------------------------------------------------------------
    -- ActorResult
    -- -----------------------------------------------------------------------

    it("ActorResult.new stores new_state", function()
        local r = Result.new({ new_state = 42 })
        assert.equals(42, r.new_state)
    end)

    it("ActorResult.new defaults messages_to_send to empty table", function()
        local r = Result.new({ new_state = 0 })
        assert.is_not_nil(r.messages_to_send)
        assert.equals(0, #r.messages_to_send)
    end)

    it("ActorResult.new defaults actors_to_create to empty table", function()
        local r = Result.new({ new_state = 0 })
        assert.is_not_nil(r.actors_to_create)
        assert.equals(0, #r.actors_to_create)
    end)

    it("ActorResult.new defaults stop to false", function()
        local r = Result.new({ new_state = 0 })
        assert.is_false(r.stop)
    end)

    it("ActorResult.new respects explicit stop=true", function()
        local r = Result.new({ new_state = 0, stop = true })
        assert.is_true(r.stop)
    end)

    it("ActorResult.new stores messages_to_send", function()
        local r = Result.new({
            new_state = 0,
            messages_to_send = {{"other", {type="hi"}}}
        })
        assert.equals(1, #r.messages_to_send)
        assert.equals("other", r.messages_to_send[1][1])
    end)

    -- -----------------------------------------------------------------------
    -- ActorSystem construction
    -- -----------------------------------------------------------------------

    it("ActorSystem.new creates an empty actor system", function()
        local sys = System.new()
        assert.is_not_nil(sys.actors)
        assert.is_not_nil(sys.queue)
        assert.is_not_nil(sys.dead_letters)
        assert.equals(0, #sys.dead_letters)
    end)

    -- -----------------------------------------------------------------------
    -- spawn
    -- -----------------------------------------------------------------------

    it("spawn registers an actor and returns its id", function()
        local sys = System.new()
        local id = sys:spawn("myactor", 0, function(s, _) return Result.new({new_state=s}) end)
        assert.equals("myactor", id)
    end)

    it("spawn sets the initial state", function()
        local sys = System.new()
        sys:spawn("a", 99, function(s, _) return Result.new({new_state=s}) end)
        assert.equals(99, sys:get_state("a"))
    end)

    it("spawn raises an error if actor id is already taken", function()
        local sys = System.new()
        sys:spawn("dup", 0, function(s, _) return Result.new({new_state=s}) end)
        assert.has_error(function()
            sys:spawn("dup", 0, function(s, _) return Result.new({new_state=s}) end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- send / run / get_state
    -- -----------------------------------------------------------------------

    it("send followed by run delivers the message to the actor", function()
        local sys = System.new()
        sys:spawn("counter", 0, function(state, msg)
            if msg.type == "inc" then
                return Result.new({ new_state = state + 1 })
            end
        end)
        sys:send("counter", {type="inc"})
        sys:run()
        assert.equals(1, sys:get_state("counter"))
    end)

    it("multiple sends accumulate state correctly", function()
        local sys = System.new()
        sys:spawn("counter", 0, function(state, msg)
            if msg.type == "inc" then
                return Result.new({ new_state = state + 1 })
            end
        end)
        sys:send("counter", {type="inc"})
        sys:send("counter", {type="inc"})
        sys:send("counter", {type="inc"})
        sys:run()
        assert.equals(3, sys:get_state("counter"))
    end)

    it("actor can store complex state (table)", function()
        local sys = System.new()
        sys:spawn("store", {items={}}, function(state, msg)
            if msg.type == "add" then
                local new_items = {}
                for _, v in ipairs(state.items) do new_items[#new_items+1] = v end
                new_items[#new_items+1] = msg.value
                return Result.new({ new_state = {items=new_items} })
            end
        end)
        sys:send("store", {type="add", value="apple"})
        sys:send("store", {type="add", value="banana"})
        sys:run()
        local s = sys:get_state("store")
        assert.equals(2, #s.items)
        assert.equals("apple",  s.items[1])
        assert.equals("banana", s.items[2])
    end)

    it("get_state raises error for unknown actor", function()
        local sys = System.new()
        assert.has_error(function()
            sys:get_state("ghost")
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Messages between actors
    -- -----------------------------------------------------------------------

    it("actor can send a message to another actor", function()
        local sys = System.new()
        -- Relay actor: forwards value to accumulator
        sys:spawn("relay", nil, function(_, msg)
            return Result.new({
                new_state = nil,
                messages_to_send = {{"accum", {type="add", val=msg.val}}}
            })
        end)
        sys:spawn("accum", 0, function(state, msg)
            if msg.type == "add" then
                return Result.new({ new_state = state + msg.val })
            end
        end)
        sys:send("relay", {val=7})
        sys:run()
        assert.equals(7, sys:get_state("accum"))
    end)

    it("cascaded messages are all processed in one run call", function()
        local sys = System.new()
        -- Chain: a → b → c
        sys:spawn("c", 0, function(state, msg)
            return Result.new({ new_state = state + msg.n })
        end)
        sys:spawn("b", nil, function(_, msg)
            return Result.new({
                new_state = nil,
                messages_to_send = {{"c", {n = msg.n * 2}}}
            })
        end)
        sys:spawn("a", nil, function(_, msg)
            return Result.new({
                new_state = nil,
                messages_to_send = {{"b", {n = msg.n + 1}}}
            })
        end)
        sys:send("a", {n = 3})   -- a → b with n=4, b → c with n=8
        sys:run()
        assert.equals(8, sys:get_state("c"))
    end)

    -- -----------------------------------------------------------------------
    -- Actor creation from behavior
    -- -----------------------------------------------------------------------

    it("behavior can spawn new actors via actors_to_create", function()
        local sys = System.new()
        -- "factory" spawns a worker when it receives a message
        sys:spawn("factory", nil, function(_, msg)
            return Result.new({
                new_state = nil,
                actors_to_create = {
                    {
                        actor_id      = msg.worker_id,
                        initial_state = 0,
                        behavior      = function(state, m)
                            if m.type == "inc" then
                                return Result.new({ new_state = state + 1 })
                            end
                        end
                    }
                }
            })
        end)
        sys:send("factory", {worker_id = "w1"})
        sys:run()
        -- w1 should now exist
        assert.equals(0, sys:get_state("w1"))
        sys:send("w1", {type="inc"})
        sys:run()
        assert.equals(1, sys:get_state("w1"))
    end)

    -- -----------------------------------------------------------------------
    -- Stop semantics
    -- -----------------------------------------------------------------------

    it("actor with stop=true is marked as stopped after run", function()
        local sys = System.new()
        sys:spawn("mortal", 0, function(state, msg)
            if msg.type == "die" then
                return Result.new({ new_state = state, stop = true })
            end
        end)
        sys:send("mortal", {type="die"})
        sys:run()
        assert.is_true(sys:is_stopped("mortal"))
    end)

    it("messages to a stopped actor go to dead_letters", function()
        local sys = System.new()
        sys:spawn("mortal", 0, function(state, msg)
            if msg.type == "die" then
                return Result.new({ new_state = state, stop = true })
            end
        end)
        sys:send("mortal", {type="die"})
        sys:run()
        -- Now send to the stopped actor
        sys:send("mortal", {type="ping"})
        sys:run()
        assert.equals(1, #sys.dead_letters)
        assert.equals("mortal", sys.dead_letters[1][1])
    end)

    -- -----------------------------------------------------------------------
    -- Dead letters
    -- -----------------------------------------------------------------------

    it("message to unknown actor goes to dead_letters", function()
        local sys = System.new()
        sys:send("nobody", {type="hello"})
        sys:run()
        assert.equals(1, #sys.dead_letters)
        assert.equals("nobody", sys.dead_letters[1][1])
    end)

    it("multiple dead letters accumulate", function()
        local sys = System.new()
        sys:send("ghost1", {})
        sys:send("ghost2", {})
        sys:run()
        assert.equals(2, #sys.dead_letters)
    end)

    -- -----------------------------------------------------------------------
    -- Queue drains completely
    -- -----------------------------------------------------------------------

    it("run with no queued messages is a no-op", function()
        local sys = System.new()
        sys:spawn("idle", 0, function(s, _) return Result.new({new_state=s}) end)
        sys:run()   -- should not error
        assert.equals(0, sys:get_state("idle"))
    end)

    it("second run call after queue is empty is a no-op", function()
        local sys = System.new()
        sys:spawn("c", 0, function(state, msg)
            return Result.new({ new_state = state + (msg.n or 0) })
        end)
        sys:send("c", {n=5})
        sys:run()
        sys:run()   -- second run on empty queue
        assert.equals(5, sys:get_state("c"))
    end)

    -- -----------------------------------------------------------------------
    -- is_stopped
    -- -----------------------------------------------------------------------

    it("is_stopped returns false for a running actor", function()
        local sys = System.new()
        sys:spawn("alive", 0, function(s, _) return Result.new({new_state=s}) end)
        assert.is_false(sys:is_stopped("alive"))
    end)

    it("is_stopped raises error for unknown actor", function()
        local sys = System.new()
        assert.has_error(function()
            sys:is_stopped("unknown")
        end)
    end)

end)
