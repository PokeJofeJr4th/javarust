use std::sync::Arc;

use jvmrs_lib::FieldType;

use crate::{
    class::code::NativeReturn,
    data::NULL,
    virtual_machine::{
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
                            &*std::ptr::addr_of!(arr.contents).cast::<Vec<f64>>()
                        })
                        .into()
                    },
                    FieldType::Long => |arr: ArrayFields<'_, u64>| {
                        format!("{:?}", unsafe {
                            &*std::ptr::addr_of!(arr.contents).cast::<Vec<i64>>()
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
                            &*std::ptr::addr_of!(arr.contents).cast::<Vec<f32>>()
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
