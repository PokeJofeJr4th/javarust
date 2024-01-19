use std::sync::{Arc, Mutex};

use crate::virtual_machine::{
    object::{AnyObj, Object, ObjectFinder, StringBuilder, StringObj},
    StackFrame,
};

pub fn init(
    heap_borrow: &[Arc<Mutex<Object>>],
    stackframe: &Mutex<StackFrame>,
) -> Result<(), String> {
    let str_ref = stackframe.lock().unwrap().locals[0];
    let obj_ref = stackframe.lock().unwrap().locals[1];

    StringObj.get(heap_borrow, str_ref as usize, |init_string| {
        AnyObj
            .get_mut(heap_borrow, obj_ref as usize, |heap_obj| {
                heap_obj
                    .class_mut_or_insert(&stackframe.lock().unwrap().class)
                    .native_fields
                    .push(Box::new(String::from(&**init_string)));
            })
            .unwrap();
    })
}

pub fn set_char_at(
    heap_borrow: &[Arc<Mutex<Object>>],
    builder_ref: usize,
    index: usize,
    character: char,
) -> Result<(), String> {
    StringBuilder.get_mut(heap_borrow, builder_ref, |string_ref| {
        string_ref.replace_range(
            string_ref
                .char_indices()
                .nth(index)
                .map(|(pos, ch)| (pos..pos + ch.len_utf8()))
                .unwrap(),
            &String::from(character),
        );
        // println!("{string_ref}");
    })
}

pub fn to_string(
    heap_borrow: &[Arc<Mutex<Object>>],
    builder_ref: usize,
) -> Result<Arc<str>, String> {
    let string = Arc::from(&*StringBuilder.get(heap_borrow, builder_ref, Clone::clone)?);
    Ok(string)
}
