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

local AVLNode = {}
AVLNode.__index = AVLNode

local function is_node(value)
    return type(value) == "table" and getmetatable(value) == AVLNode
end

local function node_height(root)
    return root == nil and -1 or root.height
end

local function node_size(root)
    return root == nil and 0 or root.size
end

local function is_non_negative_integer(value)
    return type(value) == "number" and value >= 0 and value % 1 == 0
end

function AVLNode.new(value, left, right, height, size)
    if value == nil then
        error("node value must not be nil", 2)
    end
    if left ~= nil and not is_node(left) then
        error("left must be an AVLNode or nil", 2)
    end
    if right ~= nil and not is_node(right) then
        error("right must be an AVLNode or nil", 2)
    end

    local actual_height = height
    if actual_height == nil then
        actual_height = 1 + math.max(node_height(left), node_height(right))
    elseif not is_non_negative_integer(actual_height) then
        error("height must be a non-negative integer", 2)
    end

    local actual_size = size
    if actual_size == nil then
        actual_size = 1 + node_size(left) + node_size(right)
    elseif not is_non_negative_integer(actual_size) then
        error("size must be a non-negative integer", 2)
    end

    return setmetatable({
        value = value,
        left = left,
        right = right,
        height = actual_height,
        size = actual_size,
    }, AVLNode)
end

local function node(value, left, right)
    return AVLNode.new(value, left, right)
end

local function balance_factor(root)
    return node_height(root.left) - node_height(root.right)
end

local function rotate_left(root)
    local right = root.right
    if right == nil then
        return root
    end
    local new_left = node(root.value, root.left, right.left)
    return node(right.value, new_left, right.right)
end

local function rotate_right(root)
    local left = root.left
    if left == nil then
        return root
    end
    local new_right = node(root.value, left.right, root.right)
    return node(left.value, left.left, new_right)
end

local function rebalance(root)
    local factor = balance_factor(root)
    if factor > 1 then
        local left = root.left
        if left ~= nil and balance_factor(left) < 0 then
            left = rotate_left(left)
        end
        return rotate_right(node(root.value, left, root.right))
    end
    if factor < -1 then
        local right = root.right
        if right ~= nil and balance_factor(right) > 0 then
            right = rotate_right(right)
        end
        return rotate_left(node(root.value, root.left, right))
    end
    return root
end

local function insert_node(root, value, compare)
    if root == nil then
        return AVLNode.new(value)
    end
    local order = compare(value, root.value)
    if order < 0 then
        return rebalance(node(root.value, insert_node(root.left, value, compare), root.right))
    end
    if order > 0 then
        return rebalance(node(root.value, root.left, insert_node(root.right, value, compare)))
    end
    return root
end

local function extract_min(root)
    if root.left == nil then
        return root.right, root.value
    end
    local new_left, minimum = extract_min(root.left)
    return rebalance(node(root.value, new_left, root.right)), minimum
end

local function delete_node(root, value, compare)
    if root == nil then
        return nil
    end
    local order = compare(value, root.value)
    if order < 0 then
        return rebalance(node(root.value, delete_node(root.left, value, compare), root.right))
    end
    if order > 0 then
        return rebalance(node(root.value, root.left, delete_node(root.right, value, compare)))
    end
    if root.left == nil then
        return root.right
    end
    if root.right == nil then
        return root.left
    end

    local new_right, successor = extract_min(root.right)
    return rebalance(node(successor, root.left, new_right))
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

local function validate_bst(root, minimum, maximum, has_minimum, has_maximum, compare)
    if root == nil then
        return true
    end
    if has_minimum and compare(root.value, minimum) <= 0 then
        return false
    end
    if has_maximum and compare(root.value, maximum) >= 0 then
        return false
    end
    return validate_bst(root.left, minimum, root.value, has_minimum, true, compare)
        and validate_bst(root.right, root.value, maximum, true, has_maximum, compare)
