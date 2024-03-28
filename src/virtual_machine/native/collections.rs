use std::sync::{Arc, Mutex};

use crate::{
    access,
    class::code::{native_property, NativeSingleMethod, NativeStringMethod, NativeVoid},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    method,
    virtual_machine::{
        object::{
            AnyObj, ArrayListObj, HashMapObj, HashSetObj, ObjectFinder, StringBuilder, StringObj,
        },
        StackFrame, Thread,
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
            |thread: &mut Thread,
             stackframe: &Mutex<StackFrame>,
             [this, key, value]: [u32; 3],
             verbose: bool| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) =
                        AnyObj.get(&thread.heap.lock().unwrap(), key as usize, |obj| {
                            obj.resolve_method(
                                &thread.method_area,
                                &thread.class_area,
                                "getHashCode",
                                &method!(() -> int),
                                verbose,
                            )
                        })?;
                    thread.invoke_method(method, class);
                    thread.stack.last().unwrap().lock().unwrap().locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                    // add the object to the hash map
                    HashMapObj::get_mut(&mut thread.heap.lock().unwrap(), this as usize, |map| {
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
            |thread: &mut Thread,
             stackframe: &Mutex<StackFrame>,
             [this, key]: [u32; 2],
             verbose| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) =
                        AnyObj.get(&thread.heap.lock().unwrap(), key as usize, |obj| {
                            obj.resolve_method(
                                &thread.method_area,
                                &thread.class_area,
                                "getHashCode",
                                &method!(() -> int),
                                verbose,
                            )
                        })?;
                    thread.invoke_method(method, class);
                    thread.stack.last().unwrap().lock().unwrap().locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                    // add the object to the hash map
                    HashMapObj::get(&thread.heap.lock().unwrap(), this as usize, |map| {
                        map.get(&hash_code).copied()
                    })
                    .map(|opt| opt.unwrap_or(NULL))
                    .map(Option::Some)
                }
            },
        )),
        ..Default::default()
    };
    hash_map.methods.extend([
        hash_map_init.name(hash_map.this.clone()),
        hash_map_put.name(hash_map.this.clone()),
        hash_map_get.name(hash_map.this.clone()),
        hash_map_size.name(hash_map.this.clone()),
    ]);

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
            |thread: &mut Thread,
             stackframe: &Mutex<StackFrame>,
             [this, key]: [u32; 2],
             verbose: bool| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) =
                        AnyObj.get(&thread.heap.lock().unwrap(), key as usize, |obj| {
                            obj.resolve_method(
                                &thread.method_area,
                                &thread.class_area,
                                "getHashCode",
                                &method!(() -> int),
                                verbose,
                            )
                        })?;
                    thread.invoke_method(method, class);
                    thread.stack.last().unwrap().lock().unwrap().locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                    // add the object to the hash map
                    HashSetObj::get_mut(&mut thread.heap.lock().unwrap(), this as usize, |set| {
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
            |thread: &mut Thread,
             stackframe: &Mutex<StackFrame>,
             [this, key]: [u32; 2],
             verbose| {
                if thread.pc_register == 0 {
                    // get the hash code for the object
                    let (class, method) =
                        AnyObj.get(&thread.heap.lock().unwrap(), key as usize, |obj| {
                            obj.resolve_method(
                                &thread.method_area,
                                &thread.class_area,
                                "getHashCode",
                                &method!(() -> int),
                                verbose,
                            )
                        })?;
                    thread.invoke_method(method, class);
                    thread.stack.last().unwrap().lock().unwrap().locals[0] = key;
                    Ok(None)
                } else {
                    let hash_code = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                    // check if the set contains the element
                    HashSetObj::get(&thread.heap.lock().unwrap(), this as usize, |map| {
                        u32::from(map.contains(&hash_code))
                    })
                    .map(Option::Some)
                }
            },
        )),
        ..Default::default()
    };

    hash_set.methods.extend([
        hash_set_init.name(hash_set.this.clone()),
        hash_set_contains.name(hash_set.this.clone()),
        hash_set_insert.name(hash_set.this.clone()),
        hash_set_size.name(hash_set.this.clone()),
    ]);

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
            |thread: &mut Thread, _: &_, [this, ptr]: [u32; 2], _| {
                ArrayListObj::get_mut(&mut thread.heap.lock().unwrap(), this as usize, |arrlist| {
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
            |thread: &mut Thread, _: &_, [this, ptr]: [u32; 2], _| {
                ArrayListObj::get_mut(&mut thread.heap.lock().unwrap(), this as usize, |arrlist| {
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
             stackframe: &Mutex<StackFrame>,
             [this, cmp, partition, length, target_ptr, index]: [u32; 6],
             verbose: bool| {
                if verbose {
                    println!("Partition: {partition}, Length: {length}, Index: {index}");
                }
                let pc = thread.pc_register;
                thread.pc_register += 1;
                match pc {
                    0 => {
                        stackframe.lock().unwrap().locals[2] = 1;
                        let length =
                            ArrayListObj::get(&thread.heap.lock().unwrap(), this as usize, |vec| {
                                vec.len()
                            })? as u32;
                        stackframe.lock().unwrap().locals[3] = length;
                        Ok(None)
                    }
                    1 => {
                        let target_ptr = ArrayListObj::get(
                            &thread.heap.lock().unwrap(),
                            this as usize,
                            |vec| vec.get(partition as usize).copied().unwrap(),
                        )?;
                        stackframe.lock().unwrap().locals[4] = target_ptr;
                        stackframe.lock().unwrap().locals[5] = partition;
                        Ok(None)
                    }
                    2 => {
                        let next_ptr = ArrayListObj::get(
                            &thread.heap.lock().unwrap(),
                            this as usize,
                            |vec| vec.get(index as usize - 1).copied().unwrap(),
                        )?;
                        let (resolved_class, resolved_method) =
                            AnyObj.get(&thread.heap.lock().unwrap(), cmp as usize, |obj| {
                                obj.resolve_method(
                                    &thread.method_area,
                                    &thread.class_area,
                                    "compare",
                                    &method!(((Object("java/lang/Object".into())), (Object("java/lang/Object".into()))) -> int),
                                    verbose,
                                )
                            })?;
                        thread.invoke_method(resolved_method, resolved_class);
                        let mut new_stackframe = thread.stack.last().unwrap().lock().unwrap();
                        new_stackframe.locals[0] = cmp;
                        new_stackframe.locals[1] = target_ptr;
                        new_stackframe.locals[2] = next_ptr;
                        drop(new_stackframe);
                        stackframe.lock().unwrap().operand_stack.push(3);
                        Ok(None)
                    }
                    3 => {
                        let cmp = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                        if cmp >= 0 {
                            thread.pc_register = 4;
                        } else {
                            // shift the value
                            ArrayListObj::get_mut(
                                &mut thread.heap.lock().unwrap(),
                                this as usize,
                                |vec| vec[index as usize] = vec[index as usize - 1],
                            )?;
                            // start the next loop if it's not over
                            if partition > 0 {
                                // start the next loop
                                thread.pc_register = 4;
                                stackframe.lock().unwrap().locals[5] -= 1;
                            }
                        }
                        Ok(None)
                    }
                    4 => {
                        // exit the loop and simulate the end
                        ArrayListObj::get_mut(
                            &mut thread.heap.lock().unwrap(),
                            this as usize,
                            |vec| {
                                vec[index as usize] = target_ptr;
                            },
                        )?;
                        // start the next outer loop or exit the function
                        if partition + 1 >= length {
                            // exit the function
                            Ok(Some(()))
                        } else {
                            // start the next outer loop
                            thread.pc_register = 1;
                            // increment partition
                            stackframe.lock().unwrap().locals[2] += 1;
                            Ok(None)
                        }
                    }
                    _ => Err("Impossible pc reached".to_string()),
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
            move |thread: &mut Thread,
                  stackframe: &Mutex<StackFrame>,
                  [this, builder, index, length]: [u32; 4],
                  verbose: bool| {
                let pc = thread.pc_register;
                thread.pc_register += 1;
                match pc {
                    0 => {
                        let builder = StringBuilder::new("[".to_string(), &thread.class_area);
                        let builder_ref = thread.heap.lock().unwrap().allocate(builder);
                        thread.rember_temp(stackframe, builder_ref, verbose);
                        let length = ArrayListObj::get(
                            &thread.heap.lock().unwrap(),
                            this as usize,
                            |vec| vec.len() as u32,
                        )?;
                        let mut stackframe = stackframe.lock().unwrap();
                        stackframe.locals[1] = builder_ref;
                        stackframe.locals[2] = 0;
                        stackframe.locals[3] = length;
                        drop(stackframe);
                        Ok(None)
                    }
                    1 => {
                        let next_obj = ArrayListObj::get(
                            &thread.heap.lock().unwrap(),
                            this as usize,
                            |vec| vec[index as usize],
                        )?;
                        let (resolved_class, resolved_method) =
                            AnyObj.get(&thread.heap.lock().unwrap(), next_obj as usize, |obj| {
                                obj.resolve_method(
                                    &thread.method_area,
                                    &thread.class_area,
                                    "toString",
                                    &method!(() -> Object(java_lang_string.clone())),
                                    verbose,
                                )
                            })?;
                        thread.invoke_method(resolved_method, resolved_class);
                        let mut new_stackframe = thread.stack.last().unwrap().lock().unwrap();
                        new_stackframe.locals[0] = next_obj;
                        drop(new_stackframe);
                        stackframe.lock().unwrap().operand_stack.push(2);
                        Ok(None)
                    }
                    2 => {
                        let str_ptr = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                        let string = StringObj::get(
                            &thread.heap.lock().unwrap(),
                            str_ptr as usize,
                            Clone::clone,
                        )?;
                        StringBuilder::get_mut(
                            &mut thread.heap.lock().unwrap(),
                            builder as usize,
                            |str| {
                                if index == 0 {
                                    str.push_str(&string);
                                } else {
                                    str.push_str(&format!(", {string}"));
                                }
                            },
                        )?;
                        let mut stackframe = stackframe.lock().unwrap();
                        stackframe.locals[2] += 1;
                        if stackframe.locals[2] >= length {
                            let str = StringBuilder::get_mut(
                                &mut thread.heap.lock().unwrap(),
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
                    _ => Err("Impossible PC reached".to_string()),
                }
            },
        )),
        ..Default::default()
    };
    array_list.methods.extend([
        arrlist_init.name(array_list.this.clone()),
        arrlist_append.name(array_list.this.clone()),
        arrlist_size.name(array_list.this.clone()),
        arrlist_add.name(array_list.this.clone()),
        arrlist_sort.name(array_list.this.clone()),
        arrlist_to_string.name(array_list.this.clone()),
    ]);

    method_area.extend([
        (hash_map.this.clone(), hash_map_init),
        (hash_map.this.clone(), hash_map_put),
        (hash_map.this.clone(), hash_map_get),
        (hash_map.this.clone(), hash_map_size),
        (hash_set.this.clone(), hash_set_init),
        (hash_set.this.clone(), hash_set_contains),
        (hash_set.this.clone(), hash_set_insert),
        (hash_set.this.clone(), hash_set_size),
        (array_list.this.clone(), arrlist_init),
        (array_list.this.clone(), arrlist_append),
        (array_list.this.clone(), arrlist_add),
        (array_list.this.clone(), arrlist_size),
        (array_list.this.clone(), arrlist_sort),
        (array_list.this.clone(), arrlist_to_string),
    ]);
    class_area.extend([hash_map, hash_set, array_list]);
}
