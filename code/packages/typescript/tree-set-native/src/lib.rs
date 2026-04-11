use node_bridge::*;
use tree_set::TreeSet;

macro_rules! unwrap_ref {
    ($env:expr, $value:expr, $ty:ty) => {
        unwrap_data::<$ty>($env, $value)
            .as_ref()
            .expect(concat!(stringify!($ty), " should always be wrapped"))
    };
}

macro_rules! unwrap_mut {
    ($env:expr, $value:expr, $ty:ty) => {
        unwrap_data_mut::<$ty>($env, $value)
            .as_mut()
            .expect(concat!(stringify!($ty), " should always be wrapped"))
    };
}

extern "C" {
    fn napi_get_value_int64(env: napi_env, value: napi_value, result: *mut i64) -> napi_status;
    fn napi_get_value_bool(env: napi_env, value: napi_value, result: *mut bool) -> napi_status;
}

fn i64_from_js(env: napi_env, value: napi_value) -> i64 {
    let mut result: i64 = 0;
    unsafe {
        let _ = napi_get_value_int64(env, value, &mut result);
    }
    result
}

fn bool_from_js(env: napi_env, value: napi_value) -> bool {
    let mut result: bool = false;
    unsafe {
        let _ = napi_get_value_bool(env, value, &mut result);
    }
    result
}

fn i32_from_js(env: napi_env, value: napi_value) -> i32 {
    i64_from_js(env, value) as i32
}

fn vec_i32_to_js(env: napi_env, items: &[i32]) -> napi_value {
    let arr = array_new(env);
    for (index, item) in items.iter().enumerate() {
        array_set(env, arr, index as u32, f64_to_js(env, *item as f64));
    }
    arr
}

unsafe extern "C" fn tree_set_new(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    wrap_data(env, this, TreeSet::<i32>::empty());
    this
}

unsafe extern "C" fn tree_set_add(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let value = match args.first() {
        Some(arg) => i32_from_js(env, *arg),
        None => {
            throw_error(env, "add requires one numeric argument");
            return undefined(env);
        }
    };
    let inner = unwrap_mut!(env, this, TreeSet::<i32>);
    let current = std::mem::take(inner);
    *inner = current.insert(value);
    undefined(env)
}

unsafe extern "C" fn tree_set_delete(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let value = match args.first() {
        Some(arg) => i32_from_js(env, *arg),
        None => {
            throw_error(env, "delete requires one numeric argument");
            return undefined(env);
        }
    };
    let inner = unwrap_mut!(env, this, TreeSet::<i32>);
    let current = std::mem::take(inner);
    let existed = current.contains(&value);
    *inner = current.delete(&value);
    bool_to_js(env, existed)
}

unsafe extern "C" fn tree_set_contains(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let value = match args.first() {
        Some(arg) => i32_from_js(env, *arg),
        None => {
            throw_error(env, "contains requires one numeric argument");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).contains(&value))
}

unsafe extern "C" fn tree_set_len(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    usize_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).size())
}

unsafe extern "C" fn tree_set_size(env: napi_env, info: napi_callback_info) -> napi_value {
    tree_set_len(env, info)
}

unsafe extern "C" fn tree_set_is_empty(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    bool_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).is_empty())
}

unsafe extern "C" fn tree_set_min_value(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    match unwrap_ref!(env, this, TreeSet::<i32>).min_value() {
        Some(value) => f64_to_js(env, *value as f64),
        None => null(env),
    }
}

unsafe extern "C" fn tree_set_max_value(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    match unwrap_ref!(env, this, TreeSet::<i32>).max_value() {
        Some(value) => f64_to_js(env, *value as f64),
        None => null(env),
    }
}

