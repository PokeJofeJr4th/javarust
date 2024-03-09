use std::sync::{Arc, Mutex};

use crate::{
    access,
    class::code::{native_property, NativeSingleMethod, NativeVoid},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    method,
    virtual_machine::{
        object::{AnyObj, ArrayListObj, HashMapObj, HashSetObj, ObjectFinder},
        StackFrame, Thread,
    },
};

#[allow(clippy::too_many_lines)]
pub fn add_native_collections(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
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
                    HashMapObj::SELF
                        .get_mut(&mut thread.heap.lock().unwrap(), this as usize, |map| {
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
                    HashMapObj::SELF
                        .get(&thread.heap.lock().unwrap(), this as usize, |map| {
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
                    HashSetObj::SELF
                        .get_mut(&mut thread.heap.lock().unwrap(), this as usize, |set| {
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
                    HashSetObj::SELF
                        .get(&thread.heap.lock().unwrap(), this as usize, |map| {
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
                ArrayListObj::SELF
                    .get_mut(&mut thread.heap.lock().unwrap(), this as usize, |arrlist| {
                        arrlist.push(ptr);
                    })
                    .map(Option::Some)
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
    array_list.methods.extend([
        arrlist_init.name(array_list.this.clone()),
        arrlist_append.name(array_list.this.clone()),
        arrlist_size.name(array_list.this.clone()),
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
        (array_list.this.clone(), arrlist_size),
    ]);
    class_area.extend([hash_map, hash_set, array_list]);
}
