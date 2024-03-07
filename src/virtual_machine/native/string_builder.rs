use std::sync::{Arc, Mutex};

use crate::virtual_machine::{
    object::{AnyObj, ObjectFinder, StringBuilder, StringObj},
    StackFrame, Thread,
};

pub fn init(
    thread: &mut Thread,
    _stackframe: &Mutex<StackFrame>,
    [obj_ref, str_ref]: [u32; 2],
    _verbose: bool,
) -> Result<(), String> {
    let mut heap_borrow = thread.heap.lock().unwrap();

    let init_string = StringObj.get(&heap_borrow, str_ref as usize, |init_string| {
        init_string.to_string()
    })?;
    AnyObj.get_mut(&mut heap_borrow, obj_ref as usize, |heap_obj| {
        heap_obj.native_fields.push(Box::new(init_string));
    })
}

pub fn set_char_at(
    thread: &mut Thread,
    _stackframe: &Mutex<StackFrame>,
    [builder_ref, index, character]: [u32; 3],
    verbose: bool,
) -> Result<(), String> {
    let mut heap_borrow = thread.heap.lock().unwrap();
    let builder_ref = builder_ref as usize;
    let index = index as usize;
    let character = char::from_u32(character).unwrap();
    if verbose {
        println!("setting char at {index} to {character:?}");
    }
    StringBuilder.get_mut(&mut heap_borrow, builder_ref, |string_ref| {
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
    _stackframe: &Mutex<StackFrame>,
    [builder_ref]: [u32; 1],
    _verbose: bool,
) -> Result<Arc<str>, String> {
    let string = Arc::from(&*StringBuilder.get(
        &thread.heap.lock().unwrap(),
        builder_ref as usize,
        Clone::clone,
    )?);
    Ok(string)
}
