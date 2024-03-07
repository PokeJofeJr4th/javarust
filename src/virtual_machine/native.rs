use std::sync::{Arc, Mutex};

use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::{
    access,
    class::{
        code::{
            NativeDoubleMethod, NativeNoop, NativeSingleMethod, NativeStringMethod, NativeTodo,
            NativeVoid,
        },
        Class, Field, FieldType, MethodDescriptor,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    method,
};

use self::{
    arrays::deep_to_string,
    primitives::make_primitives,
    string::{
        native_println_object, native_string_char_at, native_string_len, native_string_value_of,
    },
};

use super::{
    object::{Array1, Array2, ArrayType, Object, ObjectFinder},
    StackFrame, Thread,
};

pub mod arrays;
pub mod primitives;
pub mod string;
pub mod string_builder;
pub mod throwable;

pub static mut OBJECT_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_BUILDER_CLASS: Option<Arc<Class>> = None;
pub static mut ARRAY_CLASS: Option<Arc<Class>> = None;
pub static mut RANDOM_CLASS: Option<Arc<Class>> = None;

#[allow(clippy::too_many_lines)]
/// # Panics
pub fn add_native_methods(method_area: &mut WorkingMethodArea, class_area: &mut WorkingClassArea) {
    let java_lang_object: Arc<str> = Arc::from("java/lang/Object");
    let java_lang_string: Arc<str> = Arc::from("java/lang/String");
    let object_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        signature: None,
        attributes: Vec::new(),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };
    let object_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(
            |_: &mut _, _: &_, [obj_ref]: [u32; 1], _| {
                // basically random bits
                let fake_addr = 3_141_592u32.wrapping_add(obj_ref);
                Ok(Arc::from(format!("{fake_addr:0>8X}")))
            },
        )),
        ..Default::default()
    };

    let mut object = RawClass::new(
        access!(public native),
        java_lang_object.clone(),
        java_lang_object.clone(),
    );
    object
        .methods
        .push(object_init.name(java_lang_object.clone()));
    object
        .methods
        .push(object_to_string.name(java_lang_object.clone()));

    let mut enum_class = RawClass::new(
        access!(public abstract native),
        "java/lang/Enum".into(),
        java_lang_object.clone(),
    );
    enum_class.field_size = 2;
    enum_class.fields = vec![
        (
            Field {
                access_flags: access!(public native),
                name: "enum$name".into(),
                descriptor: FieldType::Object(java_lang_string.clone()),
                ..Default::default()
            },
            0,
        ),
        (
            Field {
                access_flags: access!(public native),
                name: "enum$id".into(),
                descriptor: FieldType::Int,
                ..Default::default()
            },
            1,
        ),
    ];
    let enum_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(((Object(java_lang_string.clone())), int) -> void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, _: &_, [obj_ref, string, id]: [u32; 3], _verbose| {
                let Some(enum_class) = thread.class_area.search("java/lang/Enum") else {
                    return Err(String::from("Couldn't find class java/lang/Enum"));
                };
                enum_class.get_mut(
                    &mut thread.heap.lock().unwrap(),
                    obj_ref as usize,
                    |instance| {
                        instance.fields = vec![string, id];
                    },
                )
            },
        )),
        ..Default::default()
    };
    let enum_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_object.clone())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, _: &_, [enum_ref]: [u32; 1], _verbose| {
                let Some(enum_class) = thread.class_area.search("java/lang/Enum") else {
                    return Err(String::from("Couldn't find class java/lang/Enum"));
                };
                enum_class.get(
                    &thread.heap.lock().unwrap(),
                    enum_ref as usize,
                    |instance| instance.fields[0],
                )
            },
        )),
        ..Default::default()
    };
    let enum_name = RawMethod {
        name: "name".into(),
        ..enum_to_string.clone()
    };
    enum_class.methods.extend([
        enum_init.name(enum_class.this.clone()),
        enum_to_string.name(enum_class.this.clone()),
        enum_name.name(enum_class.this.clone()),
    ]);

    let array = RawClass::new(
        access!(public native),
        "java/lang/Array".into(),
        java_lang_object.clone(),
    );

    let arrays_to_string = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]int)) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(arrays::to_string)),
        ..Default::default()
    };
    let arrays_to_string_obj_arr = RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!((([]Object(java_lang_object.clone()))) -> Object(java_lang_string.clone())),
        signature: None,
        code: RawCode::native(NativeStringMethod(arrays::to_string)),
        attributes: Vec::new(),
        ..Default::default()
    };
    let deep_to_string = RawMethod {
        access_flags: access!(public static native),
        name: "deepToString".into(),
        descriptor: method!((([]Object(java_lang_object.clone()))) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(deep_to_string)),
        ..Default::default()
    };
    let mut arrays = RawClass::new(
        access!(public native),
        "java/util/Arrays".into(),
        java_lang_object.clone(),
    );
    arrays.methods.extend([
        arrays_to_string.name(arrays.this.clone()),
        arrays_to_string_obj_arr.name(arrays.this.clone()),
        deep_to_string.name(arrays.this.clone()),
    ]);
    let array_methods = make_primitives(method_area, class_area, java_lang_object.clone());
    arrays.methods.extend(
        array_methods
            .iter()
            .map(|method| method.name(arrays.this.clone())),
    );

    let string_length = RawMethod {
        access_flags: access!(public native),
        name: "length".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(native_string_len)),
        ..Default::default()
    };
    let char_at = RawMethod {
        access_flags: access!(public native),
        name: "charAt".into(),
        descriptor: method!((int) -> char),
        code: RawCode::native(NativeSingleMethod(native_string_char_at)),
        ..Default::default()
    };
    let string_value_of = RawMethod {
        access_flags: access!(public native),
        name: "valueOf".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeSingleMethod(native_string_value_of)),
        ..Default::default()
    };
    let string_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeSingleMethod(|_: &mut _, _: &_, [l]: [u32; 1], _| {
            Ok(l)
        })),
        ..Default::default()
    };
    let mut string = RawClass::new(
        access!(public native),
        java_lang_string.clone(),
        java_lang_object.clone(),
    );
    string.methods.extend([
        string_length.name(string.this.clone()),
        char_at.name(string.this.clone()),
        string_value_of.name(string.this.clone()),
        string_to_string.name(string.this.clone()),
    ]);

    let builder_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(((Object(java_lang_string.clone()))) -> void),
        code: RawCode::native(NativeVoid(string_builder::init)),
        ..Default::default()
    };
    let set_char_at = RawMethod {
        access_flags: access!(public native),
        name: "setCharAt".into(),
        descriptor: method!((int, char) -> void),
        code: RawCode::native(NativeVoid(string_builder::set_char_at)),
        ..Default::default()
    };
    let to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(string_builder::to_string)),
        ..Default::default()
    };
    let mut string_builder = RawClass::new(
        access!(public native),
        "java/lang/StringBuilder".into(),
        java_lang_object.clone(),
    );
    string_builder.methods.extend([
        builder_init.name(string_builder.this.clone()),
        set_char_at.name(string_builder.this.clone()),
        to_string.name(string_builder.this.clone()),
    ]);

    let random_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, _: &_, [obj]: [u32; 1], _verbose: bool| {
                unsafe { RANDOM_CLASS.as_ref().unwrap() }.as_ref().get_mut(
                    &mut thread.heap.lock().unwrap(),
                    obj as usize,
                    |instance| {
                        instance
                            .native_fields
                            .push(Box::new(StdRng::from_entropy()));
                    },
                )
            },
        )),
        ..Default::default()
    };
    let next_int = RawMethod {
        access_flags: access!(public native),
        name: "nextInt".into(),
        descriptor: method!((int) -> int),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread,
             _stackframe: &Mutex<StackFrame>,
             [obj_ref, right_bound]: [u32; 2],
             verbose: bool| {
                if verbose {
                    println!("java/util/Random.nextInt(int): obj_ref={obj_ref}");
                }
                if verbose {
                    println!("java/util/Random.nextInt(int): right_bound={right_bound}");
                }
                unsafe { RANDOM_CLASS.as_ref().unwrap() }.as_ref().get_mut(
                    &mut thread.heap.lock().unwrap(),
                    obj_ref as usize,
                    |random_obj| {
                        random_obj.native_fields[0]
                            .downcast_mut::<StdRng>()
                            .unwrap()
                            .gen_range(0..right_bound)
                    },
                )
            },
        )),
        ..Default::default()
    };
    let mut random = RawClass::new(
        access!(public native),
        "java/util/Random".into(),
        java_lang_object.clone(),
    );
    random.methods.extend([
        random_init.name(random.this.clone()),
        next_int.name(random.this.clone()),
    ]);

    let println_string = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!(((Object(java_lang_string.clone()))) -> void),
        code: RawCode::native(NativeVoid(native_println_object)),
        ..Default::default()
    };
    let println_object = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> void),
        code: RawCode::native(NativeVoid(native_println_object)),
        ..Default::default()
    };
    let println_char = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((char) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, [_, c]: [u32; 2], _| {
            let char = char::from_u32(c).ok_or_else(|| String::from("Invalid Character code"))?;
            println!("{char}");
            Ok(())
        })),
        ..Default::default()
    };
    let println_bool = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((boolean) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, [b]: [u32; 1], _| {
            let bool = b != 0;
            println!("{bool}");
            Ok(())
        })),
        ..Default::default()
    };
    let println_int = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((int) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, [i]: [u32; 1], _| {
            let int = i as i32;
            println!("{int}");
            Ok(())
        })),
        ..Default::default()
    };
    let println_long = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((long) -> void),
        code: RawCode::native(NativeVoid(
            |_: &mut _, _: &_, [_, left, right]: [u32; 3], _| {
                let long = ((left as u64) << 32 | (right as u64)) as i64;
                println!("{long}");
                Ok(())
            },
        )),
        ..Default::default()
    };
    let println_float = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((float) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, [_, f]: [u32; 2], _| {
            let float = f32::from_bits(f);
            println!("{float}");
            Ok(())
        })),
        ..Default::default()
    };
    let println_empty = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, []: [u32; 0], _| {
            println!();
            Ok(())
        })),
        ..Default::default()
    };
    let mut printstream = RawClass::new(
        access!(public native),
        "java/io/PrintStream".into(),
        java_lang_object.clone(),
    );
    printstream.methods.extend([
        println_string.name(printstream.this.clone()),
        println_object.name(printstream.this.clone()),
        println_empty.name(printstream.this.clone()),
        println_float.name(printstream.this.clone()),
        println_int.name(printstream.this.clone()),
        println_bool.name(printstream.this.clone()),
        println_char.name(printstream.this.clone()),
        println_long.name(printstream.this.clone()),
    ]);

    // let system_out = heap.allocate(Object::from_class(class_area, &printstream));

    let mut system = RawClass::new(
        access!(public native),
        "java/lang/System".into(),
        java_lang_object.clone(),
    );
    let system_clinit = RawMethod {
        name: "<clinit>".into(),
        access_flags: access!(public static native),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, _: &_, []: [u32; 0], _verbose| {
                let system_class = thread.class_area.search("java/lang/System").unwrap();
                let out_ref = thread.heap.lock().unwrap().allocate(Object::from_class(
                    &thread.class_area,
                    &thread.class_area.search("java/io/PrintStream").unwrap(),
                ));
                system_class.static_data.lock().unwrap()[0] = out_ref;
                thread.heap.lock().unwrap().inc_ref(out_ref);
                Ok(())
            },
        )),
        ..Default::default()
    };
    system.methods.push(system_clinit.name(system.this.clone()));

    system.static_data.push(NULL);
    system.statics.push((
        Field {
            access_flags: access!(public native),
            name: "out".into(),
            descriptor: FieldType::Object("java/io/PrintStream".into()),
            attributes: Vec::new(),
            signature: None,
            constant_value: None,
        },
        0,
    ));

    let arraycopy = RawMethod {
        access_flags: access!(public native),
        name: "arraycopy".into(),
        descriptor: method!((
            (Object(java_lang_object.clone())), 
            int, 
            (Object(java_lang_object.clone())), 
            int, 
            int
        ) -> void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread,
             _: &_,
             [src_idx, start, dest_idx, start_dest, count]: [u32; 5],
             _verbose| {
                let arr_size =
                    ArrayType.get(&thread.heap.lock().unwrap(), src_idx as usize, |ty| {
                        ty.get_size()
                    })?;
                if arr_size == 1 {
                    let copied =
                        Array1.get(&thread.heap.lock().unwrap(), src_idx as usize, |fields| {
                            fields.contents[(start as usize)..(start + count) as usize].to_vec()
                        })?;
                    Array1.get_mut(
                        &mut thread.heap.lock().unwrap(),
                        dest_idx as usize,
                        |fields| {
                            for (i, value) in copied.into_iter().enumerate() {
                                fields.contents[start_dest as usize + i] = value;
                            }
                        },
                    )
                } else {
                    let copied =
                        Array2.get(&thread.heap.lock().unwrap(), src_idx as usize, |fields| {
                            fields.contents[(start as usize)..(start + count) as usize].to_vec()
                        })?;
                    Array2.get_mut(
                        &mut thread.heap.lock().unwrap(),
                        dest_idx as usize,
                        |fields| {
                            for (i, value) in copied.into_iter().enumerate() {
                                fields.contents[start_dest as usize + i] = value;
                            }
                        },
                    )
                }
            },
        )),
        ..Default::default()
    };
    system.methods.push(arraycopy.name(system.this.clone()));

    let make_concat_with_constants = RawMethod {
        access_flags: access!(public static native),
        name: "makeConcatWithConstants".into(),
        descriptor: MethodDescriptor {
            parameter_size: 5,
            parameters: vec![
                FieldType::Object("java/lang/invoke/MethodHandles$Lookup".into()),
                FieldType::Object(java_lang_string.clone()),
                FieldType::Object("java/lang/invoke/MethodType".into()),
                FieldType::Object(java_lang_string.clone()),
                FieldType::Array(Box::new(FieldType::Object("java/lang/Object".into()))),
            ],
            return_type: Some(FieldType::Object("java/lang/invoke/CallSite".into())),
        },
        code: RawCode::native(NativeTodo),
        ..Default::default()
    };

    let mut string_concat_factory = RawClass::new(
        access!(public native),
        "java/lang/invoke/StringConcatFactory".into(),
        java_lang_object.clone(),
    );
    string_concat_factory
        .methods
        .push(make_concat_with_constants.name(string_concat_factory.this.clone()));

    let sqrt_double = RawMethod {
        access_flags: access!(public static native),
        name: "sqrt".into(),
        descriptor: method!((double) -> double),
        code: RawCode::native(NativeDoubleMethod(
            |_: &mut _, _: &_, [left, right]: [u32; 2], _| {
                let param = f64::from_bits((left as u64) << 32 | (right as u64));
                Ok(param.sqrt().to_bits())
            },
        )),
        ..Default::default()
    };
    let mut math = RawClass::new(
        access!(public native),
        "java/lang/Math".into(),
        java_lang_object.clone(),
    );
    math.methods.push(sqrt_double.name(math.this.clone()));

    throwable::add_native_methods(
        &java_lang_object,
        &java_lang_string,
        method_area,
        class_area,
    );

    method_area.extend([
        (object.this.clone(), object_init),
        (object.this.clone(), object_to_string),
        (enum_class.this.clone(), enum_init),
        (enum_class.this.clone(), enum_to_string),
        (enum_class.this.clone(), enum_name),
        (arrays.this.clone(), arrays_to_string),
        (arrays.this.clone(), arrays_to_string_obj_arr),
        (arrays.this.clone(), deep_to_string),
        (string.this.clone(), string_length),
        (string.this.clone(), char_at),
        (string.this.clone(), string_value_of),
        (string.this.clone(), string_to_string),
        (string_builder.this.clone(), builder_init),
        (string_builder.this.clone(), to_string),
        (string_builder.this.clone(), set_char_at),
        (random.this.clone(), random_init),
        (random.this.clone(), next_int),
        (system.this.clone(), arraycopy),
        (system.this.clone(), system_clinit),
        (printstream.this.clone(), println_string),
        (printstream.this.clone(), println_object),
        (printstream.this.clone(), println_float),
        (printstream.this.clone(), println_int),
        (printstream.this.clone(), println_bool),
        (printstream.this.clone(), println_char),
        (printstream.this.clone(), println_long),
        (printstream.this.clone(), println_empty),
        (
            string_concat_factory.this.clone(),
            make_concat_with_constants,
        ),
        (math.this.clone(), sqrt_double),
    ]);
    method_area.extend(
        array_methods
            .into_iter()
            .map(|method| (arrays.this.clone(), method)),
    );
    class_area.extend([
        object,
        enum_class,
        array,
        arrays,
        string,
        string_builder,
        random,
        system,
        printstream,
        string_concat_factory,
        math,
    ]);
    drop((java_lang_object, java_lang_string));
}
