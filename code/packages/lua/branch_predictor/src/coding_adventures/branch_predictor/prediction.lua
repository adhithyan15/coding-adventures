-- branch_predictor/prediction.lua — Prediction result type
--
-- A Prediction is the output of a branch predictor for a single branch.
-- It contains:
--
--   predicted_taken  (boolean) — whether the branch is predicted taken
--   confidence       (number)  — 0.0 (no idea) to 1.0 (certain)
--   address          (number|nil) — predicted target address (from BTB)
--
-- The address field is only populated when a BTB is used. Without a BTB,
-- the fetch unit must wait until decode to know the target.

local Prediction = {}
Prediction.__index = Prediction

-- Create a new Prediction.
--
-- Parameters:
--   predicted_taken  (boolean) — predicted direction
--   confidence       (number)  — 0.0 to 1.0 (default 0.5)
--   address          (number|nil) — predicted target address (optional)
function Prediction.new(predicted_taken, confidence, address)
    return setmetatable({
        predicted_taken = predicted_taken,
        confidence      = confidence or 0.5,
        address         = address,   -- nil if unknown
    }, Prediction)
end

return Prediction
