use std::ffi::{c_int, c_long, c_void, CString};
use std::slice;

use ruby_bridge::VALUE;
type CoreTreeSet = tree_set::AvlTreeSet<i64>;

extern "C" {
    fn rb_num2long(val: VALUE) -> c_long;
    fn rb_int2inum(v: c_long) -> VALUE;
    fn rb_block_given_p() -> c_int;
    fn rb_yield(val: VALUE) -> VALUE;
}

static mut TREE_SET_CLASS: VALUE = 0;

fn method_id(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("no NUL");
    unsafe { ruby_bridge::rb_intern(c_name.as_ptr()) }
}

unsafe fn get_tree(self_val: VALUE) -> &'static CoreTreeSet {
    ruby_bridge::unwrap_data::<CoreTreeSet>(self_val)
}

unsafe fn get_tree_mut(self_val: VALUE) -> &'static mut CoreTreeSet {
    ruby_bridge::unwrap_data_mut::<CoreTreeSet>(self_val)
}

fn to_i64(value: VALUE) -> i64 {
    unsafe { rb_num2long(value) as i64 }
}

fn from_i64(value: i64) -> VALUE {
    unsafe { rb_int2inum(value as c_long) }
}

fn to_values(values: VALUE) -> Vec<i64> {
    if values == ruby_bridge::QNIL {
        return Vec::new();
    }
    let to_a_id = method_id("to_a");
    let array = unsafe { ruby_bridge::rb_funcallv(values, to_a_id, 0, std::ptr::null()) };
    let len = ruby_bridge::array_len(array);
    let mut result = Vec::with_capacity(len);
    for index in 0..len {
        result.push(to_i64(ruby_bridge::array_entry(array, index)));
    }
    result
}

fn wrap_set(values: Vec<i64>) -> VALUE {
    ruby_bridge::wrap_data(
        unsafe { TREE_SET_CLASS },
        CoreTreeSet::from_list(values),
    )
}

extern "C" fn tree_set_initialize(argc: c_int, argv: *const VALUE, self_val: VALUE) -> VALUE {
    let args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    if args.len() > 1 {
        ruby_bridge::raise_arg_error("TreeSet.new accepts at most one values argument");
    }
    let values = args.first().copied().unwrap_or(ruby_bridge::QNIL);
    let inner = unsafe { get_tree_mut(self_val) };
    *inner = CoreTreeSet::from_list(to_values(values));
    self_val
}

extern "C" fn tree_set_from_values(argc: c_int, argv: *const VALUE, self_val: VALUE) -> VALUE {
    unsafe { ruby_bridge::rb_funcallv(self_val, method_id("new"), argc, argv) }
}

extern "C" fn tree_set_add(self_val: VALUE, value: VALUE) -> VALUE {
    let inner = unsafe { get_tree_mut(self_val) };
    let current = std::mem::take(inner);
    *inner = current.insert(to_i64(value));
    self_val
}

extern "C" fn tree_set_delete(self_val: VALUE, value: VALUE) -> VALUE {
    let inner = unsafe { get_tree_mut(self_val) };
    let current = std::mem::take(inner);
    let existed = current.contains(&to_i64(value));
    *inner = current.delete(&to_i64(value));
    ruby_bridge::bool_to_rb(existed)
}

extern "C" fn tree_set_include(self_val: VALUE, value: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_tree(self_val).contains(&to_i64(value)) })
}

extern "C" fn tree_set_size(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_tree(self_val).size() })
}

extern "C" fn tree_set_empty(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_tree(self_val).is_empty() })
}

extern "C" fn tree_set_min(self_val: VALUE) -> VALUE {
    match unsafe { get_tree(self_val).min_value() } {
        Some(value) => from_i64(*value),
        None => ruby_bridge::QNIL,
    }
}

extern "C" fn tree_set_max(self_val: VALUE) -> VALUE {
    match unsafe { get_tree(self_val).max_value() } {
        Some(value) => from_i64(*value),
        None => ruby_bridge::QNIL,
    }
}

extern "C" fn tree_set_predecessor(self_val: VALUE, value: VALUE) -> VALUE {
    match unsafe { get_tree(self_val).predecessor(&to_i64(value)) } {
        Some(found) => from_i64(*found),
        None => ruby_bridge::QNIL,
    }
}

extern "C" fn tree_set_successor(self_val: VALUE, value: VALUE) -> VALUE {
    match unsafe { get_tree(self_val).successor(&to_i64(value)) } {
        Some(found) => from_i64(*found),
        None => ruby_bridge::QNIL,
    }
}

extern "C" fn tree_set_rank(self_val: VALUE, value: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_tree(self_val).rank(&to_i64(value)) })
}

extern "C" fn tree_set_by_rank(self_val: VALUE, rank: VALUE) -> VALUE {
    match unsafe { get_tree(self_val).to_sorted_array().get(to_i64(rank) as usize) } {
        Some(found) => from_i64(*found),
        None => ruby_bridge::QNIL,
    }
}

extern "C" fn tree_set_kth_smallest(self_val: VALUE, k: VALUE) -> VALUE {
    match unsafe { get_tree(self_val).kth_smallest(to_i64(k) as usize) } {
        Some(found) => from_i64(*found),
        None => ruby_bridge::QNIL,
    }
}

extern "C" fn tree_set_to_a(self_val: VALUE) -> VALUE {
    let values = unsafe { get_tree(self_val).to_sorted_array() };
    let array = ruby_bridge::array_new();
    for value in values {
        ruby_bridge::array_push(array, from_i64(value));
    }
    array
}

