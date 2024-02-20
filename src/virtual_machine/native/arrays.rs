use std::sync::{Arc, Mutex};

use crate::{
    class::FieldType,
    virtual_machine::object::{Array1, ArrayType, Object, ObjectFinder},
};

pub fn deep_to_string(heap: &[Arc<Mutex<Object>>], index: usize) -> Result<String, String> {
    let arr_type = ArrayType.get(heap, index, Clone::clone)?;
    match arr_type {
        FieldType::Array(_) => {
            let indices_vec = Array1.get(heap, index, |arr| arr.contents.to_vec())?;
            Ok(format!(
                "[{}]",
                indices_vec
                    .into_iter()
                    .map(|idx| deep_to_string(heap, idx as usize))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ))
        }
        FieldType::Int | FieldType::Byte | FieldType::Short => Array1
            .get(heap, index, |arr| {
                arr.contents.iter().map(|i| *i as i32).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}")),
        FieldType::Boolean => Array1
            .get(heap, index, |arr| {
                arr.contents.iter().map(|i| *i != 0).collect::<Vec<_>>()
            })
            .map(|vec| format!("{vec:?}")),
        FieldType::Char => Array1.get(heap, index, |arr| {
            format!(
                "{:?}",
                arr.contents
                    .iter()
                    .map(|c| char::from_u32(*c).unwrap_or_default())
                    .collect::<Vec<_>>()
            )
        }),
        FieldType::Float => Array1.get(heap, index, |arr| {
            format!(
                "{:?}",
                arr.contents
                    .iter()
                    .map(|f| f32::from_bits(*f))
                    .collect::<Vec<_>>()
            )
        }),
        _ => todo!(),
    }
}
