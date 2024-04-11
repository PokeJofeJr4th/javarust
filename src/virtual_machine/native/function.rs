use std::sync::Arc;

use crate::{
    access,
    class::{code::NativeSingleMethod, Field, MethodHandle},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    field, method,
    virtual_machine::{
        object::{AnyObj, LambdaOverride, Object, ObjectFinder},
        Thread,
    },
};

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
) {
    let mut function = RawClass::new(
        access!(public native abstract),
        "java/util/Function".into(),
        java_lang_object.clone(),
    );
    let mut composed_function = RawClass::new(
        access!(public native),
        "java/util/Function$Compose".into(),
        function.this.clone(),
    );
    composed_function.fields.extend([
        (
            Field {
                access_flags: access!(public),
                name: "$first".into(),
                descriptor: field!(Object(function.this.clone())),
                ..Default::default()
            },
            0,
        ),
        (
            Field {
                access_flags: access!(public),
                name: "$second".into(),
                descriptor: field!(Object(function.this.clone())),
                ..Default::default()
            },
            1,
        ),
    ]);
    composed_function.field_size += 2;

    let apply = RawMethod {
        name: "apply".into(),
        access_flags: access!(public abstract),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
        code: RawCode::Abstract,
        ..Default::default()
    };
    let apply_name = apply.name.clone();
    let apply_signature = apply.descriptor.clone();
    let and_then = RawMethod {
        name: "andThen".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(function.this.clone()))) -> Object(function.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, after]: [u32; 2], _verbose| {
                let mut compose = Object::from_class(
                    &thread
                        .class_area
                        .search("java/util/Function$Compose")
                        .unwrap(),
                );
                compose.fields[0] = this;
                compose.fields[1] = after;
                let idx = thread.heap.lock().unwrap().allocate(compose);
                Ok(Some(idx))
            },
        )),
        ..Default::default()
    };
    let compose = RawMethod {
        name: "compose".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(function.this.clone()))) -> Object(function.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, before]: [u32; 2], _verbose| {
                let mut compose = Object::from_class(
                    &thread
                        .class_area
                        .search("java/util/Function$Compose")
                        .unwrap(),
                );
                compose.fields[0] = before;
                compose.fields[1] = this;
                let idx = thread.heap.lock().unwrap().allocate(compose);
                Ok(Some(idx))
            },
        )),
        ..Default::default()
    };
    let identity = {
        let apply_name = apply_name.clone();
        let apply_signature = apply_signature.clone();
        let function_this = function.this.clone();
        let java_lang_object = java_lang_object.clone();
        RawMethod {
            name: "identity".into(),
            access_flags: access!(public static native),
            descriptor: method!(() -> Object(function.this.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, []: [u32; 0], _verbose| {
                    let lambda = Object {
                        fields: Vec::new(),
                        native_fields: vec![Box::new(LambdaOverride {
                            method_name: apply_name.clone(),
                            method_descriptor: apply_signature.clone(),
                            invoke: MethodHandle::InvokeStatic {
                                class: function_this.clone(),
                                name: "$identity".into(),
                                method_type: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
                            },
                            captures: Vec::new(),
                        })],
                        class: function_this.clone(),
                    };
                    let idx = thread.heap.lock().unwrap().allocate(lambda);
                    Ok(Some(idx))
                },
            )),
            ..Default::default()
        }
    };
    let identity_lambda = RawMethod {
        name: "$identity".into(),
        access_flags: access!(public static native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
        code: RawCode::native(NativeSingleMethod(
            |_thread: &mut Thread, [o]: [u32; 1], _verbose| Ok(Some(o)),
        )),
        ..Default::default()
    };

    let compose_apply = RawMethod {
        name: apply_name,
        access_flags: access!(public native),
        descriptor: apply_signature.clone(),
        code: RawCode::native(NativeSingleMethod(
            move |thread: &mut Thread, [this, arg]: [u32; 2], verbose| {
                match thread.pc_register {
                    0 => {
                        // invoke the first function
                        let first_fn_ptr =
                            AnyObj.inspect(&thread.heap, this as usize, |compose| {
                                compose.fields[0]
                            })?;
                        // push the return address
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(
                            first_fn_ptr,
                            "apply",
                            &apply_signature,
                            verbose,
                        )?;
                        thread.stackframe.locals[0] = first_fn_ptr;
                        thread.stackframe.locals[1] = arg;
                        Ok(None)
                    }
                    1 => {
                        // invoke the second function
                        // invoke the first function
                        let second_fn_ptr =
                            AnyObj.inspect(&thread.heap, this as usize, |compose| {
                                compose.fields[1]
                            })?;
                        // get the first value returned
                        let first_return = thread.stackframe.operand_stack.pop().unwrap();
                        // push the return address
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(
                            second_fn_ptr,
                            "apply",
                            &apply_signature,
                            verbose,
                        )?;
                        thread.stackframe.locals[0] = second_fn_ptr;
                        thread.stackframe.locals[1] = first_return;
                        Ok(None)
                    }
                    2 => {
                        // return the result
                        Ok(Some(thread.stackframe.operand_stack.pop().unwrap()))
                    }
                    pc => Err(format!("Invalid PC: {pc}").into()),
                }
            },
        )),
        ..Default::default()
    };

    composed_function
        .methods
        .push(compose_apply.name(composed_function.this.clone()));
    function.methods.extend([
        apply.name(function.this.clone()),
        and_then.name(function.this.clone()),
        identity.name(function.this.clone()),
        identity_lambda.name(function.this.clone()),
    ]);
    method_area.extend([
        (function.this.clone(), apply),
        (function.this.clone(), and_then),
        (function.this.clone(), compose),
        (function.this.clone(), identity),
        (function.this.clone(), identity_lambda),
        (composed_function.this.clone(), compose_apply),
    ]);
    class_area.extend([function, composed_function]);
}
