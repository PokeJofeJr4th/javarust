use std::{cell::RefCell, rc::Rc};

use crate::virtual_machine::{HeapElement, StackFrame};

pub fn init(
    heap_borrow: &[Rc<RefCell<HeapElement>>],
    stackframe: &RefCell<StackFrame>,
) -> Result<(), String> {
    let str_ref = stackframe.borrow_mut().locals[0];
    let obj_ref = stackframe.borrow_mut().locals[1];

    let heap_element = heap_borrow.get(str_ref as usize).unwrap().borrow();
    let HeapElement::String(init_string) = &*heap_element else {
                    return Err(format!("Expected a java/lang/String instance for java/lang/StringBuilder.<init>; got {heap_element:?}"));
                };
    let init_string = init_string.clone();
    drop(heap_element);

    let mut heap_element = heap_borrow.get(obj_ref as usize).unwrap().borrow_mut();
    let HeapElement::Object(random_obj) = &mut *heap_element else {
                    return Err(format!("Expected a java/lang/StringBuilder instance for java/lang/StringBuilder.<init>; got {heap_element:?}"));
                };
    random_obj
        .class_mut_or_insert(&stackframe.borrow().class)
        .native_fields
        .push(Box::new(init_string));
    drop(heap_element);
    Ok(())
}

pub fn set_char_at(
    heap_borrow: &[Rc<RefCell<HeapElement>>],
    stackframe: &RefCell<StackFrame>,
    builder_ref: usize,
    index: usize,
    character: char,
) -> Result<(), String> {
    let mut heap_element = heap_borrow.get(builder_ref).unwrap().borrow_mut();
    let HeapElement::Object(random_obj) = &mut *heap_element else {
                    return Err(format!("Expected a java/lang/StringBuilder instance for java/lang/StringBuilder.<init>; got {heap_element:?}"));
                };
    let string_ref = random_obj
        .class_mut_or_insert(&stackframe.borrow().class)
        .native_fields[0]
        .downcast_mut::<String>()
        .unwrap();
    string_ref.replace_range(
        string_ref
            .char_indices()
            .nth(index)
            .map(|(pos, ch)| (pos..pos + ch.len_utf8()))
            .unwrap(),
        &String::from(character),
    );
    // println!("{string_ref}");
    drop(heap_element);
    Ok(())
}

pub fn to_string(
    heap_borrow: &[Rc<RefCell<HeapElement>>],
    stackframe: &RefCell<StackFrame>,
    builder_ref: usize,
) -> Result<String, String> {
    let mut heap_element = heap_borrow.get(builder_ref).unwrap().borrow_mut();
    let HeapElement::Object(builder_obj) = &mut *heap_element else {
                    return Err(format!("Expected a java/lang/StringBuilder instance for java/lang/StringBuilder.<init>; got {heap_element:?}"));
                };
    let string = builder_obj
        .class_mut_or_insert(&stackframe.borrow().class)
        .native_fields[0]
        .downcast_ref::<String>()
        .unwrap()
        .clone();
    Ok(string)
}
