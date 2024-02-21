use std::sync::Mutex;

use crate::{
    class::{FieldType, MethodDescriptor, NativeMethod},
    virtual_machine::{
        object::{AnyObj, ObjectFinder, StringObj},
        StackFrame, Thread,
    },
};

pub struct NativeStringValueOf;

impl NativeMethod for NativeStringValueOf {
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        verbose: bool,
    ) -> Result<(), String> {
        native_string_value_of(thread, stackframe, verbose)
    }
}

pub fn native_string_value_of(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    verbose: bool,
) -> Result<(), String> {
    let obj_ref = stackframe.lock().unwrap().locals[0];
    if obj_ref == u32::MAX {
        let str_ref = thread
            .heap
            .lock()
            .unwrap()
            .allocate(StringObj::new(&thread.class_area, "null".into()));
        stackframe.lock().unwrap().operand_stack.push(str_ref);
    } else {
        let (to_string_class, to_string_method) =
            AnyObj.get(&thread.heap.lock().unwrap(), obj_ref as usize, |obj| {
                obj.resolve_method(
                    &thread.method_area,
                    &thread.class_area,
                    "toString",
                    &MethodDescriptor {
                        parameter_size: 0,
                        parameters: Vec::new(),
                        return_type: Some(FieldType::Object("java/lang/String".into())),
                    },
                )
            })?;
        let stackframes = thread.stack.len();
        // push a fake return address
        thread
            .stack
            .last_mut()
            .unwrap()
            .lock()
            .unwrap()
            .operand_stack
            .push(0);
        thread.invoke_method(to_string_method, to_string_class);
        thread.stack.last_mut().unwrap().lock().unwrap().locals[0] = obj_ref;
        while thread.stack.len() > stackframes {
            thread.tick(verbose)?;
        }
    }
    thread.return_one(verbose);
    Ok(())
}

pub fn native_println_object(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    verbose: bool,
) -> Result<(), String> {
    // println!("{stackframe:?}");
    let arg = stackframe.lock().unwrap().locals[1];
    let (to_string_class, to_string_method) =
        AnyObj.get(&thread.heap.lock().unwrap(), arg as usize, |obj| {
            obj.resolve_method(
                &thread.method_area,
                &thread.class_area,
                "toString",
                &MethodDescriptor {
                    parameter_size: 0,
                    parameters: Vec::new(),
                    return_type: Some(FieldType::Object("java/lang/String".into())),
                },
            )
        })?;
    if verbose {
        println!(
            "Resolved java/lang/Object.toString to {}.{}",
            to_string_class.this, to_string_method.name
        );
    }
    let stackframes = thread.stack.len();
    // push a fake return address
    stackframe.lock().unwrap().operand_stack.push(0);
    thread.invoke_method(to_string_method, to_string_class);
    thread.stack.last_mut().unwrap().lock().unwrap().locals[0] = arg;
    while thread.stack.len() > stackframes {
        thread.tick(verbose)?;
    }
    let ret = stackframe.lock().unwrap().operand_stack.pop().unwrap();
    let str = StringObj.get(&thread.heap.lock().unwrap(), ret as usize, Clone::clone)?;
    println!("{str}");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
pub fn native_string_len(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    _verbose: bool,
) -> Result<u32, String> {
    let string_ref = stackframe.lock().unwrap().locals[0];
    StringObj.get(&thread.heap.lock().unwrap(), string_ref as usize, |str| {
        str.len() as u32
    })
}

pub fn native_string_char_at(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    _verbose: bool,
) -> Result<u32, String> {
    let string_ref = stackframe.lock().unwrap().locals[0];
    let index = stackframe.lock().unwrap().locals[1] as usize;
    StringObj.get(&thread.heap.lock().unwrap(), string_ref as usize, |str| {
        str.chars().nth(index).unwrap() as u32
    })
}
