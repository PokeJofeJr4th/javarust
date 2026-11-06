use std::sync::Arc;

use jvmrs_lib::{access, FieldType};

use crate::native_class;
use crate::virtual_machine::native::field::{ArrayField, ObjectField, StringField};
use crate::{
    class::code::NativeReturn,
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
            Ok(format!(
                "[{}]",
                indices_vec
                    .into_iter()
                    .map(|idx| { deep_to_string(thread, [idx], verbose) })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .into())
        }
        FieldType::Int | FieldType::Byte | FieldType::Short => Array1
            .inspect(&thread.heap, index, |arr| {
                arr.contents.iter().map(|i| *i as i32).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}").into()),
        FieldType::Boolean => Array1
            .inspect(&thread.heap, index, |arr| {
                arr.contents.iter().map(|i| *i != 0).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}").into()),
        FieldType::Char => Array1.inspect(&thread.heap, index, |arr| {
            format!(
                "{:?}",
                arr.contents
                    .iter()
                    .map(|c| char::from_u32(*c).unwrap_or_default())
                    .collect::<Vec<_>>()
            )
            .into()
        }),
        FieldType::Float => Array1.inspect(&thread.heap, index, |arr| {
            format!(
                "{:?}",
                arr.contents
                    .iter()
                    .map(|f| f32::from_bits(*f))
                    .collect::<Vec<_>>()
            )
            .into()
        }),
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
        Array2.inspect(
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
    } else {
        Array1.inspect(
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
    }
}

pub fn add_native_methods(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
) {
    let int_arr = ArrayField(super::field::Int);
    let long_arr = ArrayField(super::field::Long);
    let float_arr = ArrayField(super::field::Float);
    let double_arr = ArrayField(super::field::Double);
    let obj_arr = ArrayField(ObjectField("java/lang/Object"));
    let [array, mut arrays] = native_class! {
        let method_area;

        class "java/lang/Array" {
        }

        class "java/util/Arrays" {
            public static fn "toString"(this: int_arr) StringField => {
                let thread, verbose;
                to_string(thread, [this], verbose)
            }

            public static fn "toString"(this: long_arr) StringField => {
                let thread, verbose;
                to_string(thread, [this], verbose)
            }

            public static fn "toString"(this: float_arr) StringField => {
                let thread, verbose;
                to_string(thread, [this], verbose)
            }

            public static fn "toString"(this: double_arr) StringField => {
                let thread, verbose;
                to_string(thread, [this], verbose)
            }

            public static fn "toString"(this: obj_arr) StringField => {
                let thread, verbose;
                to_string(thread, [this], verbose)
            }

            public static fn "deepToString"(this: obj_arr) StringField => {
                let thread, verbose;
                deep_to_string(thread, [this], verbose)
            }
        }
    };
    let array_methods = make_primitives(method_area, class_area, java_lang_object.clone());
    arrays.register_methods(array_methods, method_area);

    class_area.extend([array, arrays]);
}
