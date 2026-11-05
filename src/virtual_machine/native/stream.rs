use std::sync::Arc;

use jvmrs_lib::{access, method};

use crate::{
    class::code::{NativeSingleMethod, NativeVoid},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
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
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| {
                    loop {
                        // execute .$next()
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = this;
                        super::run_until_return(thread, verbose)?;
                        // If we get nothing, return true
                        let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                        else {
                            println!("Stream is over, goobbye");
                            return Ok(1);
                        };
                        // execute .test()
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = ret;
                        super::run_until_return(thread, verbose)?;
                        // If test returned false, we return false
                        let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                        if predicate_ret == 0 {
                            return Ok(0);
                        }
                    }
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
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| {
                    loop {
                        // execute .$next()
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = this;
                        super::run_until_return(thread, verbose)?;
                        // If we get nothing, return false
                        let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                        else {
                            return Ok(0);
                        };
                        // execute .test()
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = ret;
                        super::run_until_return(thread, verbose)?;
                        // If test returned true, we return true
                        let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                        if predicate_ret > 0 {
                            return Ok(1);
                        }
                    }
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
    // An empty stream always returns None, so we replace the $next function with a call to Optional.empty()
    let empty_stream = RawMethod {
        name: "empty".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> Object(stream.this.clone())),
        code: RawCode::native(make_lambda_override::<0>(
            stream_next.name.clone(),
            stream_next.descriptor.clone(),
            stream.this.clone(),
            Arc::from("empty"),
            method!(() -> Object("java/util/Optional".into())),
            Arc::from("java/util/Optional"),
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
                move |thread: &mut Thread, [stream, predicate]: [u32; 2], verbose| {
                    loop {
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(stream, "$next", &next_descriptor, verbose)?;
                        thread.stackframe.locals[0] = stream;
                        super::run_until_return(thread, verbose)?;
                        let returned_opt = thread.stackframe.operand_stack.pop().unwrap();
                        let Some(returned_obj) =
                            Optional.inspect(&thread.heap, returned_opt as usize, |o| *o)?
                        else {
                            if verbose {
                                println!("Stream end - returning None");
                            }
                            return Ok(returned_opt);
                        };
                        thread.stackframe.operand_stack.push(returned_opt);
                        thread.stackframe.operand_stack.push(0);
                        thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                        thread.stackframe.locals[0] = predicate;
                        thread.stackframe.locals[1] = returned_obj;
                        super::run_until_return(thread, verbose)?;
                        let test_result = thread.stackframe.operand_stack.pop().unwrap();
                        if test_result > 0 {
                            break;
                        }
                    }
                    if verbose {
                        println!("Predicate succeeded - returning the tested object");
                    }
                    Ok(thread.stackframe.operand_stack.pop().unwrap())
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
                stream_next.name.clone(),
                stream_next.descriptor.clone(),
                stream.this.clone(),
                filter_lambda.name.clone(),
                filter_lambda.descriptor.clone(),
                stream.this.clone(),
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
                move |thread: &mut Thread, [this, consumer]: [u32; 2], verbose| {
                    loop {
                        thread.stackframe.operand_stack.push(1);
                        thread.resolve_and_invoke(this, "$next", &next_signature, verbose)?;
                        thread.stackframe.locals[0] = this;
                        super::run_until_return(thread, verbose)?;
                        let next_value = thread.stackframe.operand_stack.pop().unwrap();
                        let next_value =
                            Optional.inspect(&thread.heap, next_value as usize, |o| *o)?;
                        let Some(next_value) = next_value else {
                            return Ok(());
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
                        super::run_until_return(thread, verbose)?;
                    }
                },
            )),
            ..Default::default()
        }
    };
    // TODO: forEachOrdered
    let generate_lambda = {
        let get_descriptor = method!(() -> Object(java_lang_object.clone()));
        RawMethod {
            name: "$generate".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object("java/util/function/Supplier".into()))) -> Object(java_lang_object.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [supplier]: [u32; 1], verbose| {
                    // Call supplier.get()
                    thread.stackframe.operand_stack.push(0);
                    thread.resolve_and_invoke(supplier, "get", &get_descriptor, verbose)?;
                    thread.stackframe.locals[0] = supplier;
                    // Return the result from the supplier
                    Ok(thread.stackframe.operand_stack.pop().unwrap())
                },
            )),
            ..Default::default()
        }
    };
    let generate = {
        RawMethod {
            name: "generate".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object("java/util/function/Supplier".into()))) -> Object(stream.this.clone())),
            code: RawCode::native(make_lambda_override::<1>(
                stream_next.name.clone(),
                stream_next.descriptor.clone(),
                stream.this.clone(),
                generate_lambda.name.clone(),
                generate_lambda.descriptor.clone(),
                stream.this.clone(),
            )),
            ..Default::default()
        }
    };
    // TODO: iterate
    // TODO: limit
    // TODO: map <toPrimitive>
    let map_lambda = {
        let next_descriptor = method!(() -> Object(java_lang_object.clone()));
        let apply_descriptor =
            method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone()));
        RawMethod {
            name: "$map".into(),
            access_flags: access!(public static native),
            descriptor: method!(((Object(stream.this.clone())), (Object("java/util/function/Function".into()))) -> Object(java_lang_object.clone())),
            code: RawCode::native(NativeSingleMethod(
                move |thread: &mut Thread, [this, func]: [u32; 2], verbose| {
                    // Get the next thing from the inner iterator
                    thread.stackframe.operand_stack.push(0);
                    thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                    thread.stackframe.locals[0] = this;
                    super::run_until_return(thread, verbose)?;

                    // If it's empty, return empty
                    let next_idx = thread.stackframe.operand_stack.pop().unwrap();
                    let Some(next_obj) =
                        Optional.inspect(&thread.heap, next_idx as usize, |o| *o)?
                    else {
                        return Ok(next_idx);
                    };

                    // Call func.apply(next)
                    thread.stackframe.operand_stack.push(0);
                    thread.resolve_and_invoke(func, "apply", &apply_descriptor, verbose)?;
                    thread.stackframe.locals[0] = func;
                    thread.stackframe.locals[1] = next_obj;
                    super::run_until_return(thread, verbose)?;

                    // Make a new optional for the function return
                    let func_ret = thread.stackframe.operand_stack.pop().unwrap();
                    let opt_ret = Optional::make(thread, func_ret, verbose);
                    Ok(opt_ret)
                },
            )),
            ..Default::default()
        }
    };
    let map = RawMethod {
        name: "map".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object("java/util/function/Function".into()))) -> Object(stream.this.clone())),
        code: RawCode::native(make_lambda_override::<1>(
            stream_next.name.clone(),
            stream_next.descriptor.clone(),
            stream.this.clone(),
            map_lambda.name.clone(),
            map_lambda.descriptor.clone(),
            stream.this.clone(),
        )),
        ..Default::default()
    };
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
                move |thread: &mut Thread, [this, predicate]: [u32; 2], verbose| loop {
                    thread.stackframe.operand_stack.push(1);
                    thread.resolve_and_invoke(this, "$next", &next_descriptor, verbose)?;
                    thread.stackframe.locals[0] = this;
                    super::run_until_return(thread, verbose)?;
                    let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                    let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                    else {
                        return Ok(1);
                    };
                    thread.stackframe.operand_stack.push(2);
                    thread.resolve_and_invoke(predicate, "test", &test_descriptor, verbose)?;
                    thread.stackframe.locals[0] = predicate;
                    thread.stackframe.locals[1] = ret;
                    super::run_until_return(thread, verbose)?;
                    let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                    if predicate_ret != 0 {
                        return Ok(0);
                    }
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
            none_match,
            map,
            map_lambda,
            generate,
            generate_lambda,
        ],
        method_area,
    );

    class_area.extend([stream]);
}
