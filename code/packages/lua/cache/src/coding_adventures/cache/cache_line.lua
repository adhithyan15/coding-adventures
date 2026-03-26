local CacheLine = {}
CacheLine.__index = CacheLine

function CacheLine.new(line_size)
    line_size = line_size or 64
    local self = setmetatable({}, CacheLine)
    self.valid = false
    self.dirty = false
    self.tag = 0
    self.last_access = 0
    self.data = {}
    for i = 1, line_size do
        self.data[i] = 0
    end
    return self
end

function CacheLine:fill(tag, data, cycle)
    self.valid = true
    self.dirty = false
    self.tag = tag
    self.last_access = cycle
    self.data = {}
    for i = 1, #data do
        self.data[i] = data[i]
    end
end

function CacheLine:touch(cycle)
    self.last_access = cycle
end

function CacheLine:invalidate()
    self.valid = false
    self.dirty = false
end

function CacheLine:line_size()
    return #self.data
end

function CacheLine:clone()
    local copy = CacheLine.new(#self.data)
    copy.valid = self.valid
    copy.dirty = self.dirty
    copy.tag = self.tag
    copy.last_access = self.last_access
    for i = 1, #self.data do
        copy.data[i] = self.data[i]
    end
    return copy
end

return CacheLine
