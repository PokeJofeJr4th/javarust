use crate::{
    class::{code::NativeReturn, FieldType, MethodDescriptor},
    data::NULL,
    virtual_machine::{
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
