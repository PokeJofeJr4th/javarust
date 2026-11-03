use std::sync::Arc;

use jvmrs_lib::{access, method, FieldType, MethodDescriptor};

use crate::{
    class::code::{NativeReturn, NativeSingleMethod},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    virtual_machine::{
        native::native_property,
        object::{AnyObj, ObjectFinder, StringObj},
        Thread,
    },
};

pub fn native_string_value_of(
    thread: &mut Thread,
    [obj_ref]: [u32; 1],
    verbose: bool,
) -> NativeReturn<u32> {
    if obj_ref == NULL {
        let str_ref = thread.heap.lock().unwrap().allocate_str("null".into());
        Ok(Some(str_ref))
    } else if thread.pc_register == 0 {
        let (to_string_class, to_string_method) =
            AnyObj.inspect(&thread.heap, obj_ref as usize, |obj| {
                obj.resolve_method(
                    &thread.method_area,
                    &thread.class_area,
                    "toString",
                    &MethodDescriptor {
                        parameter_size: 0,
                        parameters: Vec::new(),
                        return_type: Some(FieldType::Object("java/lang/String".into())),
                    },
                    verbose,
                )
            })?;
        // push a fake return address
        thread.stackframe.operand_stack.push(1);
        thread.invoke_method(to_string_method, to_string_class);
        thread.stackframe.locals[0] = obj_ref;
        Ok(None)
    } else {
        Ok(Some(thread.stackframe.operand_stack.pop().unwrap()))
    }
}

pub fn native_println_object(
    thread: &mut Thread,
    [_, arg]: [u32; 2],
    verbose: bool,
) -> NativeReturn<()> {
    // println!("{stackframe:?}");
    if thread.pc_register == 0 {
        let (to_string_class, to_string_method) =
            AnyObj.inspect(&thread.heap, arg as usize, |obj| {
                obj.resolve_method(
                    &thread.method_area,
                    &thread.class_area,
                    "toString",
                    &MethodDescriptor {
                        parameter_size: 0,
                        parameters: Vec::new(),
                        return_type: Some(FieldType::Object("java/lang/String".into())),
                    },
                    verbose,
                )
            })?;
        if verbose {
            println!(
                "Resolved java/lang/Object.toString to {}.{}",
                to_string_class.this, to_string_method.name
            );
        }
        // push a fake return address
        thread.stackframe.operand_stack.push(1);
        thread.invoke_method(to_string_method, to_string_class);
        thread.stackframe.locals[0] = arg;
        Ok(None)
    } else {
        let ret = thread.stackframe.operand_stack.pop().unwrap();
        let str = StringObj::SELF.inspect(&thread.heap, ret as usize, |a| a.clone())?;
        println!("{str}");
        Ok(Some(()))
    }
}

pub fn native_string_char_at(
    thread: &mut Thread,
    [string_ref, index]: [u32; 2],
    _verbose: bool,
) -> NativeReturn<u32> {
    let index = index as usize;
    StringObj::SELF
        .inspect(&thread.heap, string_ref as usize, |str| {
            str.chars().nth(index).unwrap() as u32
        })
        .map(Option::Some)
}

pub fn add_native_methods(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
    let string_length = RawMethod {
        access_flags: access!(public native),
        name: "length".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(native_property(StringObj::SELF, |s| {
            s.len() as u32
        }))),
        ..Default::default()
    };
    let char_at = RawMethod {
        access_flags: access!(public native),
        name: "charAt".into(),
        descriptor: method!((int) -> char),
        code: RawCode::native(NativeSingleMethod(native_string_char_at)),
        ..Default::default()
    };
    let string_value_of = RawMethod {
        access_flags: access!(public native),
        name: "valueOf".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeSingleMethod(native_string_value_of)),
        ..Default::default()
    };
    let string_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeSingleMethod(|_: &mut _, [l]: [u32; 1], _| {
            Ok(Some(l))
        })),
        ..Default::default()
    };
    let string_compare_to = RawMethod {
        access_flags: access!(public native),
        name: "compareTo".into(),
        descriptor: method!(((Object(java_lang_string.clone()))) -> int),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, other]: [u32; 2], _| {
                let this_str = StringObj::inspect(&thread.heap, this as usize, |a| a.clone())?;
                let other_str = StringObj::inspect(&thread.heap, other as usize, |a| a.clone())?;
                Ok(Some(match this_str.cmp(&other_str) {
                    std::cmp::Ordering::Less => u32::MAX,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }))
            },
        )),
        ..Default::default()
    };
    let string_contains = RawMethod {
        name: "contains".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object("java/lang/CharSequence".into()))) -> boolean),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, seq]: [u32; 2], _verbose| {
                let this_str = StringObj::inspect(&thread.heap, this as usize, |s| s.clone())?;
                let seq_str = StringObj::inspect(&thread.heap, seq as usize, |s| s.clone())?;
                let contains = this_str.contains(&*seq_str);
                Ok(Some(u32::from(contains)))
            },
        )),
        ..Default::default()
    };
    let mut string = RawClass::new(
        access!(public native),
        java_lang_string.clone(),
        java_lang_object.clone(),
    );
    string.register_methods(
        [
            string_length,
            char_at,
            string_value_of,
            string_to_string,
            string_compare_to,
            string_contains,
        ],
        method_area,
    );
    class_area.push(string);
}
