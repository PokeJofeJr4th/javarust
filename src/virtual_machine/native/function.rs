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

pub fn make_option(thread: &Thread, value: u32) -> u32 {
    let mut opt = Object::from_class(&thread.class_area.search("java/util/Optional").unwrap());
    opt.fields[0] = value;
    let idx = thread.heap.lock().unwrap().allocate(opt);
    idx
}

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

    composed_function.register_method(compose_apply, method_area);
    function.register_methods([apply, and_then, identity, identity_lambda], method_area);

    let mut optional = RawClass::new(
        access!(public native),
        "java/util/Optional".into(),
        java_lang_object.clone(),
    );
    optional.fields.push((
        Field {
            access_flags: access!(public),
            name: "$value".into(),
            descriptor: field!(Object(java_lang_object.clone())),
            ..Default::default()
        },
        0,
    ));
    optional.field_size += 1;

    let opt_empty = RawMethod {
        name: "empty".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, []: [u32; 0], _verbose| Ok(Some(make_option(thread, u32::MAX))),
        )),
        ..Default::default()
    };
    let opt_of = RawMethod {
        name: "of".into(),
        access_flags: access!(public static native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [idx]: [u32; 1], _verbose: bool| {
                Ok(Some(make_option(thread, idx)))
            },
        )),
        ..Default::default()
    };
    let opt_of_nullable = RawMethod {
        name: "ofNullable".into(),
        access_flags: access!(public static native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [idx]: [u32; 1], _verbose: bool| {
                Ok(Some(make_option(
                    thread,
                    if idx == 0 { u32::MAX } else { idx },
                )))
            },
        )),
        ..Default::default()
    };
    optional.register_methods([opt_empty, opt_of, opt_of_nullable], method_area);

    class_area.extend([function, composed_function, optional]);
}
