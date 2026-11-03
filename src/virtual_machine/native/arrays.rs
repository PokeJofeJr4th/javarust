use std::sync::Arc;

use jvmrs_lib::{access, method, FieldType};

use crate::{
    class::code::{NativeReturn, NativeStringMethod},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    virtual_machine::{
        native::make_primitives,
        object::{Array1, Array2, ArrayFields, ArrayType, ObjectFinder},
        Thread,
    },
};

#[allow(clippy::only_used_in_recursion)]
pub fn deep_to_string(
    thread: &mut Thread,
    [index]: [u32; 1],
    verbose: bool,
) -> NativeReturn<Arc<str>> {
    let index = index as usize;
    let arr_type = ArrayType::SELF.inspect(&thread.heap, index, |a| a.clone())?;
    match arr_type {
        FieldType::Array(_) => {
            let indices_vec = Array1.inspect(&thread.heap, index, |arr| arr.contents.to_vec())?;
            Ok(Some(
                format!(
                    "[{}]",
                    indices_vec
                        .into_iter()
                        .map(|idx| { deep_to_string(thread, [idx], verbose) })
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into(),
            ))
        }
        FieldType::Int | FieldType::Byte | FieldType::Short => Array1
            .inspect(&thread.heap, index, |arr| {
                arr.contents.iter().map(|i| *i as i32).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}").into())
            .map(Option::Some),
        FieldType::Boolean => Array1
            .inspect(&thread.heap, index, |arr| {
                arr.contents.iter().map(|i| *i != 0).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}").into())
            .map(Option::Some),
        FieldType::Char => Array1
            .inspect(&thread.heap, index, |arr| {
                format!(
                    "{:?}",
                    arr.contents
                        .iter()
                        .map(|c| char::from_u32(*c).unwrap_or_default())
                        .collect::<Vec<_>>()
                )
                .into()
            })
            .map(Option::Some),
        FieldType::Float => Array1
            .inspect(&thread.heap, index, |arr| {
                format!(
                    "{:?}",
                    arr.contents
                        .iter()
                        .map(|f| f32::from_bits(*f))
                        .collect::<Vec<_>>()
                )
                .into()
            })
            .map(Option::Some),
        _ => todo!(),
    }
}

pub fn to_string(
    thread: &mut Thread,
    [arr_ref]: [u32; 1],
    _verbose: bool,
) -> NativeReturn<Arc<str>> {
    let arr_ref = arr_ref as usize;
    let field_type = ArrayType::SELF.inspect(&thread.heap, arr_ref, |a| a.clone())?;
    if field_type.get_size() == 2 {
        Array2
            .inspect(
                &thread.heap,
                arr_ref,
                match field_type {
                    FieldType::Double => |arr: ArrayFields<'_, u64>| {
                        format!("{:?}", unsafe {
                            &*std::ptr::addr_of!(arr.contents).cast::<&[f64]>()
                        })
                        .into()
                    },
                    FieldType::Long => |arr: ArrayFields<'_, u64>| {
                        format!("{:?}", unsafe {
                            &*std::ptr::addr_of!(arr.contents).cast::<&[i64]>()
                        })
                        .into()
                    },
                    _ => unreachable!(),
                },
            )
            .map(Option::Some)
    } else {
        Array1
            .inspect(
                &thread.heap,
                arr_ref,
                match field_type {
                    FieldType::Int => |arr: ArrayFields<'_, u32>| {
                        format!("{:?}", unsafe {
                            &*std::ptr::addr_of!(arr.contents).cast::<&[i32]>()
                        })
                        .into()
                    },
                    FieldType::Float => |arr: ArrayFields<'_, u32>| {
                        format!("{:?}", unsafe {
                            &*std::ptr::addr_of!(arr.contents).cast::<&[f32]>()
                        })
                        .into()
                    },
                    _ => |arr: ArrayFields<'_, u32>| {
                        format!(
                            "[{}]",
                            arr.contents
                                .iter()
                                .map(|item| if *item == NULL {
                                    String::from("null")
                                } else {
                                    format!("&{item:0>8X}")
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                        .into()
                    },
                },
            )
            .map(Option::Some)
    }
}

pub fn add_native_methods(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
    let array = RawClass::new(
        access!(public native),
        "java/lang/Array".into(),
        java_lang_object.clone(),
    );

    let arrays_to_string_int = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]int)) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(to_string)),
        ..Default::default()
    };
    let arrays_to_string_float = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]float)) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(to_string)),
        ..Default::default()
    };
    let arrays_to_string_long = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]long)) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(to_string)),
        ..Default::default()
    };
    let arrays_to_string_double = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]double)) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(to_string)),
        ..Default::default()
    };
    let arrays_to_string_obj_arr = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]Object(java_lang_object.clone()))) -> Object(java_lang_string.clone())),
        signature: None,
        code: RawCode::native(NativeStringMethod(to_string)),
        attributes: Vec::new(),
        ..Default::default()
    };
    let deep_to_string = RawMethod {
        access_flags: access!(public static native),
        name: "deepToString".into(),
        descriptor: method!((([]Object(java_lang_object.clone()))) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(deep_to_string)),
        ..Default::default()
    };
    let mut arrays = RawClass::new(
        access!(public native),
        "java/util/Arrays".into(),
        java_lang_object.clone(),
    );
    arrays.register_methods(
        [
            arrays_to_string_int,
            arrays_to_string_float,
            arrays_to_string_long,
            arrays_to_string_double,
            arrays_to_string_obj_arr,
            deep_to_string,
        ],
        method_area,
    );
    let array_methods = make_primitives(method_area, class_area, java_lang_object.clone());
    arrays.methods.extend(
        array_methods
            .iter()
            .map(|method| method.name(arrays.this.clone())),
    );
    arrays.register_methods(array_methods, method_area);

    class_area.extend([array, arrays]);
}
