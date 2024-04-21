use std::sync::Arc;

use crate::{
    access,
    class::{
        code::{NativeMethod, NativeSingleMethod, NativeVoid},
        Field, MethodDescriptor, MethodHandle,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    field, method,
    virtual_machine::{
        object::{AnyObj, LambdaOverride, Object, ObjectFinder, StringObj},
        Thread,
    },
};

pub struct Optional;

impl Optional {
    pub fn make(thread: &Thread, value: u32, verbose: bool) -> u32 {
        let mut opt = Object::from_class(&thread.class_area.search("java/util/Optional").unwrap());
        opt.fields[0] = value;
        if value != u32::MAX {
            thread.rember(value, verbose);
        }
        let idx = thread.heap.lock().unwrap().allocate(opt);
        idx
    }
}

impl ObjectFinder for Optional {
    type Target<'a> = &'a mut Option<u32>;

    fn extract<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> crate::virtual_machine::error::Result<T> {
        let value = object.fields[0];
        let mut value = if value == u32::MAX { None } else { Some(value) };
        let output = func(&mut value);
        object.fields[0] = value.unwrap_or(u32::MAX);
        Ok(output)
    }
}

pub fn make_lambda_override<const CAPTURES: usize>(
    overrided_name: &Arc<str>,
    overrided_descriptor: &MethodDescriptor,
    instance_class: &Arc<str>,
    invoke_name: &Arc<str>,
    invoke_descriptor: &MethodDescriptor,
    invoke_class: &Arc<str>,
) -> impl NativeMethod {
    let method_name = overrided_name.clone();
    let method_descriptor = overrided_descriptor.clone();
    let invoke = MethodHandle::InvokeStatic {
        class: invoke_class.clone(),
        name: invoke_name.clone(),
        method_type: invoke_descriptor.clone(),
    };
    let instance_class = instance_class.clone();
    NativeSingleMethod(
        move |thread: &mut Thread, caps: [u32; CAPTURES], _verbose| {
            let lambda_object = LambdaOverride {
                method_name: method_name.clone(),
                method_descriptor: method_descriptor.clone(),
                invoke: invoke.clone(),
                captures: caps.to_vec(),
            }
            .as_object(instance_class.clone());
            let idx = thread.heap.lock().unwrap().allocate(lambda_object);
            Ok(Some(idx))
        },
    )
}

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
    let mut function = RawClass::new(
        access!(public native abstract),
        "java/util/function/Function".into(),
        java_lang_object.clone(),
    );
    function.static_data.extend([0, 0]);
    function.statics.extend([(
        Field {
            access_flags: access!(public static),
            name: "$IDENTITY".into(),
            descriptor: field!(Object(function.this.clone())),
            ..Default::default()
        },
        0,
    )]);

    let apply = RawMethod {
        name: "apply".into(),
        access_flags: access!(public abstract),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
        code: RawCode::Abstract,
        ..Default::default()
    };
    let apply_name = apply.name.clone();
    let apply_signature = apply.descriptor.clone();
    let compose_lambda = {
        let apply_signature = apply_signature.clone();
        RawMethod {
            name: "$compose".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object(function.this.clone())), (Object(function.this.clone())), (Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread,
                      [first_fn_ptr, second_fn_ptr, arg]: [u32; 3],
                      verbose| {
                    match thread.pc_register {
                        0 => {
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
        }
    };
    let compose = {
        let function_this = function.this.clone();
        let apply_name = apply.name.clone();
        let apply_signature = apply.descriptor.clone();
        let compose_handle = MethodHandle::InvokeStatic {
            class: function_this.clone(),
            name: "$compose".into(),
            method_type: compose_lambda.descriptor.clone(),
        };
        RawMethod {
            name: "compose".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object(function.this.clone()))) -> Object(function.this.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, before]: [u32; 2], _verbose| {
                    let lambda_object = LambdaOverride {
                        method_name: apply_name.clone(),
                        method_descriptor: apply_signature.clone(),
                        invoke: compose_handle.clone(),
                        captures: vec![before, this],
                    }
                    .as_object(function_this.clone());
                    let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                    Ok(Some(idx))
                },
            )),
            ..Default::default()
        }
    };
    let and_then = RawMethod {
        name: "andThen".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(function.this.clone()))) -> Object(function.this.clone())),
        code: RawCode::native(make_lambda_override::<2>(
            &apply.name,
            &apply.descriptor,
            &function.this,
            &compose_lambda.name,
            &compose_lambda.descriptor,
            &function.this,
        )),
        ..Default::default()
    };
    let function_clinit = {
        let function_this = function.this.clone();
        let identity_override = LambdaOverride {
            method_name: apply_name,
            method_descriptor: apply_signature,
            invoke: MethodHandle::InvokeStatic {
                class: function.this.clone(),
                name: "$identity".into(),
                method_type: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
            },
            captures: Vec::new(),
        };
        RawMethod {
            name: "<clinit>".into(),
            access_flags: access!(static native),
            descriptor: method!(() -> void),
            code: RawCode::native(NativeVoid(
                move |thread: &mut Thread, []: [u32; 0], _verbose| {
                    let identity_lambda = Object {
                        fields: Vec::new(),
                        native_fields: vec![Box::new(identity_override.clone())],
                        class: function_this.clone(),
                    };
                    let idx = thread.heap.lock().unwrap().allocate(identity_lambda);
                    thread
                        .class_area
                        .search("java/util/function/Function")
                        .unwrap()
                        .static_data
                        .lock()
                        .unwrap()[0] = idx;
                    Ok(Some(()))
                },
            )),
            ..Default::default()
        }
    };
    let identity = RawMethod {
        name: "identity".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> Object(function.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            move |thread: &mut Thread, []: [u32; 0], _verbose| {
                let function = thread
                    .class_area
                    .search("java/util/function/Function")
                    .unwrap();
                if thread.maybe_initialize_class(&function) {
                    return Ok(None);
                }
                let value = function.static_data.lock().unwrap()[0];
                Ok(Some(value))
            },
        )),
        ..Default::default()
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

    function.register_methods(
        [
            apply,
            and_then,
            compose,
            identity,
            identity_lambda,
            function_clinit,
            compose_lambda,
        ],
        method_area,
    );

    let mut predicate = RawClass::new(
        access!(public native),
        "java/util/function/Predicate".into(),
        java_lang_object.clone(),
    );
    let predicate_test = RawMethod {
        name: "test".into(),
        access_flags: access!(public abstract),
        descriptor: method!(((Object(java_lang_object.clone()))) -> boolean),
        code: RawCode::Abstract,
        ..Default::default()
    };
    let predicate_neg_lambda = {
        let test_signature = predicate_test.descriptor.clone();
        RawMethod {
            name: "$negative".into(),
            access_flags: access!(public static),
            descriptor: method!(((Object(predicate.this.clone())), (Object(java_lang_object.clone()))) -> boolean),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [predicate, object]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(predicate, "test", &test_signature, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = object;
                        Ok(None)
                    }
                    1 => {
                        // reverse the output
                        let value = thread.stackframe.operand_stack.pop().unwrap();
                        Ok(Some(u32::from(value == 0)))
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    let predicate_negate = RawMethod {
        name: "negate".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> Object(predicate.this.clone())),
        code: RawCode::native(make_lambda_override::<1>(
            &predicate_test.name,
            &predicate_test.descriptor,
            &predicate.this,
            &predicate_neg_lambda.name,
            &predicate_neg_lambda.descriptor,
            &predicate.this,
        )),
        ..Default::default()
    };
    let predicate_not = RawMethod {
        name: "negate".into(),
        access_flags: access!(public static native),
        descriptor: method!(((Object(predicate.this.clone()))) -> Object(predicate.this.clone())),
        code: RawCode::native(make_lambda_override::<1>(
            &predicate_test.name,
            &predicate_test.descriptor,
            &predicate.this,
            &predicate_neg_lambda.name,
            &predicate_neg_lambda.descriptor,
            &predicate.this,
        )),
        ..Default::default()
    };
    let is_equal = {
        let equals_handle = MethodHandle::InvokeStatic {
            class: "java/util/Objects".into(),
            name: "equals".into(),
            method_type: method!(((Object(java_lang_object.clone()))) -> boolean),
        };
        let test_name = predicate_test.name.clone();
        let test_signature = predicate_test.descriptor.clone();
        let predicate_name = predicate.this.clone();
        RawMethod {
            name: "isEqual".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object(java_lang_object.clone()))) -> Object(predicate.this.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [target_ref]: [u32; 1], _verbose| {
                    let lambda_object = LambdaOverride {
                        method_name: test_name.clone(),
                        method_descriptor: test_signature.clone(),
                        invoke: equals_handle.clone(),
                        captures: vec![target_ref],
                    }
                    .as_object(predicate_name.clone());
                    let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                    Ok(Some(idx))
                },
            )),
            ..Default::default()
        }
    };
    let predicate_and_lambda = {
        let test_signature = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "$and".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object(predicate.this.clone())), (Object(predicate.this.clone())), (Object(java_lang_object.clone()))) -> boolean),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [first, second, operand]: [u32; 3], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(first, "test", &test_signature, verbose)?;
                        thread.stackframe.locals[0] = first;
                        thread.stackframe.locals[1] = operand;
                        Ok(None)
                    }
                    1 => {
                        let ret = thread.stackframe.operand_stack.pop().unwrap();
                        if ret == 0 {
                            return Ok(Some(0));
                        }
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(second, "test", &test_signature, verbose)?;
                        thread.stackframe.locals[0] = second;
                        thread.stackframe.locals[1] = operand;
                        Ok(None)
                    }
                    2 => Ok(Some(thread.stackframe.operand_stack.pop().unwrap())),
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    let predicate_or_lambda = {
        let test_signature = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "$or".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object(predicate.this.clone())), (Object(predicate.this.clone())), (Object(java_lang_object.clone()))) -> boolean),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [first, second, operand]: [u32; 3], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(first, "test", &test_signature, verbose)?;
                        thread.stackframe.locals[0] = first;
                        thread.stackframe.locals[1] = operand;
                        Ok(None)
                    }
                    1 => {
                        let ret = thread.stackframe.operand_stack.pop().unwrap();
                        if ret == 1 {
                            return Ok(Some(1));
                        }
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(second, "test", &test_signature, verbose)?;
                        thread.stackframe.locals[0] = second;
                        thread.stackframe.locals[1] = operand;
                        Ok(None)
                    }
                    2 => Ok(Some(thread.stackframe.operand_stack.pop().unwrap())),
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    let predicate_and = RawMethod {
        name: "and".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(predicate.this.clone()))) -> Object(predicate.this.clone())),
        code: RawCode::native(make_lambda_override::<2>(
            &predicate_test.name,
            &predicate_test.descriptor,
            &predicate.this,
            &predicate_and_lambda.name,
            &predicate_and_lambda.descriptor,
            &predicate.this,
        )),
        ..Default::default()
    };
    let predicate_or = RawMethod {
        name: "or".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(predicate.this.clone()))) -> Object(predicate.this.clone())),
        code: RawCode::native(make_lambda_override::<2>(
            &predicate_test.name,
            &predicate_test.descriptor,
            &predicate.this,
            &predicate_or_lambda.name,
            &predicate_or_lambda.descriptor,
            &predicate.this,
        )),
        ..Default::default()
    };
    predicate.register_methods(
        [
            predicate_test,
            predicate_neg_lambda,
            predicate_not,
            is_equal,
            predicate_negate,
            predicate_and_lambda,
            predicate_or_lambda,
            predicate_and,
            predicate_or,
        ],
        method_area,
    );

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
    optional.statics.push((
        Field {
            access_flags: access!(public static),
            name: "$EMPTY".into(),
            descriptor: field!(Object(java_lang_object.clone())),
            ..Default::default()
        },
        0,
    ));
    optional.static_data.push(0);

    let opt_clinit = RawMethod {
        name: "<clinit>".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeVoid(|thread: &mut Thread, []: [u32; 0], verbose| {
            let optional = thread.class_area.search("java/util/Optional").unwrap();
            optional.static_data.lock().unwrap()[0] = Optional::make(thread, u32::MAX, verbose);
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let opt_empty = RawMethod {
        name: "empty".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, []: [u32; 0], _verbose| {
                let optional = thread.class_area.search("java/util/Optional").unwrap();
                if thread.maybe_initialize_class(&optional) {
                    return Ok(None);
                }
                let empty = optional.static_data.lock().unwrap()[0];
                Ok(Some(empty))
            },
        )),
        ..Default::default()
    };
    let opt_of = RawMethod {
        name: "of".into(),
        access_flags: access!(public static native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [idx]: [u32; 1], verbose: bool| {
                Ok(Some(Optional::make(thread, idx, verbose)))
            },
        )),
        ..Default::default()
    };
    let opt_of_nullable = RawMethod {
        name: "ofNullable".into(),
        access_flags: access!(public static native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [idx]: [u32; 1], verbose: bool| {
                if idx != 0 {
                    return Ok(Some(Optional::make(thread, idx, verbose)));
                }
                let nullable = thread.class_area.search("java/util/Optional").unwrap();
                if thread.maybe_initialize_class(&nullable) {
                    return Ok(None);
                }
                let null = nullable.static_data.lock().unwrap()[0];
                Ok(Some(null))
            },
        )),
        ..Default::default()
    };
    let equals_descriptor = method!(((Object(java_lang_object.clone()))) -> boolean);
    let opt_equals = RawMethod {
        name: "equals".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> boolean),
        code: RawCode::native(NativeSingleMethod(
            move |thread: &mut Thread, [this, other]: [u32; 2], verbose| match thread.pc_register {
                0 => {
                    let Some(other_inner) = AnyObj.inspect(&thread.heap, other as usize, |o| {
                        if o.isinstance(&thread.class_area, "java/util/Optional", verbose) {
                            Some(o.fields[0])
                        } else {
                            None
                        }
                    })?
                    else {
                        // if it's not an Optional, return false
                        return Ok(Some(0));
                    };
                    let this_inner =
                        AnyObj.inspect(&thread.heap, this as usize, |o| o.fields[0])?;
                    // if they're both None, they're equal
                    if this_inner == u32::MAX && other_inner == u32::MAX {
                        return Ok(Some(1));
                    }
                    // if one's None but the other isn't, they're not equal
                    if this_inner == u32::MAX || other_inner == u32::MAX {
                        return Ok(Some(0));
                    }
                    thread.stackframe.operand_stack.push(1);
                    thread.resolve_and_invoke(this_inner, "equals", &equals_descriptor, verbose)?;
                    thread.stackframe.locals[0] = this_inner;
                    thread.stackframe.locals[1] = other_inner;
                    Ok(None)
                }
                // re-return the value from Object.equals
                1 => Ok(Some(thread.stackframe.operand_stack.pop().unwrap())),
                _ => unreachable!(),
            },
        )),
        ..Default::default()
    };
    let opt_filter = {
        let test_signature = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "filter".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object(predicate.this.clone()))) -> Object(optional.this.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        let this_value =
                            AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;

                        if this_value == u32::MAX {
                            let optional = thread.class_area.search("java/util/Optional").unwrap();
                            if thread.maybe_initialize_class(&optional) {
                                return Ok(None);
                            }
                            let null = optional.static_data.lock().unwrap()[0];
                            return Ok(Some(null));
                        }

                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(predicate, "test", &test_signature, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = this_value;
                        Ok(None)
                    }
                    1 => {
                        let result = *thread.stackframe.operand_stack.last().unwrap();

                        if result == 0 {
                            let optional = thread.class_area.search("java/util/Optional").unwrap();
                            if thread.maybe_initialize_class(&optional) {
                                return Ok(None);
                            }
                            let null = optional.static_data.lock().unwrap()[0];
                            return Ok(Some(null));
                        }
                        Ok(Some(this))
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    // TODO: flatMap
    // TODO: get
    let hash_code_descriptor = method!(() -> int);
    let opt_hash_code = RawMethod {
        name: "hashCode".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(
            move |thread: &mut Thread, [this]: [u32; 1], verbose| {
                match thread.pc_register {
                    0 => {
                        let value =
                            AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;
                        if value == u32::MAX {
                            Ok(Some(0))
                        } else {
                            // push return address
                            thread.stackframe.operand_stack.push(1);
                            thread.resolve_and_invoke(
                                value,
                                "hashCode",
                                &hash_code_descriptor,
                                verbose,
                            )?;
                            thread.stackframe.locals[0] = value;
                            Ok(None)
                        }
                    }
                    1 => Ok(Some(thread.stackframe.operand_stack.pop().unwrap())),
                    _ => unreachable!(),
                }
            },
        )),
        ..Default::default()
    };
    // TODO: ifPresent
    // TODO: ifPresentOrElse
    let is_empty = RawMethod {
        name: "isEmpty".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> boolean),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this]: [u32; 1], _verbose| {
                Ok(Some(
                    AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0] == u32::MAX)?
                        as u32,
                ))
            },
        )),
        ..Default::default()
    };
    let is_present = RawMethod {
        name: "isPresent".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> boolean),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this]: [u32; 1], _verbose| {
                Ok(Some(
                    AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0] != u32::MAX)?
                        as u32,
                ))
            },
        )),
        ..Default::default()
    };
    // TODO: map
    // TODO: orElse
    // TODO: orElseGet
    // TODO: orElseThrow
    // TODO: stream
    let to_string_descriptor = method!(() -> Object(java_lang_string.clone()));
    let opt_to_string =
        RawMethod::to_string(move |thread: &mut Thread, [this]: [u32; 1], verbose| {
            match thread.pc_register {
                0 => {
                    let value = AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;
                    if value == u32::MAX {
                        Ok(Some("None".into()))
                    } else {
                        // push return address
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(
                            value,
                            "toString",
                            &to_string_descriptor,
                            verbose,
                        )?;
                        thread.stackframe.locals[0] = value;
                        Ok(None)
                    }
                }
                1 => StringObj::inspect(
                    &thread.heap,
                    thread.stackframe.operand_stack.pop().unwrap() as usize,
                    |s| Some(format!("Some({s})").into()),
                ),
                _ => unreachable!(),
            }
        });
    optional.register_methods(
        [
            opt_empty,
            opt_of,
            opt_of_nullable,
            is_empty,
            is_present,
            opt_to_string,
            opt_hash_code,
            opt_equals,
            opt_clinit,
            opt_filter,
        ],
        method_area,
    );

    let mut consumer = RawClass::new(
        access!(public native abstract),
        "java/util/function/Consumer".into(),
        java_lang_object.clone(),
    );

    let consumer_accept = RawMethod {
        name: "accept".into(),
        access_flags: access!(public abstract native),
        descriptor: method!(((Object(java_lang_object.clone()))) -> void),
        code: RawCode::Abstract,
        ..Default::default()
    };
    consumer.register_method(consumer_accept, method_area);

    class_area.extend([function, optional, predicate, consumer]);
}
