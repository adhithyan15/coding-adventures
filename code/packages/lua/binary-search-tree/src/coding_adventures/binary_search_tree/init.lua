local M = {VERSION = "0.1.0"}

local function default_compare(left, right)
    if left < right then
        return -1
    end
    if left > right then
        return 1
    end
    return 0
end

local BSTNode = {}
BSTNode.__index = BSTNode

local function is_node(value)
    return type(value) == "table" and getmetatable(value) == BSTNode
end

local function node_size(root)
    return root == nil and 0 or root.size
end

function BSTNode.new(value, left, right, size)
    if value == nil then
        error("node value must not be nil", 2)
    end
    if left ~= nil and not is_node(left) then
        error("left must be a BSTNode or nil", 2)
    end
    if right ~= nil and not is_node(right) then
        error("right must be a BSTNode or nil", 2)
    end
    local actual_size = size
    if actual_size == nil then
        actual_size = 1 + node_size(left) + node_size(right)
    elseif type(actual_size) ~= "number" or actual_size < 0 or actual_size % 1 ~= 0 then
        error("size must be a non-negative integer", 2)
    end
    return setmetatable({value = value, left = left, right = right, size = actual_size}, BSTNode)
end

local function with_children(root, left, right)
    return BSTNode.new(root.value, left, right)
end

local function insert_node(root, value, compare)
    if root == nil then
        return BSTNode.new(value)
    end
    local order = compare(value, root.value)
    if order < 0 then
        return with_children(root, insert_node(root.left, value, compare), root.right)
    end
    if order > 0 then
        return with_children(root, root.left, insert_node(root.right, value, compare))
    end
    return root
end

local function extract_min(root)
    if root.left == nil then
        return root.right, root.value
    end
    local new_left, minimum = extract_min(root.left)
    return with_children(root, new_left, root.right), minimum
end

local function delete_node(root, value, compare)
    if root == nil then
        return nil
    end
    local order = compare(value, root.value)
    if order < 0 then
        return with_children(root, delete_node(root.left, value, compare), root.right)
    end
    if order > 0 then
        return with_children(root, root.left, delete_node(root.right, value, compare))
    end
    if root.left == nil then
        return root.right
    end
    if root.right == nil then
        return root.left
    end

    local new_right, successor = extract_min(root.right)
    return BSTNode.new(successor, root.left, new_right)
end

local function kth_smallest(root, k)
    if root == nil or type(k) ~= "number" or k <= 0 or k % 1 ~= 0 then
        return nil
    end
    local left_size = node_size(root.left)
    if k == left_size + 1 then
        return root.value
    end
    if k <= left_size then
        return kth_smallest(root.left, k)
    end
    return kth_smallest(root.right, k - left_size - 1)
end

local function rank(root, value, compare)
    if root == nil then
        return 0
    end
    local order = compare(value, root.value)
    if order < 0 then
        return rank(root.left, value, compare)
    end
    if order > 0 then
        return node_size(root.left) + 1 + rank(root.right, value, compare)
    end
    return node_size(root.left)
end

local function append_inorder(root, out)
    if root == nil then
        return
    end
    append_inorder(root.left, out)
    out[#out + 1] = root.value
    append_inorder(root.right, out)
end

local function height(root)
    if root == nil then
        return -1
    end
    return 1 + math.max(height(root.left), height(root.right))
end

local function validate(root, minimum, maximum, has_minimum, has_maximum, compare)
    if root == nil then
        return -1, 0
    end
    if has_minimum and compare(root.value, minimum) <= 0 then
        return nil
    end
    if has_maximum and compare(root.value, maximum) >= 0 then
        return nil
    end

    local left_height, left_size = validate(root.left, minimum, root.value, has_minimum, true, compare)
    if left_height == nil then
        return nil
    end
    local right_height, right_size = validate(root.right, root.value, maximum, true, has_maximum, compare)
    if right_height == nil then
        return nil
    end

    local actual_size = 1 + left_size + right_size
    if root.size ~= actual_size then
        return nil
    end
    return 1 + math.max(left_height, right_height), actual_size
end

local function build_balanced(values, first, last)
    if first > last then
        return nil
    end
    local middle = math.floor((first + last + 1) / 2)
    local value = values[middle]
    if value == nil then
        error(string.format("value at index %d must not be nil", middle), 3)
    end
    return BSTNode.new(
        value,
        build_balanced(values, first, middle - 1),
        build_balanced(values, middle + 1, last)
    )
end

local BinarySearchTree = {}
BinarySearchTree.__index = BinarySearchTree

function BinarySearchTree.new(root, compare)
    if root ~= nil and not is_node(root) then
        error("root must be a BSTNode or nil", 2)
    end
    local comparator = compare or default_compare
    if type(comparator) ~= "function" then
        error("compare must be a function", 2)
    end
    return setmetatable({root = root, compare = comparator}, BinarySearchTree)
end

function BinarySearchTree.empty(compare)
    return BinarySearchTree.new(nil, compare)
end

function BinarySearchTree.from_sorted_array(values, compare)
    if type(values) ~= "table" then
        error("values must be a table", 2)
    end
    return BinarySearchTree.new(build_balanced(values, 1, #values), compare)
end

function BinarySearchTree:insert(value)
    if value == nil then
        error("value must not be nil", 2)
    end
    return BinarySearchTree.new(insert_node(self.root, value, self.compare), self.compare)
end

function BinarySearchTree:delete(value)
    if value == nil then
        error("value must not be nil", 2)
    end
    return BinarySearchTree.new(delete_node(self.root, value, self.compare), self.compare)
end

function BinarySearchTree:search(value)
    local current = self.root
    while current ~= nil do
        local order = self.compare(value, current.value)
        if order < 0 then
            current = current.left
        elseif order > 0 then
            current = current.right
        else
            return current
        end
    end
    return nil
end

function BinarySearchTree:contains(value)
    return self:search(value) ~= nil
end

function BinarySearchTree:min_value()
    local current = self.root
    while current ~= nil and current.left ~= nil do
        current = current.left
    end
    if current == nil then
        return nil
    end
    return current.value
end

function BinarySearchTree:max_value()
    local current = self.root
    while current ~= nil and current.right ~= nil do
        current = current.right
    end
    if current == nil then
        return nil
    end
    return current.value
end

function BinarySearchTree:predecessor(value)
    local current = self.root
    local best = nil
    while current ~= nil do
        if self.compare(value, current.value) <= 0 then
            current = current.left
        else
            best = current.value
            current = current.right
        end
    end
    return best
end

function BinarySearchTree:successor(value)
    local current = self.root
    local best = nil
    while current ~= nil do
        if self.compare(value, current.value) >= 0 then
            current = current.right
        else
            best = current.value
            current = current.left
        end
    end
    return best
end

function BinarySearchTree:kth_smallest(k)
    return kth_smallest(self.root, k)
end

function BinarySearchTree:rank(value)
    return rank(self.root, value, self.compare)
end

function BinarySearchTree:to_sorted_array()
    local out = {}
    append_inorder(self.root, out)
    return out
end

function BinarySearchTree:is_valid()
    return validate(self.root, nil, nil, false, false, self.compare) ~= nil
end

function BinarySearchTree:height()
    return height(self.root)
end

function BinarySearchTree:size()
    return node_size(self.root)
end

function BinarySearchTree:__tostring()
    local root_value = self.root == nil and "nil" or tostring(self.root.value)
    return string.format("BinarySearchTree(root=%s, size=%d)", root_value, self:size())
end

M.default_compare = default_compare
M.BSTNode = BSTNode
M.BinarySearchTree = BinarySearchTree

return M
