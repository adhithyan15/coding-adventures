local M = {VERSION = "0.1.0"}

-- Lua tables cannot store an interior nil value.  NULL is the explicit empty
-- slot used by from_level_order and to_array.
local NULL = setmetatable({}, {
    __tostring = function()
        return "NULL"
    end,
})

local BinaryTreeNode = {}
BinaryTreeNode.__index = BinaryTreeNode
local is_node

function BinaryTreeNode.new(value, left, right)
    if value == nil or value == NULL then
        error("node value must not be nil or NULL", 2)
    end
    if left ~= nil and not is_node(left) then
        error("left must be a BinaryTreeNode or nil", 2)
    end
    if right ~= nil and not is_node(right) then
        error("right must be a BinaryTreeNode or nil", 2)
    end
    return setmetatable({value = value, left = left, right = right}, BinaryTreeNode)
end

is_node = function(value)
    return type(value) == "table" and getmetatable(value) == BinaryTreeNode
end

local function validate_child(value, name)
    if value ~= nil and not is_node(value) then
        error(name .. " must be a BinaryTreeNode or nil", 3)
    end
end

local function find(root, value)
    if root == nil then
        return nil
    end
    if root.value == value then
        return root
    end
    return find(root.left, value) or find(root.right, value)
end

local function height(root)
    if root == nil then
        return -1
    end
    return 1 + math.max(height(root.left), height(root.right))
end

local function size(root)
    if root == nil then
        return 0
    end
    return 1 + size(root.left) + size(root.right)
end

local function is_full(root)
    if root == nil then
        return true
    end
    if root.left == nil and root.right == nil then
        return true
    end
    if root.left == nil or root.right == nil then
        return false
    end
    return is_full(root.left) and is_full(root.right)
end

local function is_complete(root)
    local queue = {root == nil and NULL or root}
    local index = 1
    local seen_empty = false

    while index <= #queue do
        local node = queue[index]
        index = index + 1
        if node == NULL then
            seen_empty = true
        else
            if seen_empty then
                return false
            end
            queue[#queue + 1] = node.left == nil and NULL or node.left
            queue[#queue + 1] = node.right == nil and NULL or node.right
        end
    end
    return true
end

local function is_perfect(root)
    local tree_height = height(root)
    if tree_height < 0 then
        return size(root) == 0
    end
    return size(root) == (2 ^ (tree_height + 1)) - 1
end

local function build_from_level_order(values, index, length)
    if index > length then
        return nil
    end
    local value = values[index]
    if value == nil or value == NULL then
        return nil
    end
    return BinaryTreeNode.new(
        value,
        build_from_level_order(values, 2 * index, length),
        build_from_level_order(values, 2 * index + 1, length)
    )
end

local function append_inorder(root, out)
    if root == nil then
        return
    end
    append_inorder(root.left, out)
    out[#out + 1] = root.value
    append_inorder(root.right, out)
end

local function append_preorder(root, out)
    if root == nil then
        return
    end
    out[#out + 1] = root.value
    append_preorder(root.left, out)
    append_preorder(root.right, out)
end

local function append_postorder(root, out)
    if root == nil then
        return
    end
    append_postorder(root.left, out)
    append_postorder(root.right, out)
    out[#out + 1] = root.value
end

local function fill_array(root, index, out)
    if root == nil or index > #out then
        return
    end
    out[index] = root.value
    fill_array(root.left, 2 * index, out)
    fill_array(root.right, 2 * index + 1, out)
end

local function render_ascii(node, prefix, is_tail, lines)
    lines[#lines + 1] = prefix .. (is_tail and "`-- " or "|-- ") .. tostring(node.value)

    local children = {}
    if node.left ~= nil then
        children[#children + 1] = node.left
    end
    if node.right ~= nil then
        children[#children + 1] = node.right
    end

    local next_prefix = prefix .. (is_tail and "    " or "|   ")
    for index, child in ipairs(children) do
        render_ascii(child, next_prefix, index == #children, lines)
    end
end

local BinaryTree = {}
BinaryTree.__index = BinaryTree

function BinaryTree.new(root)
    validate_child(root, "root")
    return setmetatable({root = root}, BinaryTree)
end

function BinaryTree.with_root(root)
    return BinaryTree.new(root)
end

function BinaryTree.singleton(value)
    return BinaryTree.new(BinaryTreeNode.new(value))
end

function BinaryTree.from_level_order(values)
    if type(values) ~= "table" then
        error("values must be a table", 2)
    end
    return BinaryTree.new(build_from_level_order(values, 1, #values))
end

function BinaryTree:find(value)
    return find(self.root, value)
end

function BinaryTree:left_child(value)
    local node = self:find(value)
    return node == nil and nil or node.left
end

function BinaryTree:right_child(value)
    local node = self:find(value)
    return node == nil and nil or node.right
end

function BinaryTree:is_full()
    return is_full(self.root)
end

function BinaryTree:is_complete()
    return is_complete(self.root)
end

function BinaryTree:is_perfect()
    return is_perfect(self.root)
end

function BinaryTree:height()
    return height(self.root)
end

function BinaryTree:size()
    return size(self.root)
end

function BinaryTree:inorder()
    local out = {}
    append_inorder(self.root, out)
    return out
end

function BinaryTree:preorder()
    local out = {}
    append_preorder(self.root, out)
    return out
end

function BinaryTree:postorder()
    local out = {}
    append_postorder(self.root, out)
    return out
end

function BinaryTree:level_order()
    if self.root == nil then
        return {}
    end

    local out = {}
    local queue = {self.root}
    local index = 1
    while index <= #queue do
        local node = queue[index]
        index = index + 1
        out[#out + 1] = node.value
        if node.left ~= nil then
            queue[#queue + 1] = node.left
        end
        if node.right ~= nil then
            queue[#queue + 1] = node.right
        end
    end
    return out
end

function BinaryTree:to_array()
    local tree_height = self:height()
    if tree_height < 0 then
        return {}
    end

    local length = (2 ^ (tree_height + 1)) - 1
    local out = {}
    for index = 1, length do
        out[index] = NULL
    end
    fill_array(self.root, 1, out)
    return out
end

function BinaryTree:to_ascii()
    if self.root == nil then
        return ""
    end
    local lines = {}
    render_ascii(self.root, "", true, lines)
    return table.concat(lines, "\n")
end

function BinaryTree:__tostring()
    local root_value = self.root == nil and "nil" or tostring(self.root.value)
    return string.format("BinaryTree(root=%s, size=%d)", root_value, self:size())
end

M.NULL = NULL
M.BinaryTreeNode = BinaryTreeNode
M.BinaryTree = BinaryTree
M.find = find
M.height = height
M.size = size
M.is_full = is_full
M.is_complete = is_complete
M.is_perfect = is_perfect

return M
