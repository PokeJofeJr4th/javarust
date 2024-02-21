use std::sync::{Arc, Mutex};

use crate::{
    class::{FieldType, MethodDescriptor, NativeMethod, NativeSingleMethod},
    virtual_machine::{
        object::{AnyObj, ObjectFinder, StringObj},
        thread::heap_allocate,
        StackFrame, Thread,
    },
};

pub struct StringValueOf;

impl NativeMethod for StringValueOf {
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        verbose: bool,
    ) -> Result<(), String> {
        string_value_of(thread, stackframe, verbose)
    }
}

pub fn string_value_of(
    thread: &mut Thread,
    stackframe: &Mutex<StackFrame>,
    verbose: bool,
) -> Result<(), String> {
    let obj_ref = stackframe.lock().unwrap().locals[0];
    if obj_ref == u32::MAX {
        let str_ref = heap_allocate(
            &mut thread.heap.lock().unwrap(),
            StringObj::new(&thread.class_area, "null".into()),
        );
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