extern "C" fn tree_set_range(argc: c_int, argv: *const VALUE, self_val: VALUE) -> VALUE {
    let args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    if !(args.len() == 2 || args.len() == 3) {
        ruby_bridge::raise_arg_error("range expects minimum, maximum, and optional inclusive");
    }
    let min = to_i64(args[0]);
    let max = to_i64(args[1]);
    let inclusive = if args.len() == 3 {
        args[2] != ruby_bridge::QFALSE && args[2] != ruby_bridge::QNIL
    } else {
        true
    };
    let values = unsafe { get_tree(self_val).range(&min, &max, inclusive) };
    let array = ruby_bridge::array_new();
    for value in values {
        ruby_bridge::array_push(array, from_i64(value));
    }
    array
}

fn values_from_other(other: VALUE) -> Vec<i64> {
    to_values(other)
}

extern "C" fn tree_set_union(self_val: VALUE, other: VALUE) -> VALUE {
    wrap_set(
        unsafe { get_tree(self_val) }
            .union(&CoreTreeSet::from_list(values_from_other(other)))
            .to_sorted_array(),
    )
}

extern "C" fn tree_set_intersection(self_val: VALUE, other: VALUE) -> VALUE {
    wrap_set(
        unsafe { get_tree(self_val) }
            .intersection(&CoreTreeSet::from_list(values_from_other(other)))
            .to_sorted_array(),
    )
}

extern "C" fn tree_set_difference(self_val: VALUE, other: VALUE) -> VALUE {
    wrap_set(
        unsafe { get_tree(self_val) }
            .difference(&CoreTreeSet::from_list(values_from_other(other)))
            .to_sorted_array(),
    )
}

extern "C" fn tree_set_symmetric_difference(self_val: VALUE, other: VALUE) -> VALUE {
    wrap_set(
        unsafe { get_tree(self_val) }
            .symmetric_difference(&CoreTreeSet::from_list(values_from_other(other)))
            .to_sorted_array(),
    )
}

extern "C" fn tree_set_subset(self_val: VALUE, other: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_tree(self_val).is_subset(&CoreTreeSet::from_list(values_from_other(other))) })
}

extern "C" fn tree_set_superset(self_val: VALUE, other: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_tree(self_val).is_superset(&CoreTreeSet::from_list(values_from_other(other))) })
}

extern "C" fn tree_set_disjoint(self_val: VALUE, other: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_tree(self_val).is_disjoint(&CoreTreeSet::from_list(values_from_other(other))) })
}

extern "C" fn tree_set_equals(self_val: VALUE, other: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_tree(self_val).equals(&CoreTreeSet::from_list(values_from_other(other))) })
}

extern "C" fn tree_set_each(argc: c_int, argv: *const VALUE, self_val: VALUE) -> VALUE {
    let _args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    let each_id = unsafe { ruby_bridge::rb_intern(c"each".as_ptr()) };
    if unsafe { rb_block_given_p() } == 0 {
        let array = tree_set_to_a(self_val);
        return unsafe { ruby_bridge::rb_funcallv(array, each_id, 0, std::ptr::null()) };
    }

    for value in unsafe { get_tree(self_val).to_sorted_array() } {
        unsafe {
            rb_yield(from_i64(value));
        }
    }
    self_val
}

extern "C" fn tree_set_inspect(self_val: VALUE) -> VALUE {
    ruby_bridge::str_to_rb(&format!("TreeSet({:?})", unsafe { get_tree(self_val) }.to_sorted_array()))
}

unsafe extern "C" fn tree_set_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(klass, CoreTreeSet::empty())
}

#[no_mangle]
pub unsafe extern "C" fn Init_tree_set_native() {
    let module = ruby_bridge::define_module("CodingAdventures");
    let namespace = ruby_bridge::define_module_under(module, "TreeSetNative");
    let class = ruby_bridge::define_class_under(namespace, "TreeSet", ruby_bridge::object_class());
    TREE_SET_CLASS = class;

    ruby_bridge::define_alloc_func(class, tree_set_alloc);
    ruby_bridge::define_method_raw(class, "initialize", tree_set_initialize as *const c_void, -1);
    ruby_bridge::define_singleton_method_raw(class, "from_values", tree_set_from_values as *const c_void, -1);
    ruby_bridge::define_method_raw(class, "add", tree_set_add as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "delete", tree_set_delete as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "include?", tree_set_include as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "member?", tree_set_include as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "size", tree_set_size as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "length", tree_set_size as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "empty?", tree_set_empty as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "min", tree_set_min as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "max", tree_set_max as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "first", tree_set_min as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "last", tree_set_max as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "predecessor", tree_set_predecessor as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "successor", tree_set_successor as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "rank", tree_set_rank as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "by_rank", tree_set_by_rank as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "kth_smallest", tree_set_kth_smallest as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "to_a", tree_set_to_a as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "to_sorted_array", tree_set_to_a as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "range", tree_set_range as *const c_void, -1);
    ruby_bridge::define_method_raw(class, "union", tree_set_union as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "intersection", tree_set_intersection as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "difference", tree_set_difference as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "symmetric_difference", tree_set_symmetric_difference as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "subset?", tree_set_subset as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "superset?", tree_set_superset as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "disjoint?", tree_set_disjoint as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "equals", tree_set_equals as *const c_void, 1);
    ruby_bridge::define_method_raw(class, "each", tree_set_each as *const c_void, -1);
    ruby_bridge::define_method_raw(class, "inspect", tree_set_inspect as *const c_void, 0);
    ruby_bridge::define_method_raw(class, "to_s", tree_set_inspect as *const c_void, 0);
}
