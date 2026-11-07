use std::sync::Arc;

use jvmrs_lib::method;

use crate::{
    data::{WorkingClassArea, WorkingMethodArea},
    native_class,
    virtual_machine::{native::field::Boolean, object::ObjectFinder},
};

use super::function::Optional;

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
) {
    let accept_descriptor = method!(((Object(java_lang_object.clone()))) -> void);
    let apply_descriptor =
        method!(((Object(java_lang_object.clone()))) -> Object("java/lang/Object".into()));
    class_area.extend(native_class! {
        use method_area;
        impl "java/util/stream/Stream" {
            fn "$next"(&_this) -> java::lang::Object;

            fn allMatch(&this, predicate: java::util::function::Predicate) -> Boolean {
                use thread, verbose;
                loop {
                    // execute .$next()
                    super::resolve_and_invoke(
                        thread,
                        this,
                        "$next",
                        &method!(() -> Object("java/lang/Object".into())),
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = this;
                    super::run_until_return(thread, verbose)?;
                    // If we get nothing, return true
                    let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                    let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                    else {
                        if verbose {
                            println!("Stream is over, goobbye");
                        }
                        return Ok(true);
                    };
                    // execute .test()
                    super::resolve_and_invoke(
                        thread,
                        predicate,
                        "test",
                        &method!(((Object("java/lang/Object".into()))) -> boolean),
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = predicate;
                    thread.stackframe.locals[1] = ret;
                    super::run_until_return(thread, verbose)?;
                    // If test returned false, we return false
                    let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                    if predicate_ret == 0 {
                        return Ok(false);
                    }
                }
            }

            fn anyMatch(&this, predicate: java::util::function::Predicate) -> Boolean {
                use thread, verbose;
                loop {
                    // execute .$next()
                    super::resolve_and_invoke(
                        thread,
                        this,
                        "$next",
                        &method!(() -> Object("java/lang/Object".into())),
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = this;
                    super::run_until_return(thread, verbose)?;
                    // If we get nothing, return false
                    let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                    let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                    else {
                        return Ok(false);
                    };
                    // execute .test()
                    super::resolve_and_invoke(
                        thread,
                        predicate,
                        "test",
                        &method!(((Object("java/lang/Object".into()))) -> boolean),
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = predicate;
                    thread.stackframe.locals[1] = ret;
                    super::run_until_return(thread, verbose)?;
                    // If test returned true, we return true
                    let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                    if predicate_ret > 0 {
                        return Ok(true);
                    }
                }
            }

            fn noneMatch(&this, predicate: java::util::function::Predicate) -> Boolean {
                use thread, verbose;
                loop {
                    super::resolve_and_invoke(thread, this, "$next", &method!(() -> Object("java/lang/Object".into())), verbose)?;
                    thread.stackframe.locals[0] = this;
                    super::run_until_return(thread, verbose)?;
                    let ret_opt = thread.stackframe.operand_stack.pop().unwrap();
                    let Some(ret) = Optional.inspect(&thread.heap, ret_opt as usize, |o| *o)?
                    else {
                        return Ok(true);
                    };
                    super::resolve_and_invoke(
                        thread,
                        predicate,
                        "test",
                        &method!(((Object("java/lang/Object".into()))) -> boolean),
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = predicate;
                    thread.stackframe.locals[1] = ret;
                    super::run_until_return(thread, verbose)?;
                    let predicate_ret = thread.stackframe.operand_stack.pop().unwrap();
                    if predicate_ret != 0 {
                        return Ok(false);
                    }
                }
            }

            fn empty() -> java::util::stream::Stream {
                use thread, _verbose;
                let invoke = jvmrs_lib::MethodHandle::InvokeStatic {
                    class: Arc::from("java/util/Optional"),
                    name: Arc::from("empty"),
                    method_type: method!(()->Object("java/util/Optional".into())),
                };
                let class = thread.class_area.search("java/util/stream/Stream").unwrap();
                let lambda_object = crate::virtual_machine::object::LambdaOverride {
                    method_name: "$next".into(),
                    method_descriptor: method!(() -> Object("java/lang/Object".into())),
                    invoke,
                    captures: Vec::new(),
                }.as_object(class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn "$filter"(stream: java::util::stream::Stream, predicate: java::util::function::Predicate) -> java::util::Optional {
                use thread, verbose;
                loop {
                    super::resolve_and_invoke(thread, stream, "$next", &method!(() -> Object("java/lang/Object".into())), verbose)?;
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
                    super::resolve_and_invoke(
                        thread,
                        predicate,
                        "test",
                        &method!(((Object("java/lang/object".into()))) -> boolean),
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = predicate;
                    thread.stackframe.locals[1] = returned_obj;
                    super::run_until_return(thread, verbose)?;
                    let test_result = thread.stackframe.operand_stack.pop().unwrap();
                    if test_result > 0 {
                        return Ok(returned_opt);
                    }
                }
            }

            fn filter(&this, predicate: java::util::function::Predicate) -> java::util::stream::Stream {
                use thread, _verbose;
                let invoke = jvmrs_lib::MethodHandle::InvokeStatic {
                    class: Arc::from("java/util/stream/Stream"),
                    name: Arc::from("$filter"),
                    method_type: method!(((Object("java/util/stream/Stream".into())),(Object("java/util/function/Predicate".into())))->Object("java/util/stream/Stream".into())),
                };
                let class = thread.class_area.search("java/util/stream/Stream").unwrap();
                let lambda_object = crate::virtual_machine::object::LambdaOverride {
                    method_name: "$next".into(),
                    method_descriptor: method!(() -> Object("java/lang/Object".into())),
                    invoke,
                    captures: vec![this, predicate],
                }.as_object(class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn forEach(&this, consumer: java::util::function::Consumer) {
                use thread, verbose;
                loop {
                    super::resolve_and_invoke(thread, this, "$next", &method!(() -> Object("java/lang/Object".into())), verbose)?;
                    thread.stackframe.locals[0] = this;
                    super::run_until_return(thread, verbose)?;
                    let next_value = thread.stackframe.operand_stack.pop().unwrap();
                    let next_value =
                        Optional.inspect(&thread.heap, next_value as usize, |o| *o)?;
                    let Some(next_value) = next_value else {
                        return Ok(());
                    };
                    // invoke the consumer
                    super::resolve_and_invoke(
                        thread,
                        consumer,
                        "accept",
                        &accept_descriptor,
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = consumer;
                    thread.stackframe.locals[1] = next_value;
                    super::run_until_return(thread, verbose)?;
                }
            }

            fn "$generate"(supplier: java::util::function::Supplier) -> java::lang::Object {
                use thread, verbose;
                super::resolve_and_invoke(thread, supplier, "get", &method!(() -> Object("java/lang/Object".into())), verbose)?;
                thread.stackframe.locals[0] = supplier;
                // Return the result from the supplier
                Ok(thread.stackframe.operand_stack.pop().unwrap())
            }

            fn generate(supplier: java::util::function::Supplier) -> java::util::stream::Stream {
                use thread, _verbose;
                let invoke = jvmrs_lib::MethodHandle::InvokeStatic {
                    class: Arc::from("java/util/stream/Stream"),
                    name: Arc::from("$generate"),
                    method_type: method!(((Object("java/util/function/Supplier".into())))->Object("java/lang/Object".into())),
                };
                let class = thread.class_area.search("java/util/stream/Stream").unwrap();
                let lambda_object = crate::virtual_machine::object::LambdaOverride {
                    method_name: "$next".into(),
                    method_descriptor: method!(() -> Object("java/lang/Object".into())),
                    invoke,
                    captures: vec![supplier],
                }.as_object(class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }

            fn "$map"(this: java::util::stream::Stream, func: java::util::function::Function) -> java::lang::Object {
                use thread, verbose;
                // Get the next thing from the inner iterator
                super::resolve_and_invoke(thread, this, "$next", &method!(() -> Object("java/lang/Object".into())), verbose)?;
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
                super::resolve_and_invoke(thread, func, "apply", &apply_descriptor, verbose)?;
                thread.stackframe.locals[0] = func;
                thread.stackframe.locals[1] = next_obj;
                super::run_until_return(thread, verbose)?;

                // Make a new optional for the function return
                let func_ret = thread.stackframe.operand_stack.pop().unwrap();
                let opt_ret = Optional::make(thread, func_ret, verbose);
                Ok(opt_ret)
            }

            fn map(&this, function: java::util::function::Function) -> java::util::stream::Stream {
                use thread, _verbose;
                let invoke = jvmrs_lib::MethodHandle::InvokeStatic {
                    class: Arc::from("java/util/stream/Stream"),
                    name: Arc::from("$map"),
                    method_type: method!(((Object("java/util/stream/Stream".into())),(Object("java/util/function/Function".into())))->Object("java/lang/Object".into())),
                };
                let class = thread.class_area.search("java/util/stream/Stream").unwrap();
                let lambda_object = crate::virtual_machine::object::LambdaOverride {
                    method_name: "$next".into(),
                    method_descriptor: method!(() -> Object("java/lang/Object".into())),
                    invoke,
                    captures: vec![this, function],
                }.as_object(class);
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                Ok(idx)
            }
        }
    });
    // TODO: collect
    // TODO: concat
    // TODO: count
    // TODO: distinct
    // TODO: dropWhile
    // TODO: findAny
    // TODO: findFirst
    // TODO: flatMap <toPrimitive>
    // TODO: forEachOrdered
    // TODO: iterate
    // TODO: limit
    // TODO: map <toPrimitive>
    // TODO: mapMulti <toPrimitive>
    // TODO: max
    // TODO: min
    // TODO: of
    // TODO: ofNullable
    // TODO: peek
    // TODO: reduce
    // TODO: skip
    // TODO: sorted
    // TODO: takeWhile
    // TODO: toArray
    // TODO: toList
}
