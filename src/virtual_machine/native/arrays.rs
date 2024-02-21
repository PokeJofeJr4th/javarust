use std::sync::{Arc, Mutex};

use crate::{
    class::FieldType,
    virtual_machine::{
        object::{Array1, Array2, ArrayFields, ArrayType, ObjectFinder},
        StackFrame, Thread,
    },
};

pub fn deep_to_string(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    verbose: bool,
) -> Result<Arc<str>, String> {
    let heap = thread.heap.lock().unwrap();
    let index = stackframe.lock().unwrap().operand_stack.pop().unwrap() as usize;
    let arr_type = ArrayType.get(&heap, index, Clone::clone)?;
    match arr_type {
        FieldType::Array(_) => {
            let indices_vec = Array1.get(&heap, index, |arr| arr.contents.to_vec())?;
            drop(heap);
            Ok(format!(
                "[{}]",
                indices_vec
                    .into_iter()
                    .map(|idx| {
                        stackframe.lock().unwrap().operand_stack.push(idx);
                        deep_to_string(thread, stackframe, verbose)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
            .into())
        }
        FieldType::Int | FieldType::Byte | FieldType::Short => Array1
            .get(&heap, index, |arr| {
                arr.contents.iter().map(|i| *i as i32).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}").into()),
        FieldType::Boolean => Array1
            .get(&heap, index, |arr| {
                arr.contents.iter().map(|i| *i != 0).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}").into()),
        FieldType::Char => Array1.get(&heap, index, |arr| {
            format!(
                "{:?}",
                arr.contents
                    .iter()
                    .map(|c| char::from_u32(*c).unwrap_or_default())
                    .collect::<Vec<_>>()
            )
            .into()
        }),
        FieldType::Float => Array1.get(&heap, index, |arr| {
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
    stackframe: &Mutex<StackFrame>,
    verbose: bool,
) -> Result<Arc<str>, String> {
    let arr_ref = stackframe.lock().unwrap().locals[0] as usize;
    let field_type = ArrayType.get(&thread.heap.lock().unwrap(), arr_ref, Clone::clone)?;
    if field_type.get_size() == 2 {
        Array2.get(
            &thread.heap.lock().unwrap(),
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
    } else {
        Array1.get(
            &thread.heap.lock().unwrap(),
            arr_ref,
            match field_type {
                FieldType::Int => |arr: ArrayFields<'_, u32>| {
                    format!("{:?}", unsafe {
                        &*std::ptr::addr_of!(arr.contents).cast::<Vec<i32>>()
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
                        "{:?}",
                        arr.contents
                            .iter()
                            .map(|item| format!("&{item:<8X}"))
                            .collect::<Vec<_>>()
                    )
                    .into()
                },
            },
        )
    }
}
