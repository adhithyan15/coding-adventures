# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_heap"

class TestHeap < Minitest::Test
  include CodingAdventures::Heap

  def test_min_heap_orders_values
    heap = MinHeap.new
    [5, 1, 4, 2, 3].each { |value| heap << value }

    assert_equal 1, heap.peek
    assert_equal [1, 2, 3, 4, 5], 5.times.map { heap.pop }
  end

  def test_max_heap_orders_values
    heap = MaxHeap.new
    [5, 1, 4, 2, 3].each { |value| heap << value }

    assert_equal 5, heap.peek
    assert_equal [5, 4, 3, 2, 1], 5.times.map { heap.pop }
  end

  def test_replace_and_empty_pop
    heap = MinHeap.new
    assert_nil heap.pop
    heap.replace(7)
    assert_equal 7, heap.peek
  end
end
