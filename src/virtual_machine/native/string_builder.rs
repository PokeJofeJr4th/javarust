use std::sync::Arc;

use crate::{
    class::code::NativeReturn,
    virtual_machine::{
        object::{AnyObj, ObjectFinder, StringBuilder, StringObj},
        Thread,
    },
};

#[allow(clippy::significant_drop_tightening)]
pub fn init(thread: &mut Thread, [obj_ref, str_ref]: [u32; 2], _verbose: bool) -> NativeReturn<()> {
    let init_string = StringObj::SELF.inspect(&thread.heap, str_ref as usize, |init_string| {
        init_string.to_string()
    })?;
    AnyObj
        .inspect(&thread.heap, obj_ref as usize, |heap_obj| {
            heap_obj.native_fields.push(Box::new(init_string));
        })
        .map(Option::Some)
}

pub fn set_char_at(
    thread: &mut Thread,
    [builder_ref, index, character]: [u32; 3],
    verbose: bool,
) -> NativeReturn<()> {
    let builder_ref = builder_ref as usize;
    let index = index as usize;
    let character = char::from_u32(character).unwrap();
    if verbose {
        println!("setting char at {index} to {character:?}");
    }
    StringBuilder::SELF
        .inspect(&thread.heap, builder_ref, |string_ref| {
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
        .map(Option::Some)
}

pub fn to_string(
    thread: &mut Thread,
    [builder_ref]: [u32; 1],
    _verbose: bool,
) -> NativeReturn<Arc<str>> {
    let string =
        Arc::from(&*StringBuilder::SELF.inspect(&thread.heap, builder_ref as usize, |a| a.clone())?);
    Ok(Some(string))
}
