use std::sync::Arc;

use crate::{
    access,
    class::code::{NativeSingleMethod, NativeTodo, NativeVoid},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    method,
    virtual_machine::{object::ObjectFinder, Thread},
};

use super::function::{make_lambda_override, Optional};

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
) {
    let mut stream = RawClass::new(
        access!(public abstract native),
        "java/util/stream/Stream".into(),
        java_lang_object.clone(),
    );

    let stream_next = RawMethod {
        name: "$next".into(),
        access_flags: access!(public abstract native),
        descriptor: method!(() -> Object(java_lang_object.clone())),
        code: RawCode::Abstract,
        ..Default::default()
    };
    let all_match = {
        let next_descriptor = stream_next.descriptor.clone();
        let test_descriptor = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "allMatch".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object("java/util/function/Predicate".into()))) -> boolean),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = this;
                        Ok(None)
                    }
                    1 => {
                        let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                        else {
                            return Ok(Some(1));
                        };
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = ret;
                        Ok(None)
                    }
                    2 => {
                        let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                        if predicate_ret == 0 {
                            Ok(Some(0))
                        } else {
                            thread.pc_register = 0;
                            Ok(None)
                        }
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };

    let any_match = {
        let next_descriptor = stream_next.descriptor.clone();
        let test_descriptor = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "anyMatch".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object("java/util/function/Predicate".into()))) -> boolean),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = this;
                        Ok(None)
                    }
                    1 => {
                        let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                        else {
                            return Ok(Some(0));
                        };
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = ret;
                        Ok(None)
                    }
                    2 => {
                        let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                        if predicate_ret == 0 {
                            thread.pc_register = 0;
                            Ok(None)
                        } else {
                            Ok(Some(1))
                        }
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    // TODO: collect
    // TODO: concat
    // TODO: count
    // TODO: distinct
    // TODO: dropWhile
    let empty_stream = RawMethod {
        name: "empty".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> Object(stream.this.clone())),
        code: RawCode::native(make_lambda_override::<0>(
            &stream_next.name,
            &stream_next.descriptor,
            &stream.this,
            &Arc::from("empty"),
            &method!(() -> Object("java/util/Optional".into())),
            &Arc::from("java/util/Optional"),
        )),
        ..Default::default()
    };
    let filter_lambda = {
        let next_descriptor = method!(() -> Object(java_lang_object.clone()));
        let test_descriptor = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "$filter".into(),
            access_flags: access!(private native static),
            descriptor: method!(((Object(stream.this.clone())), (Object("java/util/function/Predicate".into()))) -> Object("java/util/Optional".into())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [stream, predicate]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(stream, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = stream;
                        Ok(None)
                    }
                    1 => {
                        let returned_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(returned_obj) =
                            Optional.inspect(&thread.heap, returned_opt as usize, |o| *o)?
                        else {
                            if verbose {
                                println!("Stream end - returning None");
                            }
                            return Ok(Some(returned_opt));
                        };
                        thread.stackframe.operand_stack.push(returned_opt);
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = returned_obj;
                        Ok(None)
                    }
                    2 => {
                        let test_result = thread.stackframe.operand_stack.pop().unwrap();
                        if test_result == 0 {
                            if verbose {
                                println!("Predicate failed - continuing to next option");
                            }
                            thread.stackframe.operand_stack.pop();
                            thread.pc_register = 0;
                            return Ok(None);
                        }
                        if verbose {
                            println!("Predicate succeeded - returning the tested object");
                        }
                        Ok(Some(thread.stackframe.operand_stack.pop().unwrap()))
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    let filter = {
        RawMethod {
            name: "filter".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object(
                "java/util/function/Predicate".into()
            ))) -> Object(
                stream.this.clone()
            )),
            code: RawCode::native(make_lambda_override::<2>(
                &stream_next.name,
                &stream_next.descriptor,
                &stream.this,
                &filter_lambda.name,
                &filter_lambda.descriptor,
                &stream.this,
            )),
            ..Default::default()
        }
    };
    // TODO: findAny
    // TODO: findFirst
    // TODO: flatMap <toPrimitive>
    let for_each = {
        let next_signature = method!(() -> Object(java_lang_object.clone()));
        let accept_signature = method!(((Object(java_lang_object.clone()))) -> void);
        RawMethod {
            name: "forEach".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object("java/util/function/Consumer".into()))) -> void),
            code: RawCode::native(NativeVoid(
                move |thread: &mut Thread, [this, consumer]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(this, "$next", &next_signature, verbose)?;
                        thread.stackframe.locals[0] = this;
                        Ok(None)
                    }
                    1 => {
                        let next_value = thread.stackframe.operand_stack.pop().unwrap();
                        let next_value =
                            Optional.inspect(&thread.heap, next_value as usize, |o| *o)?;
                        let Some(next_value) = next_value else {
                            return Ok(Some(()));
                        };
                        // invoke the consumer
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(
                            consumer,
                            "accept",
                            &accept_signature,
                            verbose,
                        )?;
                        thread.stackframe.locals[0] = consumer;
                        thread.stackframe.locals[1] = next_value;
                        Ok(None)
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    // TODO: forEachOrdered
    // TODO: generate
    // TODO: iterate
    // TODO: limit
    // TODO: map <toPrimitive>
    // TODO: mapMulti <toPrimitive>
    // TODO: max
    // TODO: min
    let none_match = {
        let next_descriptor = stream_next.descriptor.clone();
        let test_descriptor = method!(((Object(java_lang_object.clone()))) -> boolean);
        RawMethod {
            name: "noneMatch".into(),
            access_flags: access!(public native),
            descriptor: method!(((Object("java/util/function/Predicate".into()))) -> boolean),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| match thread
                    .pc_register
                {
                    0 => {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = this;
                        Ok(None)
                    }
                    1 => {
                        let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                        else {
                            return Ok(Some(1));
                        };
                        thread.stackframe.operand_stack.push(2);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = ret;
                        Ok(None)
                    }
                    2 => {
                        let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                        if predicate_ret == 0 {
                            thread.pc_register = 0;
                            Ok(None)
                        } else {
                            Ok(Some(0))
                        }
                    }
                    _ => unreachable!(),
                },
            )),
            ..Default::default()
        }
    };
    // TODO: of
    // TODO: ofNullable
    // TODO: peek
    // TODO: reduce
    // TODO: skip
    // TODO: sorted
    // TODO: takeWhile
    // TODO: toArray
    // TODO: toList
    stream.register_methods(
        [
            all_match,
            any_match,
            stream_next,
            for_each,
            filter,
            filter_lambda,
            empty_stream,
        ],
        method_area,
    );

    class_area.extend([stream]);
}
