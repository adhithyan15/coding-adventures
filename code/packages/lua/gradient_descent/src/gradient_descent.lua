local M = {}

function M.sgd(weights, gradients, learning_rate)
  if #weights ~= #gradients or #weights == 0 then
    error("Arrays must have the same non-zero length")
  end
  local res = {}
  for i = 1, #weights do
    res[i] = weights[i] - (learning_rate * gradients[i])
  end
  return res
end

return M
