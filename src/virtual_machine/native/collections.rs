use std::collections::HashSet;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Write};

use jvmrs_lib::{access, field, method};

use crate::virtual_machine::native::field::{Boolean, Int, ObjectField, StringField};
use crate::{
    class::{
        code::{NativeDoubleMethod, NativeSingleMethod},
        Field,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    native_class,
    virtual_machine::{
        native::function::Optional,
        object::{
            AnyObj, ArrayListObj, HashMapObj, HashSetObj, Object, ObjectFinder as _, StringObj,
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
    let object_field = ObjectField("java/lang/Object");
    let comparator_field = ObjectField("java/util/Comparator");
    let stream_field = ObjectField("java/util/stream/Stream");
    let compare_descriptor =
        method!(((Object("java/lang/Object".into())), (Object("java/lang/Object".into()))) -> int);
    let to_string_descriptor = method!(() -> Object(java_lang_string.clone()));
    class_area.extend(native_class!(
        let method_area;

        impl "java/util/HashMap" {
            fn "<init>"(&this) {
                let thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |obj| {
                    obj.native_fields.push(Box::<HashMap<u32, u32>>::default());
                })
            }

            fn size(&this) -> Int {
                let thread, _verbose;
                HashMapObj::inspect(&thread.heap, this as usize, |m| m.len() as i32)
            }

            fn put(&this, key: object_field, value: object_field) -> object_field {
                let thread, verbose;
                super::resolve_and_invoke(thread, key, "hashCode", &method!(() -> int), verbose)?;
                thread.stackframe.locals[0] = key;
                super::run_until_return(thread, verbose)?;
                let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                // add the object to the hash map
                HashMapObj::inspect(&thread.heap, this as usize, |map| {
                    map.insert(hash_code, value)
                })
                .map(Option::unwrap_or_default)
            }

            fn get(&this, key: object_field) -> object_field {
                let thread, verbose;
                super::resolve_and_invoke(thread, key, "hashCode", &method!(() -> int), verbose)?;
                thread.stackframe.locals[0] = key;
                super::run_until_return(thread, verbose)?;
                let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                // add the object to the hash map
                HashMapObj::inspect(&thread.heap, this as usize, |map| {
                    map.get(&hash_code).copied()
                })
                .map(|opt| opt.unwrap_or(NULL))
            }
        }

        impl "java/util/HashSet" {
            fn "<init>"(&this) {
                let thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |o| o.native_fields.push(Box::<HashSet<u32>>::default()))
            }

            fn size(&this) -> Int {
                let thread, _verbose;
                HashSetObj::inspect(&thread.heap, this as usize, |this| this.len() as i32)
            }

            fn add(&this, key: object_field) -> Boolean {
                let thread, verbose;
                super::resolve_and_invoke(thread, key, "hashCode", &method!(() -> int), verbose)?;
                thread.stackframe.locals[0] = key;
                super::run_until_return(thread, verbose)?;
                let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                // add the object to the hash map
                HashSetObj::inspect(&thread.heap, this as usize, |set| set.insert(hash_code))
            }

            fn contains(&this, key: object_field) -> Boolean {
                let thread, verbose;
                super::resolve_and_invoke(thread, key, "hashCode", &method!(() -> int), verbose)?;
                thread.stackframe.locals[0] = key;
                super::run_until_return(thread, verbose)?;
                let hash_code = thread.stackframe.operand_stack.pop().unwrap();
                HashSetObj::inspect(&thread.heap, this as usize, |map| {
                    map.contains(&hash_code)
                })
            }
        }

        impl "java/util/ArrayList" {
            fn "<init>"(&this) {
                let thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |o| o.native_fields.push(Box::new(Vec::<u32>::new())))
            }

            fn add(&this, ptr: object_field) -> Boolean {
                let thread, _verbose;
                ArrayListObj::inspect(&thread.heap, this as usize, |o| o.push(ptr))?;
                Ok(true)
            }

            fn size(&this) -> Int {
                let thread, _verbose;
                ArrayListObj::inspect(&thread.heap, this as usize, |o| o.len() as i32)
            }

            fn sort(&this, cmp: comparator_field) {
                let thread, verbose;
                let length =
                    ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| arrlist.len())?;
                for partition in 1..length {
                    let target = ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| {
                        arrlist[partition]
                    })?;
                    for index in (0..partition).rev() {
                        let next = ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| {
                            arrlist[index]
                        })?;

                        // cmp.compare(target, next)
                        super::resolve_and_invoke(
                            thread,
                            cmp,
                            "compare",
                            &compare_descriptor,
                            verbose,
                        )?;
                        thread.stackframe.locals[0] = cmp;
                        thread.stackframe.locals[1] = target;
                        thread.stackframe.locals[2] = next;
                        super::run_until_return(thread, verbose)?;

                        let compare_value = thread.stackframe.operand_stack.pop().unwrap();
                        if compare_value as i32 >= 0 {
                            // target is greater than list[index]; place it and finish this partition
                            ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| {
                                arrlist[index + 1] = target;
                            })?;
                            break;
                        }
                        // target is less than list[index]; move it left
                        ArrayListObj::inspect(&thread.heap, this as usize, |arrlist| {
                            arrlist[index + 1] = arrlist[index];
                        })?;
                    }
                }
                Ok(())
            }

            fn toString(&this) -> StringField {
                let thread, verbose;
                let mut str = String::from("[");
                let length = ArrayListObj::inspect(&thread.heap, this as usize, |vec| vec.len() as u32)?;
                for index in 0..length {
                    let next_obj = ArrayListObj::inspect(&thread.heap, this as usize, |vec| vec[index as usize])?;
                    super::resolve_and_invoke(
                        thread,
                        next_obj,
                        "toString",
                        &to_string_descriptor,
                        verbose,
                    )?;
                    thread.stackframe.locals[0] = next_obj;
                    super::run_until_return(thread, verbose)?;
                    let str_ptr = thread.stackframe.operand_stack.pop().unwrap();
                    let string = StringObj::inspect(&thread.heap, str_ptr as usize, |arc| arc.clone())?;
                    if index == 0 {
                        str.push_str(&string);
                    } else {
                        write!(str, ", {string}").unwrap();
                    }
                }
                str.push(']');
                Ok(Arc::from(str))
            }

            fn stream(&this) -> stream_field {
                let thread, verbose;
                let array_stream = thread
                    .class_area
                    .search("java/util/ArrayList$Stream")
                    .unwrap();
                let mut stream = Object::from_class(array_stream);
                stream.fields[0] = this;
                stream.fields[2] =
                    ArrayListObj::inspect(&thread.heap, this as usize, |arrls| arrls.len() as u32)?;
                thread.rember(this, verbose);
                let stream_idx = thread.heap.lock().unwrap().allocate(stream);
                Ok(stream_idx)
            }
        }

        impl(abstract) "java/util/Comparator" {
            fn compare(&this, first: object_field, second: object_field) -> Boolean;
        }
    ));

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
                descriptor: field!(Object("java/util/ArrayList".into())),
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
                Ok(opt_idx)
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
                    difference as u64
                })
            },
        )),
        ..Default::default()
    };
    array_stream
        .interfaces
        .push("java/util/stream/Stream".into());
    array_stream.register_methods([arr_stream_next, arr_stream_count], method_area);

    class_area.extend([array_stream]);
}
