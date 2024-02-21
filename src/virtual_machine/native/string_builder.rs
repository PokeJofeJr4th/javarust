use std::sync::{Arc, Mutex};

use crate::virtual_machine::{
    object::{AnyObj, Object, ObjectFinder, StringBuilder, StringObj},
    StackFrame, Thread,
};

pub fn init(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    _verbose: bool,
) -> Result<(), String> {
    let heap_borrow = thread.heap.lock().unwrap();
    let str_ref = stackframe.lock().unwrap().locals[0];
    let obj_ref = stackframe.lock().unwrap().locals[1];

    StringObj.get(&heap_borrow, str_ref as usize, |init_string| {
        AnyObj
            .get_mut(&heap_borrow, obj_ref as usize, |heap_obj| {
                heap_obj
                    .class_mut(&stackframe.lock().unwrap().class.this)
                    .unwrap()
                    .native_fields
                    .push(Box::new(String::from(&**init_string)));
            })
            .unwrap();
    })
}

pub fn set_char_at(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    verbose: bool,
) -> Result<(), String> {
    let heap_borrow = thread.heap.lock().unwrap();
    let builder_ref = stackframe.lock().unwrap().locals[0] as usize;
    let index = stackframe.lock().unwrap().locals[1] as usize;
    let character = char::from_u32(stackframe.lock().unwrap().locals[2]).unwrap();
    if verbose {
        println!("setting char at {index} to {character:?}");
    }
    StringBuilder.get_mut(&heap_borrow, builder_ref, |string_ref| {
        if verbose {
            println!("StringBuilder = {string_ref:?}");
        }
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
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    _verbose: bool,
) -> Result<Arc<str>, String> {
    let builder_ref = stackframe.lock().unwrap().locals[0] as usize;
    let string =
        Arc::from(&*StringBuilder.get(&thread.heap.lock().unwrap(), builder_ref, Clone::clone)?);
    Ok(string)
}
