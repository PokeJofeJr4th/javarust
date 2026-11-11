use std::sync::Arc;

use jvmrs_lib::{access, field, method, MethodHandle};

use crate::{
    class::{code::NativeSingleMethod, Field},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    native_class,
    virtual_machine::{
        native::field::Boolean,
        object::{AnyObj, LambdaOverride, Object, ObjectFinder, StringObj},
        Thread,
    },
};

pub struct Optional;

impl Optional {
    pub fn make(thread: &Thread, value: u32, verbose: bool) -> u32 {
        let mut opt = Object::from_class(thread.class_area.search("java/util/Optional").unwrap());
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

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
    class_area.extend(native_class!{
        use method_area;

        impl java::util::function::Function {
            static "$IDENTITY": java::util::function::Function = NULL;

            fn apply(&this, val: java::lang::Object) -> java::lang::Object;

            fn "<clinit>"() {
                use thread, _v;
                let class = thread.class_area.search("java/util/function/Function").unwrap();
                let identity_lambda = Object {
                    fields: Vec::new(),
                    native_fields: vec![Box::new(LambdaOverride {
                        method_name: "apply".into(),
                        method_descriptor: method!(((Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                        invoke: MethodHandle::InvokeStatic {
                            class: "java/util/function/Function".into(),
                            name: "$identity".into(),
                            method_type: method!(((Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                        },
                        captures: Vec::new(),
                    })],
                    class,
                };
                let idx = thread.heap.lock().unwrap().allocate(identity_lambda);
                thread
                    .class_area
                    .search("java/util/function/Function")
                    .unwrap()
                    .static_data
                    .lock()
                    .unwrap()[0] = idx;
                Ok(())
            }

            fn "$identity"(o: java::lang::Object) -> java::lang::Object {
                use _t, _v;
                Ok(o)
            }

            fn identity() -> java::util::function::Function {
                use thread, verbose;
                let function = thread
                    .class_area
                    .search("java/util/function/Function")
                    .unwrap();
                if thread.maybe_initialize_class(&function) {
                    super::run_until_return(thread, verbose)?;
                }
                let value = function.static_data.lock().unwrap()[0];
                Ok(value)
            }

            fn "$compose"(
                first_fn_ptr: java::util::function::Function,
                second_fn_ptr: java::util::function::Function,
                arg: java::lang::Object
            ) -> java::lang::Object {
                use thread, verbose;
                // Run the first function
                super::resolve_and_invoke(
                    thread,
                    first_fn_ptr,
                    "apply",
                    &method!(((Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                    verbose,
                )?;
                thread.stackframe.locals[0] = first_fn_ptr;
                thread.stackframe.locals[1] = arg;
                super::run_until_return(thread, verbose)?;
                // get the first value returned
                let first_return = thread.stackframe.operand_stack.pop().unwrap();

                // Run the second function
                super::resolve_and_invoke(
                    thread,
                    second_fn_ptr,
                    "apply",
                    &method!(((Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                    verbose,
                )?;
                thread.stackframe.locals[0] = second_fn_ptr;
                thread.stackframe.locals[1] = first_return;
                super::run_until_return(thread, verbose)?;

                // Return the result
                Ok(thread.stackframe.operand_stack.pop().unwrap())
            }

            fn compose(&this, before: java::util::function::Function) -> java::util::function::Function {
                use thread, _v;
                let function_class = thread.class_area.search("java/util/function/Function").unwrap();
                let lambda_object = LambdaOverride {
                    method_name: "apply".into(),
                    method_descriptor: method!(((Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                    invoke: MethodHandle::InvokeStatic {
                        class: "java/util/function/Function".into(),
                        name: "$compose".into(),
                        method_type: method!(((Object("java/util/function/Function".into())), (Object("java/util/function/Function".into())), (Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                    },
                    captures: vec![before, this],
                }
                .as_object(function_class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn andThen(&this, before: java::util::function::Function) -> java::util::function::Function {
                use thread, _v;
                let function_class = thread.class_area.search("java/util/function/Function").unwrap();
                let lambda_object = LambdaOverride {
                    method_name: "apply".into(),
                    method_descriptor: method!(((Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                    invoke: MethodHandle::InvokeStatic {
                        class: "java/util/function/Function".into(),
                        name: "$compose".into(),
                        method_type: method!(((Object("java/util/function/Function".into())), (Object("java/util/function/Function".into())), (Object("java/lang/Object".into()))) -> Object("java/lang/Object".into())),
                    },
                    captures: vec![this, before],
                }
                .as_object(function_class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }
        }

        impl java::util::function::Predicate {
            fn test(&this, obj: java::lang::Object) -> Boolean;

            fn "$negate"(predicate: java::util::function::Predicate, object: java::lang::Object) -> Boolean {
                use thread, verbose;
                super::resolve_and_invoke(thread, predicate, "test", &method!(((Object("java/lang/Object".into()))) -> boolean), verbose)?;
                thread.stackframe.locals[0] = predicate;
                thread.stackframe.locals[1] = object;
                super::run_until_return(thread, verbose)?;
                let value = thread.stackframe.operand_stack.pop().unwrap();
                Ok(value == 0)
            }

            fn negate(&this) -> java::util::function::Predicate {
                use thread, _v;
                let invoke = MethodHandle::InvokeStatic {
                    class: "java/util/function/Predicate".into(),
                    name: "$negate".into(),
                    method_type: method!(((Object("java/util/function/Predicate".into())), (Object("java/lang/Object".into()))) -> boolean),
                };
                let class = thread.class_area.search("java/util/function/Predicate").unwrap();
                let lambda_object = LambdaOverride {
                    method_name: "test".into(),
                    method_descriptor: method!(((Object("java/lang/Object".into()))) -> boolean),
                    invoke,
                    captures: vec![this],
                }
                .as_object(class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn isEqual(obj: java::lang::Object) -> java::util::function::Predicate {
                use thread, _v;

                let equals_handle = MethodHandle::InvokeStatic {
                    class: "java/lang/Objects".into(),
                    name: "equals".into(),
                    method_type: method!(((Object("java/lang/Object".into()))) -> boolean),
                };

                let predicate_class = thread.class_area.search("java/util/function/Predicate").unwrap();
                let lambda_object = LambdaOverride {
                    method_name: "test".into(),
                    method_descriptor: method!(((Object("java/lang/Object".into()))) -> boolean),
                    invoke: equals_handle,
                    captures: vec![obj],
                }
                .as_object(predicate_class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn "$and"(
                first: java::util::function::Predicate,
                second: java::util::function::Predicate,
                operand: java::lang::Object
            ) -> Boolean {
                use thread, verbose;
                super::resolve_and_invoke(thread, first, "test", &method!(((Object("java/lang/Object".into()))) -> boolean), verbose)?;
                thread.stackframe.locals[0] = first;
                thread.stackframe.locals[1] = operand;
                super::run_until_return(thread, verbose)?;
                let ret = thread.stackframe.operand_stack.pop().unwrap();
                if ret == 0 {
                    return Ok(false);
                }
                super::resolve_and_invoke(thread, second, "test", &method!(((Object("java/lang/Object".into()))) -> boolean), verbose)?;
                thread.stackframe.locals[0] = second;
                thread.stackframe.locals[1] = operand;
                super::run_until_return(thread, verbose)?;
                Ok(thread.stackframe.operand_stack.pop().unwrap() > 0)
            }

            fn and(&this, other: java::util::function::Predicate) -> java::util::function::Predicate {
                use thread, _v;
                let predicate_class = thread.class_area.search("java/util/function/Function").unwrap();
                let lambda_object = LambdaOverride {
                    method_name: "test".into(),
                    method_descriptor: method!(((Object("java/lang/Object".into()))) -> boolean),
                    invoke: MethodHandle::InvokeStatic {
                        class: "java/util/function/Predicate".into(),
                        name: "$and".into(),
                        method_type: method!(((Object("java/util/function/Predicate".into())), (Object("java/util/function/Predicate".into())), (Object("java/lang/Object".into()))) -> boolean),
                    },
                    captures: vec![this, other],
                }
                .as_object(predicate_class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn "$or"(
                first: java::util::function::Predicate,
                second: java::util::function::Predicate,
                operand: java::lang::Object
            ) -> Boolean {
                use thread, verbose;
                super::resolve_and_invoke(thread, first, "test", &method!(((Object("java/lang/Object".into()))) -> boolean), verbose)?;
                thread.stackframe.locals[0] = first;
                thread.stackframe.locals[1] = operand;
                super::run_until_return(thread, verbose)?;
                let ret = thread.stackframe.operand_stack.pop().unwrap();
                if ret > 0 {
                    return Ok(true);
                }
                super::resolve_and_invoke(thread, second, "test", &method!(((Object("java/lang/Object".into()))) -> boolean), verbose)?;
                thread.stackframe.locals[0] = second;
                thread.stackframe.locals[1] = operand;
                super::run_until_return(thread, verbose)?;
                Ok(thread.stackframe.operand_stack.pop().unwrap() > 0)
            }

            fn or(&this, other: java::util::function::Predicate) -> java::util::function::Predicate {
                use thread, _v;
                let predicate_class = thread.class_area.search("java/util/function/Function").unwrap();
                let lambda_object = LambdaOverride {
                    method_name: "test".into(),
                    method_descriptor: method!(((Object("java/lang/Object".into()))) -> boolean),
                    invoke: MethodHandle::InvokeStatic {
                        class: "java/util/function/Predicate".into(),
                        name: "$or".into(),
                        method_type: method!(((Object("java/util/function/Predicate".into())), (Object("java/util/function/Predicate".into())), (Object("java/lang/Object".into()))) -> boolean),
                    },
                    captures: vec![this, other],
                }
                .as_object(predicate_class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }
        }
    });

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

    let opt_clinit = RawMethod::clinit(|thread: &mut Thread, []: [u32; 0], verbose| {
        let optional = thread.class_area.search("java/util/Optional").unwrap();
        let value = Optional::make(thread, u32::MAX, verbose);
        optional.static_data.lock().unwrap()[0] = value;
        thread.rember(value, verbose);
        Ok(())
    });
    let opt_empty = RawMethod {
        name: "empty".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, []: [u32; 0], verbose| {
                let optional = thread.class_area.search("java/util/Optional").unwrap();
                if thread.maybe_initialize_class(&optional) {
                    super::run_until_return(thread, verbose)?;
                }
                let empty = optional.static_data.lock().unwrap()[0];
                Ok(empty)
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
                Ok(Optional::make(thread, idx, verbose))
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
                    return Ok(Optional::make(thread, idx, verbose));
                }
                let nullable = thread.class_area.search("java/util/Optional").unwrap();
                if thread.maybe_initialize_class(&nullable) {
                    super::run_until_return(thread, verbose)?;
                }
                let null = nullable.static_data.lock().unwrap()[0];
                Ok(null)
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
            move |thread: &mut Thread, [this, other]: [u32; 2], verbose| {
                let Some(other_inner) = AnyObj.inspect(&thread.heap, other as usize, |o| {
                    if o.isinstance(&thread.class_area, "java/util/Optional", verbose) {
                        Some(o.fields[0])
                    } else {
                        None
                    }
                })?
                else {
                    // if it's not an Optional, return false
                    return Ok(0);
                };
                let this_inner = AnyObj.inspect(&thread.heap, this as usize, |o| o.fields[0])?;

                // if they're both None, they're equal
                if this_inner == u32::MAX && other_inner == u32::MAX {
                    return Ok(1);
                }
                // if one's None but the other isn't, they're not equal
                if this_inner == u32::MAX || other_inner == u32::MAX {
                    return Ok(0);
                }

                // Call this.equals(other)
                super::resolve_and_invoke(
                    thread,
                    this_inner,
                    "equals",
                    &equals_descriptor,
                    verbose,
                )?;
                thread.stackframe.locals[0] = this_inner;
                thread.stackframe.locals[1] = other_inner;
                super::run_until_return(thread, verbose)?;
                Ok(thread.stackframe.operand_stack.pop().unwrap())
            },
        )),
        ..Default::default()
    };
    let opt_filter = {
        let test_signature = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "filter".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object("java/util/function/Predicate".into()))) -> Object(optional.this.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| {
                    let this_value =
                        AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;

                    // if we're already empty, return empty
                    if this_value == u32::MAX {
                        let optional = thread.class_area.search("java/util/Optional").unwrap();
                        if thread.maybe_initialize_class(&optional) {
                            super::run_until_return(thread, verbose)?;
                        }
                        let null = optional.static_data.lock().unwrap()[0];
                        return Ok(null);
                    }

                    // Call the predicate
                    super::resolve_and_invoke(thread, predicate, "test", &test_signature, verbose)?;
                    thread.stackframe.locals[0] = predicate;
                    thread.stackframe.locals[1] = this_value;
                    super::run_until_return(thread, verbose)?;

                    // If the predicate was false, return empty
                    let result = *thread.stackframe.operand_stack.last().unwrap();
                    if result == 0 {
                        let optional = thread.class_area.search("java/util/Optional").unwrap();
                        if thread.maybe_initialize_class(&optional) {
                            super::run_until_return(thread, verbose)?;
                        }
                        let null = optional.static_data.lock().unwrap()[0];
                        return Ok(null);
                    }
                    Ok(this)
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
                let value = AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;
                // If we're empty, return 0
                if value == u32::MAX {
                    return Ok(0);
                }
                // Call inner.hashCode
                super::resolve_and_invoke(
                    thread,
                    value,
                    "hashCode",
                    &hash_code_descriptor,
                    verbose,
                )?;
                thread.stackframe.locals[0] = value;
                super::run_until_return(thread, verbose)?;

                Ok(thread.stackframe.operand_stack.pop().unwrap())
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
                Ok(
                    AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0] == u32::MAX)?
                        as u32,
                )
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
                Ok(
                    AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0] != u32::MAX)?
                        as u32,
                )
            },
        )),
        ..Default::default()
    };
    let function_descriptor =
        method!(((Object(java_lang_object.clone()))) -> (Object(java_lang_object.clone())));
    let opt_map = RawMethod {
        name: "map".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object("java/util/function/Function".into()))) -> Object(optional.this.clone())),
        code: RawCode::native(NativeSingleMethod(
            move |thread: &mut Thread, [this, func]: [u32; 2], verbose| {
                let value = AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;
                // if we're empty, return empty
                if value == u32::MAX {
                    return Ok(this);
                }

                // Call func.apply
                super::resolve_and_invoke(thread, func, "apply", &function_descriptor, verbose)?;
                thread.stackframe.locals[1] = value;
                super::run_until_return(thread, verbose)?;

                // Make a new optional with the result
                let inner_value = thread.stackframe.operand_stack.pop().unwrap();
                Ok(Optional::make(thread, inner_value, verbose))
            },
        )),
        ..Default::default()
    };
    // TODO: orElse
    // TODO: orElseGet
    // TODO: orElseThrow
    // TODO: stream
    let to_string_descriptor = method!(() -> Object(java_lang_string.clone()));
    let opt_to_string =
        RawMethod::to_string(move |thread: &mut Thread, [this]: [u32; 1], verbose| {
            let value = AnyObj.inspect(&thread.heap, this as usize, |obj| obj.fields[0])?;
            // If we're empty, return nothing
            if value == u32::MAX {
                return Ok("None".into());
            }

            // call inner.toString
            super::resolve_and_invoke(thread, value, "toString", &to_string_descriptor, verbose)?;
            thread.stackframe.locals[0] = value;
            super::run_until_return(thread, verbose)?;

            // Format the string result
            StringObj::inspect(
                &thread.heap,
                thread.stackframe.operand_stack.pop().unwrap() as usize,
                |s| format!("Optional[{s}]").into(),
            )
        });
    optional.register_methods(
        [
            opt_empty,
            opt_of,
            opt_of_nullable,
            is_empty,
            is_present,
            opt_map,
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

    class_area.extend([optional, consumer]);
}
