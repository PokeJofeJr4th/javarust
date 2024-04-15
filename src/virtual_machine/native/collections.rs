use std::sync::Arc;

use crate::{
    access,
    class::{
        code::{
            native_property, NativeDoubleMethod, NativeSingleMethod, NativeStringMethod, NativeVoid,
        },
        Field,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    field, method,
    virtual_machine::{
        native::function::Optional,
        object::{
            AnyObj, ArrayListObj, HashMapObj, HashSetObj, Object, ObjectFinder, StringBuilder,
            StringObj,
        },
        Thread,
    },
};

#[allow(clippy::too_many_lines)]
pub fn add_native_collections(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
    let mut hash_map = RawClass::new(
        access!(public native),
        "java/util/HashMap".into(),
        java_lang_object.clone(),
    );

    let hash_map_init = HashMapObj::default_init();
    let hash_map_size = RawMethod {
        access_flags: access!(public native),
        name: "size".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(native_property(
            HashMapObj::SELF,
            |map| map.len() as u32,
        ))),
        ..Default::default()
    };
    let hash_map_put = RawMethod {
        access_flags: access!(public native),
        name: "put".into(),
        descriptor: method!(((Object(java_lang_object.clone())), (Object(java_lang_object.clone())))->void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, [this, key, value]: [u32; 3], verbose: bool| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) = AnyObj.inspect(&thread.heap, key as usize, |obj| {
                        obj.resolve_method(
                            &thread.method_area,
                            &thread.class_area,
                            "getHashCode",
                            &method!(() -> int),
                            verbose,
                        )
                    })?;
                    thread.stackframe.operand_stack.push(1);
                    thread.invoke_method(method, class);
                    thread.stackframe.locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                    // add the object to the hash map
                    HashMapObj::inspect(&thread.heap, this as usize, |map| {
                        map.insert(hash_code, value)
                    })
                    .map(|_| Some(()))
                }
            },
        )),
        ..Default::default()
    };
    let hash_map_get = RawMethod {
        access_flags: access!(public native),
        name: "get".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, key]: [u32; 2], verbose| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) = AnyObj.inspect(&thread.heap, key as usize, |obj| {
                        obj.resolve_method(
                            &thread.method_area,
                            &thread.class_area,
                            "getHashCode",
                            &method!(() -> int),
                            verbose,
                        )
                    })?;
                    thread.stackframe.operand_stack.push(1);
                    thread.invoke_method(method, class);
                    thread.stackframe.locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                    // add the object to the hash map
                    HashMapObj::inspect(&thread.heap, this as usize, |map| {
                        map.get(&hash_code).copied()
                    })
                    .map(|opt| opt.unwrap_or(NULL))
                    .map(Option::Some)
                }
            },
        )),
        ..Default::default()
    };
    hash_map.register_methods(
        [hash_map_init, hash_map_put, hash_map_get, hash_map_size],
        method_area,
    );

    let mut hash_set = RawClass::new(
        access!(public native),
        "java/util/HashSet".into(),
        java_lang_object.clone(),
    );

    let hash_set_init = HashSetObj::default_init();
    let hash_set_size = RawMethod {
        access_flags: access!(public native),
        name: "size".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(native_property(
            HashSetObj::SELF,
            |set| set.len() as u32,
        ))),
        ..Default::default()
    };
    let hash_set_insert = RawMethod {
        access_flags: access!(public native),
        name: "insert".into(),
        descriptor: method!(((Object(java_lang_object.clone())))->void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, [this, key]: [u32; 2], verbose: bool| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) = AnyObj.inspect(&thread.heap, key as usize, |obj| {
                        obj.resolve_method(
                            &thread.method_area,
                            &thread.class_area,
                            "getHashCode",
                            &method!(() -> int),
                            verbose,
                        )
                    })?;
                    thread.stackframe.operand_stack.push(1);
                    thread.invoke_method(method, class);
                    thread.stackframe.locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                    // add the object to the hash map
                    HashSetObj::inspect(&thread.heap, this as usize, |set| {
                        set.insert(hash_code);
                    })
                    .map(Option::Some)
                }
            },
        )),
        ..Default::default()
    };
    let hash_set_contains = RawMethod {
        access_flags: access!(public native),
        name: "contains".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> boolean),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, key]: [u32; 2], verbose| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) = AnyObj.inspect(&thread.heap, key as usize, |obj| {
                        obj.resolve_method(
                            &thread.method_area,
                            &thread.class_area,
                            "getHashCode",
                            &method!(() -> int),
                            verbose,
                        )
                    })?;
                    thread.stackframe.operand_stack.push(1);
                    thread.invoke_method(method, class);
                    thread.stackframe.locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                    // check if the set contains the element
                    HashSetObj::inspect(&thread.heap, this as usize, |map| {
                        u32::from(map.contains(&hash_code))
                    })
                    .map(Option::Some)
                }
            },
        )),
        ..Default::default()
    };

    hash_set.register_methods(
        [
            hash_set_init,
            hash_set_contains,
            hash_set_insert,
            hash_set_size,
        ],
        method_area,
    );

    let mut array_list = RawClass::new(
        access!(public native),
        "java/util/ArrayList".into(),
        java_lang_object.clone(),
    );
    let arrlist_init = ArrayListObj::default_init();
    let arrlist_append = RawMethod {
        access_flags: access!(public native),
        name: "append".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, [this, ptr]: [u32; 2], _| {
                ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| {
                    arrlist.push(ptr);
                })
                .map(Option::Some)
            },
        )),
        ..Default::default()
    };
    let arrlist_add = RawMethod {
        access_flags: access!(public native),
        name: "add".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> boolean),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, ptr]: [u32; 2], _| {
                ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| {
                    arrlist.push(ptr);
                })
                .map(|()| Some(1))
            },
        )),
        ..Default::default()
    };
    let arrlist_size = RawMethod {
        access_flags: access!(public native),
        name: "size".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(native_property(
            ArrayListObj::SELF,
            |arrls| arrls.len() as u32,
        ))),
        ..Default::default()
    };
    let arrlist_sort = RawMethod {
        access_flags: access!(public native),
        name: "sort".into(),
        descriptor: method!(((Object("java/util/Comparator".into()))) -> void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread,
             [this, cmp, partition, length, target_ptr, index]: [u32; 6],
             verbose: bool| {
                if verbose {
                    println!("Partition: {partition}, Length: {length}, Index: {index}");
                }
                let pc = thread.pc_register;
                thread.pc_register += 1;
                match pc {
                    0 => {
                        thread.stackframe.locals[2] = 1;
                        let length =
                            ArrayListObj::inspect(&thread.heap, this as usize, |v| v.len())? as u32;
                        thread.stackframe.locals[3] = length;
                        Ok(None)
                    }
                    1 => {
                        let target_ptr =
                            ArrayListObj::inspect(&thread.heap, this as usize, |vec| {
                                vec.get(partition as usize).copied().unwrap()
                            })?;
                        thread.stackframe.locals[4] = target_ptr;
                        thread.stackframe.locals[5] = partition;
                        Ok(None)
                    }
                    2 => {
                        let next_ptr = ArrayListObj::inspect(&thread.heap, this as usize, |vec| {
                            vec.get(index as usize - 1).copied().unwrap()
                        })?;
                        let (resolved_class, resolved_method) =
                            AnyObj.inspect(&thread.heap, cmp as usize, |obj| {
                                obj.resolve_method(
                                    &thread.method_area,
                                    &thread.class_area,
                                    "compare",
                                    &method!(((Object("java/lang/Object".into())), (Object("java/lang/Object".into()))) -> int),
                                    verbose,
                                )
                            })?;
                        thread.stackframe.operand_stack.push(3);
                        thread.invoke_method(resolved_method, resolved_class);
                        thread.stackframe.locals[0] = cmp;
                        thread.stackframe.locals[1] = target_ptr;
                        thread.stackframe.locals[2] = next_ptr;
                        Ok(None)
                    }
                    3 => {
                        let cmp = thread.stackframe.operand_stack.pop().unwrap() as i32;
                        if cmp >= 0 {
                            thread.pc_register = 4;
                        } else {
                            // shift the value
                            ArrayListObj::inspect(&thread.heap, this as usize, |vec| {
                                vec[index as usize] = vec[index as usize - 1];
                            })?;
                            // start the next loop if it's not over
                            if partition > 0 {
                                // start the next loop
                                thread.pc_register = 4;
                                thread.stackframe.locals[5] -= 1;
                            }
                        }
                        Ok(None)
                    }
                    4 => {
                        // exit the loop and simulate the end
                        ArrayListObj::inspect(&thread.heap, this as usize, |vec| {
                            vec[index as usize] = target_ptr;
                        })?;
                        // start the next outer loop or exit the function
                        if partition + 1 >= length {
                            // exit the function
                            Ok(Some(()))
                        } else {
                            // start the next outer loop
                            thread.pc_register = 1;
                            // increment partition
                            thread.stackframe.locals[2] += 1;
                            Ok(None)
                        }
                    }
                    _ => Err("Impossible pc reached".to_string().into()),
                }
                /*
                def insertion_sort_wo_swap(a_list):
                    # 0
                    for partition in range(1, len(a_list)):
                        # 1
                        target = a_list[partition]
                        for index in range(partition, -1, -1):
                            # 2
                            if target >= a_list[index - 1]:
                            # 3
                                break
                            a_list[index] = a_list[index - 1]
                        # 4
                        a_list[index] = target
                */
            },
        )),
        ..Default::default()
    };
    let java_lang_string = java_lang_string.clone();
    let arrlist_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(
            move |thread: &mut Thread, [this, builder, index, length]: [u32; 4], verbose: bool| {
                let pc = thread.pc_register;
                thread.pc_register += 1;
                match pc {
                    0 => {
                        let builder = StringBuilder::new("[".to_string(), &thread.class_area);
                        let builder_ref = thread.heap.lock().unwrap().allocate(builder);
                        thread.rember_temp(builder_ref, verbose);
                        let length = ArrayListObj::inspect(&thread.heap, this as usize, |vec| {
                            vec.len() as u32
                        })?;
                        thread.stackframe.locals[1] = builder_ref;
                        thread.stackframe.locals[2] = 0;
                        thread.stackframe.locals[3] = length;
                        Ok(None)
                    }
                    1 => {
                        let next_obj = ArrayListObj::inspect(&thread.heap, this as usize, |vec| {
                            vec[index as usize]
                        })?;
                        let (resolved_class, resolved_method) =
                            AnyObj.inspect(&thread.heap, next_obj as usize, |obj| {
                                obj.resolve_method(
                                    &thread.method_area,
                                    &thread.class_area,
                                    "toString",
                                    &method!(() -> Object(java_lang_string.clone())),
                                    verbose,
                                )
                            })?;
                        thread.stackframe.operand_stack.push(2);
                        thread.invoke_method(resolved_method, resolved_class);
                        thread.stackframe.locals[0] = next_obj;
                        Ok(None)
                    }
                    2 => {
                        let str_ptr = thread.stackframe.operand_stack.pop().unwrap();
                        let string =
                            StringObj::inspect(&thread.heap, str_ptr as usize, |arc| arc.clone())?;
                        StringBuilder::inspect(&thread.heap, builder as usize, |str| {
                            if index == 0 {
                                str.push_str(&string);
                            } else {
                                str.push_str(&format!(", {string}"));
                            }
                        })?;
                        thread.stackframe.locals[2] += 1;
                        if thread.stackframe.locals[2] >= length {
                            let str = StringBuilder::inspect(
                                &thread.heap,
                                builder as usize,
                                |builder| {
                                    builder.push(']');
                                    Arc::<str>::from(&**builder)
                                },
                            )?;
                            Ok(Some(str))
                        } else {
                            thread.pc_register = 1;
                            Ok(None)
                        }
                    }
                    _ => Err("Impossible PC reached".to_string().into()),
                }
            },
        )),
        ..Default::default()
    };

    let mut array_stream = RawClass::new(
        access!(public native),
        "java/util/ArrayList$Stream".into(),
        java_lang_object.clone(),
    );
    array_stream.fields.extend([
        (
            Field {
                name: "array".into(),
                access_flags: access!(private),
                descriptor: field!(Object(array_list.this.clone())),
                ..Default::default()
            },
            0,
        ),
        (
            Field {
                name: "index".into(),
                access_flags: access!(private),
                descriptor: field!(int),
                ..Default::default()
            },
            1,
        ),
        (
            Field {
                name: "end_index".into(),
                access_flags: access!(private),
                descriptor: field!(int),
                ..Default::default()
            },
            2,
        ),
    ]);
    array_stream.field_size += 3;
    let arr_stream_next = RawMethod {
        name: "$next".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> Object(java_lang_object.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this]: [u32; 1], verbose| {
                let next_item = AnyObj.inspect(&thread.heap, this as usize, |o| {
                    let [array_ref, index, end_index] = &mut o.fields[0..3] else {
                        return Ok(None);
                    };
                    if *index >= *end_index {
                        return Ok(None);
                    }
                    let next_index = *index;
                    *index += 1;
                    ArrayListObj::inspect(&thread.heap, *array_ref as usize, |arr| {
                        arr[next_index as usize]
                    })
                    .map(Option::Some)
                })??;
                let opt_idx = Optional::make(thread, next_item.unwrap_or(u32::MAX), verbose);
                Ok(Some(opt_idx))
            },
        )),
        ..Default::default()
    };
    let arr_stream_count = RawMethod {
        name: "count".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> long),
        code: RawCode::native(NativeDoubleMethod(
            |thread: &mut Thread, [this]: [u32; 1], _verbose| {
                AnyObj.inspect(&thread.heap, this as usize, |o| {
                    let difference = o.fields[2] - o.fields[1];
                    o.fields[1] = o.fields[2];
                    Some(difference as u64)
                })
            },
        )),
        ..Default::default()
    };
    array_stream
        .interfaces
        .push("java/util/stream/Stream".into());
    array_stream.register_methods([arr_stream_next, arr_stream_count], method_area);

    let arrlist_stream = RawMethod {
        name: "stream".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> Object("java/util/stream/Stream".into())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this]: [u32; 1], _verbose| {
                let array_stream = thread
                    .class_area
                    .search("java/util/ArrayList$Stream")
                    .unwrap();
                let mut stream = Object::from_class(&array_stream);
                stream.fields[0] = this;
                stream.fields[2] =
                    ArrayListObj::inspect(&thread.heap, this as usize, |arrls| arrls.len() as u32)?;
                let stream_idx = thread.heap.lock().unwrap().allocate(stream);
                Ok(Some(stream_idx))
            },
        )),
        ..Default::default()
    };
    array_list.register_methods(
        [
            arrlist_init,
            arrlist_append,
            arrlist_size,
            arrlist_add,
            arrlist_sort,
            arrlist_to_string,
            arrlist_stream,
        ],
        method_area,
    );

    let mut comparator = RawClass::new(
        access!(public abstract native),
        "java/util/Comparator".into(),
        java_lang_object.clone(),
    );
    let compare = RawMethod {
        name: "compare".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object(java_lang_object.clone())), (Object(java_lang_object.clone()))) -> boolean),
        code: RawCode::Abstract,
        ..Default::default()
    };

    comparator.register_method(compare, method_area);

    class_area.extend([hash_map, hash_set, array_list, comparator, array_stream]);
}
