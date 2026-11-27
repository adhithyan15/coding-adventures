local M = {}

-- Ordered entries are separate from inode contents. This keeps directory
-- order stable across hosts and lexical lookup independent of descriptors.
function M.demo_image()
  return {
    {parent = 1, entries = {{"hello", 2}, {"empty", 3}, {"etc", 4}}},
    {data = "Hello, Unix!\n"},
    {data = ""},
    {parent = 1, entries = {{"notes", 5}}},
    {data = "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n"},
  }
end

function M.resolve(image, path)
  if path == "" or path:find("\0", 1, true) then return nil, "EINVAL" end
  local inode = 1
  for part in path:gmatch("[^/]+") do
    local node = image[inode]
    if not node then return nil, "ENOENT" end
    if not node.entries then return nil, "ENOTDIR" end
    if part == ".." then
      inode = node.parent
    elseif part ~= "." then
      local child
      for _, entry in ipairs(node.entries) do
        if entry[1] == part then
          child = entry[2]
          break
        end
      end
      if not child then return nil, "ENOENT" end
      inode = child
    end
  end
  local node = image[inode]
  if not node then return nil, "ENOENT" end
  if path:sub(-1) == "/" and not node.entries then return nil, "ENOTDIR" end
  return inode
end

function M.read_inode(image, inode, offset, count)
  local node = image[inode]
  if not node then return nil, "ENOENT" end
  if not node.data then return nil, "EISDIR" end
  if offset < 0 or count < 0 then return nil, "EINVAL" end
  local finish = offset + math.min(count, math.max(0, #node.data - offset))
  if finish <= offset then return "" end
  return node.data:sub(offset + 1, finish)
end

local Descriptors = {}
Descriptors.__index = Descriptors

function M.descriptors(capacity)
  return setmetatable({capacity = capacity, slots = {}}, Descriptors)
end

function Descriptors:get(fd)
  local slot = self.slots[fd]
  if slot then return {inode = slot.inode, offset = slot.offset} end
end

function Descriptors:allocate(inode)
  for fd = 0, self.capacity - 1 do
    if not self.slots[fd] then
      self.slots[fd] = {inode = inode, offset = 0}
      return fd
    end
  end
end

function Descriptors:advance(fd, bytes)
  local slot = self.slots[fd]
  if not slot or bytes < 0 or bytes > math.maxinteger - slot.offset then return false end
  slot.offset = slot.offset + bytes
  return true
end

function Descriptors:close(fd)
  if not self.slots[fd] then return false end
  self.slots[fd] = nil
  return true
end

local Kernel = {}
Kernel.__index = Kernel

function M.kernel(image, descriptors)
  return setmetatable({image = image, descriptors = descriptors}, Kernel)
end

function Kernel:open(path)
  local inode, err = M.resolve(self.image, path)
  if not inode then return {call = "open", path = path, error = err} end
  if self.image[inode].entries then return {call = "open", path = path, error = "EISDIR"} end
  local fd = self.descriptors:allocate(inode)
  if not fd then return {call = "open", path = path, error = "EMFILE"} end
  return {call = "open", path = path, fd = fd, inode = inode, offset = 0}
end

function Kernel:read(fd, count)
  local slot = self.descriptors:get(fd)
  if not slot then return {call = "read", fd = fd, error = "EBADF"} end
  local data, err = M.read_inode(self.image, slot.inode, slot.offset, count)
  if not data then return {call = "read", fd = fd, error = err} end
  self.descriptors:advance(fd, #data)
  return {
    call = "read", fd = fd, inode = slot.inode, offset = slot.offset,
    bytes = #data, nextOffset = slot.offset + #data,
  }, data
end

function Kernel:close(fd)
  local event = {call = "close", fd = fd}
  if not self.descriptors:close(fd) then event.error = "EBADF" end
  return event
end

function Kernel:list(path)
  local event = {call = "list", path = path}
  local inode, err = M.resolve(self.image, path)
  if not inode then event.error = err; return event end
  local entries = self.image[inode].entries
  if not entries then event.error = "ENOTDIR"; return event end
  event.inode, event.names = inode, {}
  for _, entry in ipairs(entries) do event.names[#event.names + 1] = entry[1] end
  return event
end

function M.run(command)
  local kernel = M.kernel(M.demo_image(), M.descriptors(4))
  local events, output, words = {}, {}, {}
  for word in command:gmatch("[^ \t\n\r\f\v]+") do words[#words + 1] = word end
  local function record(event)
    events[#events + 1] = event
    if event.error then output[#output + 1] = "error: " .. event.error .. "\n" end
  end
  if words[1] == "cat" and #words == 2 then
    local opened = kernel:open(words[2])
    record(opened)
    if opened.fd then
      while true do
        local read, data = kernel:read(opened.fd, 8)
        record(read)
        if not data or data == "" then break end
        output[#output + 1] = data
      end
      record(kernel:close(opened.fd))
    end
  elseif words[1] == "ls" and #words <= 2 then
    local listed = kernel:list(words[2] or "/")
    record(listed)
    for _, name in ipairs(listed.names or {}) do output[#output + 1] = name .. "\n" end
  elseif #words > 0 then
    output[1] = "usage: cat PATH or ls [PATH]\n"
  end
  return {output = table.concat(output), events = events}
end

return M