end

local function validate_avl(root, minimum, maximum, has_minimum, has_maximum, compare)
    if root == nil then
        return -1, 0
    end
    if has_minimum and compare(root.value, minimum) <= 0 then
        return nil
    end
    if has_maximum and compare(root.value, maximum) >= 0 then
        return nil
    end

    local left_height, left_size = validate_avl(
        root.left,
        minimum,
        root.value,
        has_minimum,
        true,
        compare
    )
    if left_height == nil then
        return nil
    end
    local right_height, right_size = validate_avl(
        root.right,
        root.value,
        maximum,
        true,
        has_maximum,
        compare
    )
    if right_height == nil then
        return nil
    end

    local actual_height = 1 + math.max(left_height, right_height)
    local actual_size = 1 + left_size + right_size
    if root.height ~= actual_height
        or root.size ~= actual_size
        or math.abs(left_height - right_height) > 1
    then
        return nil
    end
    return actual_height, actual_size
end

local AVLTree = {}
AVLTree.__index = AVLTree

function AVLTree.new(root, compare)
    if root ~= nil and not is_node(root) then
        error("root must be an AVLNode or nil", 2)
    end
    local comparator = compare or default_compare
    if type(comparator) ~= "function" then
        error("compare must be a function", 2)
    end
    return setmetatable({root = root, compare = comparator}, AVLTree)
end

function AVLTree.empty(compare)
    return AVLTree.new(nil, compare)
end

function AVLTree.from_values(values, compare)
    if type(values) ~= "table" then
        error("values must be a table", 2)
    end
    local tree = AVLTree.empty(compare)
    for index, value in ipairs(values) do
        if value == nil then
            error(string.format("value at index %d must not be nil", index), 2)
        end
        tree = tree:insert(value)
    end
    return tree
end

function AVLTree:insert(value)
    if value == nil then
        error("value must not be nil", 2)
    end
    return AVLTree.new(insert_node(self.root, value, self.compare), self.compare)
end

function AVLTree:delete(value)
    if value == nil then
        error("value must not be nil", 2)
    end
    return AVLTree.new(delete_node(self.root, value, self.compare), self.compare)
end

function AVLTree:search(value)
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

function AVLTree:contains(value)
    return self:search(value) ~= nil
end

function AVLTree:min_value()
    local current = self.root
    while current ~= nil and current.left ~= nil do
        current = current.left
    end
    if current == nil then
        return nil
    end
    return current.value
end

function AVLTree:max_value()
    local current = self.root
    while current ~= nil and current.right ~= nil do
        current = current.right
    end
    if current == nil then
        return nil
    end
    return current.value
end

function AVLTree:predecessor(value)
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

function AVLTree:successor(value)
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

function AVLTree:kth_smallest(k)
    return kth_smallest(self.root, k)
end

function AVLTree:rank(value)
    return rank(self.root, value, self.compare)
end

function AVLTree:to_sorted_array()
    local out = {}
    append_inorder(self.root, out)
    return out
end

function AVLTree:is_valid_bst()
    return validate_bst(self.root, nil, nil, false, false, self.compare)
end

function AVLTree:is_valid_avl()
    return validate_avl(self.root, nil, nil, false, false, self.compare) ~= nil
end

function AVLTree:balance_factor(root)
    if root == nil then
        return 0
    end
    if not is_node(root) then
        error("node must be an AVLNode or nil", 2)
    end
    return balance_factor(root)
end

function AVLTree:height()
    return node_height(self.root)
end

function AVLTree:size()
    return node_size(self.root)
end

function AVLTree:__tostring()
    local root_value = self.root == nil and "nil" or tostring(self.root.value)
    return string.format("AVLTree(root=%s, height=%d, size=%d)", root_value, self:height(), self:size())
end

M.default_compare = default_compare
M.AVLNode = AVLNode
M.AVLTree = AVLTree

return M