unsafe extern "C" fn tree_set_predecessor(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let value = match args.first() {
        Some(arg) => i32_from_js(env, *arg),
        None => {
            throw_error(env, "predecessor requires one numeric argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, TreeSet::<i32>).predecessor(&value) {
        Some(found) => f64_to_js(env, *found as f64),
        None => null(env),
    }
}

unsafe extern "C" fn tree_set_successor(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let value = match args.first() {
        Some(arg) => i32_from_js(env, *arg),
        None => {
            throw_error(env, "successor requires one numeric argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, TreeSet::<i32>).successor(&value) {
        Some(found) => f64_to_js(env, *found as f64),
        None => null(env),
    }
}

unsafe extern "C" fn tree_set_rank(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let value = match args.first() {
        Some(arg) => i32_from_js(env, *arg),
        None => {
            throw_error(env, "rank requires one numeric argument");
            return undefined(env);
        }
    };
    usize_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).rank(&value))
}

unsafe extern "C" fn tree_set_by_rank(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let rank = match args.first() {
        Some(arg) => i64_from_js(env, *arg) as usize,
        None => {
            throw_error(env, "byRank requires one numeric argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, TreeSet::<i32>).to_sorted_array().get(rank) {
        Some(found) => f64_to_js(env, *found as f64),
        None => null(env),
    }
}

unsafe extern "C" fn tree_set_kth_smallest(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let k = match args.first() {
        Some(arg) => i64_from_js(env, *arg) as usize,
        None => {
            throw_error(env, "kthSmallest requires one numeric argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, TreeSet::<i32>).kth_smallest(k) {
        Some(found) => f64_to_js(env, *found as f64),
        None => null(env),
    }
}

unsafe extern "C" fn tree_set_to_sorted_array(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    vec_i32_to_js(env, &unwrap_ref!(env, this, TreeSet::<i32>).to_sorted_array())
}

unsafe extern "C" fn tree_set_range(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 3);
    if args.len() < 2 {
        throw_error(env, "range requires at least two numeric arguments");
        return undefined(env);
    }
    let min = i32_from_js(env, args[0]);
    let max = i32_from_js(env, args[1]);
    let inclusive = if args.len() >= 3 { bool_from_js(env, args[2]) } else { true };
    vec_i32_to_js(env, &unwrap_ref!(env, this, TreeSet::<i32>).range(&min, &max, inclusive))
}

unsafe extern "C" fn tree_set_union_values(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "unionValues requires a TreeSet argument");
            return undefined(env);
        }
    };
    vec_i32_to_js(
        env,
        &unwrap_ref!(env, this, TreeSet::<i32>)
            .union(other)
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_intersection_values(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "intersectionValues requires a TreeSet argument");
            return undefined(env);
        }
    };
    vec_i32_to_js(
        env,
        &unwrap_ref!(env, this, TreeSet::<i32>)
            .intersection(other)
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_difference_values(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "differenceValues requires a TreeSet argument");
            return undefined(env);
        }
    };
    vec_i32_to_js(
        env,
        &unwrap_ref!(env, this, TreeSet::<i32>)
            .difference(other)
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_symmetric_difference_values(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "symmetricDifferenceValues requires a TreeSet argument");
            return undefined(env);
        }
    };
    vec_i32_to_js(
        env,
        &unwrap_ref!(env, this, TreeSet::<i32>)
            .symmetric_difference(other)
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_is_subset(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "isSubset requires a TreeSet argument");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).is_subset(other))
}

unsafe extern "C" fn tree_set_is_superset(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "isSuperset requires a TreeSet argument");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).is_superset(other))
}

unsafe extern "C" fn tree_set_is_disjoint(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "isDisjoint requires a TreeSet argument");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).is_disjoint(other))
}

unsafe extern "C" fn tree_set_equals(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let other = match args.first() {
        Some(arg) => unwrap_ref!(env, *arg, TreeSet::<i32>),
        None => {
            throw_error(env, "equals requires a TreeSet argument");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, TreeSet::<i32>).equals(other))
}

unsafe extern "C" fn tree_set_to_string(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    str_to_js(
        env,
        &format!("TreeSet({:?})", unwrap_ref!(env, this, TreeSet::<i32>).to_sorted_array()),
    )
}

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    let properties = [
        method_property("add", Some(tree_set_add)),
        method_property("delete", Some(tree_set_delete)),
        method_property("contains", Some(tree_set_contains)),
        method_property("len", Some(tree_set_len)),
        method_property("size", Some(tree_set_size)),
        method_property("isEmpty", Some(tree_set_is_empty)),
        method_property("minValue", Some(tree_set_min_value)),
        method_property("maxValue", Some(tree_set_max_value)),
        method_property("predecessor", Some(tree_set_predecessor)),
        method_property("successor", Some(tree_set_successor)),
        method_property("rank", Some(tree_set_rank)),
        method_property("byRank", Some(tree_set_by_rank)),
        method_property("kthSmallest", Some(tree_set_kth_smallest)),
        method_property("toSortedArray", Some(tree_set_to_sorted_array)),
        method_property("range", Some(tree_set_range)),
        method_property("unionValues", Some(tree_set_union_values)),
        method_property("intersectionValues", Some(tree_set_intersection_values)),
        method_property("differenceValues", Some(tree_set_difference_values)),
        method_property(
            "symmetricDifferenceValues",
            Some(tree_set_symmetric_difference_values),
        ),
        method_property("isSubset", Some(tree_set_is_subset)),
        method_property("isSuperset", Some(tree_set_is_superset)),
        method_property("isDisjoint", Some(tree_set_is_disjoint)),
        method_property("equals", Some(tree_set_equals)),
        method_property("toString", Some(tree_set_to_string)),
    ];

    let class = define_class(env, "NativeTreeSet", Some(tree_set_new), &properties);
    set_named_property(env, exports, "NativeTreeSet", class);
    exports
}
