local M = {}

function M.sigmoid(x)
  if x < -709 then return 0.0 end
  if x > 709 then return 1.0 end
  return 1.0 / (1.0 + math.exp(-x))
end

function M.sigmoid_derivative(x)
  local sig = M.sigmoid(x)
  return sig * (1.0 - sig)
end

function M.relu(x)
  return math.max(0.0, x)
end

function M.relu_derivative(x)
  if x > 0.0 then return 1.0 else return 0.0 end
end

function M.tanh(x)
  if math.tanh then
    return math.tanh(x)
  end
  local ex = math.exp(x)
  local emx = math.exp(-x)
  return (ex - emx) / (ex + emx)
end

function M.tanh_derivative(x)
  local t = M.tanh(x)
  return 1.0 - (t * t)
end

return M
