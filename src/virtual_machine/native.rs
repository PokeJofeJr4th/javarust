use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex, OnceLock},
};

use jvmrs_lib::{access, method, FieldType};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::{
    class::{
        code::{
            native_property, NativeDoubleMethod, NativeNoop, NativeSingleMethod,
            NativeStringMethod, NativeVoid,
        },
        Class, Field,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{Heap, SharedClassArea, WorkingClassArea, WorkingMethodArea, NULL},
};

use self::{
    arrays::deep_to_string,
    primitives::make_primitives,
    string::{native_println_object, native_string_char_at, native_string_value_of},
};

use super::{
    object::{AnyObj, Array1, Array2, ArrayType, Object, ObjectFinder, Random, StringObj},
    Thread,
};

pub mod arrays;
pub mod character;
pub mod collections;
pub mod function;
pub mod primitives;
pub mod reflect;
pub mod stream;
pub mod string;
pub mod string_builder;
pub mod throwable;

pub static mut OBJECT_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_BUILDER_CLASS: Option<Arc<Class>> = None;
pub static mut ARRAY_CLASS: Option<Arc<Class>> = None;
pub static mut RANDOM_CLASS: Option<Arc<Class>> = None;

/// return a Class object of the given name
pub fn get_class(
    heap: &Mutex<Heap>,
    class_area: &SharedClassArea,
    obj_class: Arc<str>,
) -> Option<u32> {
    static CLASS_CACHE: Mutex<OnceLock<HashMap<Arc<str>, u32>>> = Mutex::new(OnceLock::new());
    let mut binding = CLASS_CACHE.lock().unwrap();
    let class_cache = binding.get_or_init(HashMap::new);
    if let Some(&ptr) = class_cache.get(&obj_class) {
        return Some(ptr);
    }
    let class_class = class_area.search("java/lang/Class")?;
    let mut class_obj = Object::from_class(&class_class);
    class_obj.native_fields.push(Box::new(class_class));
    let ptr = heap.lock().unwrap().allocate(class_obj);
    binding.get_mut().unwrap().insert(obj_class, ptr);
    drop(binding);
    Some(ptr)
}

#[allow(clippy::too_many_lines)]
/// # Panics
pub fn add_native_methods(method_area: &mut WorkingMethodArea, class_area: &mut WorkingClassArea) {
    let java_lang_object: Arc<str> = Arc::from("java/lang/Object");
    let java_lang_string: Arc<str> = Arc::from("java/lang/String");

    let noop_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        signature: None,
        attributes: Vec::new(),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };
    let object_get_class = RawMethod {
        access_flags: access!(public native),
        name: "getClass".into(),
        descriptor: method!(() -> Object("java/lang/Class".into())),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [obj]: [u32; 1], _| {
                let obj_class =
                    AnyObj.inspect(&thread.heap, obj as usize, |obj| obj.class.clone())?;
                Ok(Some(
                    get_class(&thread.heap, &thread.class_area, obj_class).unwrap_or(NULL),
                ))
            },
        )),
        ..Default::default()
    };
    let object_to_string = RawMethod::to_string(|_: &mut _, [obj_ref]: [u32; 1], _| {
        Ok(Some(Arc::from(format!("{obj_ref:0>8X}"))))
    });
    let object_hash = RawMethod {
        access_flags: access!(public native),
        name: "hashCode".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(|_: &mut _, [ptr]: [u32; 1], _| {
            let mut hasher = DefaultHasher::new();
            ptr.hash(&mut hasher);
            let value = hasher.finish() as u32;
            Ok(Some(value))
        })),
        ..Default::default()
    };

    let mut object = RawClass::new(
        access!(public native),
        java_lang_object.clone(),
        java_lang_object.clone(),
    );
    object.register_methods(
        [noop_init, object_to_string, object_hash, object_get_class],
        method_area,
    );

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
            |thread: &mut Thread, [obj_ref, string, id]: [u32; 3], _verbose| {
                AnyObj
                    .inspect(&thread.heap, obj_ref as usize, |instance| {
                        instance.fields = vec![string, id];
                    })
                    .map(Option::Some)
            },
        )),
        ..Default::default()
    };
    let enum_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_object.clone())),
        code: RawCode::native(NativeSingleMethod(native_property(AnyObj, |obj| {
            obj.fields[0]
        }))),
        ..Default::default()
    };
    let enum_name = RawMethod {
        name: "name".into(),
        ..enum_to_string.clone()
    };
    enum_class.register_methods([enum_init, enum_to_string, enum_name], method_area);

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
    arrays.register_methods(
        [arrays_to_string, arrays_to_string_obj_arr, deep_to_string],
        method_area,
    );
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
        code: RawCode::native(NativeSingleMethod(native_property(StringObj::SELF, |s| {
            s.len() as u32
        }))),
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
        code: RawCode::native(NativeSingleMethod(|_: &mut _, [l]: [u32; 1], _| {
            Ok(Some(l))
        })),
        ..Default::default()
    };
    let string_compare_to = RawMethod {
        access_flags: access!(public native),
        name: "compareTo".into(),
        descriptor: method!(((Object(java_lang_string.clone()))) -> int),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, other]: [u32; 2], _| {
                let this_str = StringObj::inspect(&thread.heap, this as usize, |a| a.clone())?;
                let other_str = StringObj::inspect(&thread.heap, other as usize, |a| a.clone())?;
                Ok(Some(match this_str.cmp(&other_str) {
                    std::cmp::Ordering::Less => u32::MAX,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }))
            },
        )),
        ..Default::default()
    };
    let string_contains = RawMethod {
        name: "contains".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object("java/lang/CharSequence".into()))) -> boolean),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [this, seq]: [u32; 2], _verbose| {
                let this_str = StringObj::inspect(&thread.heap, this as usize, |s| s.clone())?;
                let seq_str = StringObj::inspect(&thread.heap, seq as usize, |s| s.clone())?;
                let contains = this_str.contains(&*seq_str);
                Ok(Some(u32::from(contains)))
            },
        )),
        ..Default::default()
    };
    let mut string = RawClass::new(
        access!(public native),
        java_lang_string.clone(),
        java_lang_object.clone(),
    );
    string.register_methods(
        [
            string_length,
            char_at,
            string_value_of,
            string_to_string,
            string_compare_to,
            string_contains,
        ],
        method_area,
    );

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
    let to_string = RawMethod::to_string(string_builder::to_string);
    let mut string_builder = RawClass::new(
        access!(public native),
        "java/lang/StringBuilder".into(),
        java_lang_object.clone(),
    );
    string_builder.register_methods([builder_init, set_char_at, to_string], method_area);

    let random_init = Random::make_init(StdRng::from_entropy);
    let next_int = RawMethod {
        access_flags: access!(public native),
        name: "nextInt".into(),
        descriptor: method!((int) -> int),
        code: RawCode::native(NativeSingleMethod(
            |thread: &mut Thread, [obj_ref, right_bound]: [u32; 2], verbose: bool| {
                if verbose {
                    println!("java/util/Random.nextInt(int): obj_ref={obj_ref}");
                    println!("java/util/Random.nextInt(int): right_bound={right_bound}");
                }
                Random::inspect(&thread.heap, obj_ref as usize, |random_obj| {
                    random_obj.gen_range(0..right_bound)
                })
                .map(Option::Some)
            },
        )),
        ..Default::default()
    };
    let mut random = RawClass::new(
        access!(public native),
        "java/util/Random".into(),
        java_lang_object.clone(),
    );
    random.register_methods([random_init, next_int], method_area);

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
        code: RawCode::native(NativeVoid(|_: &mut _, [_, c]: [u32; 2], _| {
            let char = char::from_u32(c).ok_or_else(|| String::from("Invalid Character code"))?;
            println!("{char}");
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let println_bool = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((boolean) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, [_, b]: [u32; 2], _| {
            let bool = b != 0;
            println!("{bool}");
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let println_int = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((int) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, [_, i]: [u32; 2], _| {
            let int = i as i32;
            println!("{int}");
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let println_long = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((long) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, [_, left, right]: [u32; 3], _| {
            let long = ((left as u64) << 32 | (right as u64)) as i64;
            println!("{long}");
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let println_float = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!((float) -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, [_, f]: [u32; 2], _| {
            let float = f32::from_bits(f);
            println!("{float}");
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let println_empty = RawMethod {
        access_flags: access!(public native),
        name: "println".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeVoid(|_: &mut _, [_]: [u32; 1], _| {
            println!();
            Ok(Some(()))
        })),
        ..Default::default()
    };
    let mut printstream = RawClass::new(
        access!(public native),
        "java/io/PrintStream".into(),
        java_lang_object.clone(),
    );
    printstream.register_methods(
        [
            println_string,
            println_object,
            println_empty,
            println_float,
            println_int,
            println_bool,
            println_char,
            println_long,
        ],
        method_area,
    );

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
        code: RawCode::native(NativeVoid(|thread: &mut Thread, []: [u32; 0], verbose| {
            let system_class = thread.class_area.search("java/lang/System").unwrap();
            let out_ref = thread.heap.lock().unwrap().allocate(Object::from_class(
                &thread.class_area.search("java/io/PrintStream").unwrap(),
            ));
            system_class.static_data.lock().unwrap()[0] = out_ref;
            thread.rember(out_ref, verbose);
            Ok(Some(()))
        })),
        ..Default::default()
    };
    system.register_method(system_clinit, method_area);

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
             [src_idx, start, dest_idx, start_dest, count]: [u32; 5],
             _verbose| {
                let arr_size =
                    ArrayType::inspect(&thread.heap, src_idx as usize, |f| f.get_size())?;
                if arr_size == 1 {
                    let copied = Array1.inspect(&thread.heap, src_idx as usize, |fields| {
                        fields.contents[(start as usize)..(start + count) as usize].to_vec()
                    })?;
                    Array1
                        .inspect(&thread.heap, dest_idx as usize, |fields| {
                            for (i, value) in copied.into_iter().enumerate() {
                                fields.contents[start_dest as usize + i] = value;
                            }
                        })
                        .map(Option::Some)
                } else {
                    let copied = Array2.inspect(&thread.heap, src_idx as usize, |fields| {
                        fields.contents[(start as usize)..(start + count) as usize].to_vec()
                    })?;
                    Array2
                        .inspect(&thread.heap, dest_idx as usize, |fields| {
                            for (i, value) in copied.into_iter().enumerate() {
                                fields.contents[start_dest as usize + i] = value;
                            }
                        })
                        .map(Option::Some)
                }
            },
        )),
        ..Default::default()
    };
    system.register_method(arraycopy, method_area);

    let sqrt_double = RawMethod {
        access_flags: access!(public static native),
        name: "sqrt".into(),
        descriptor: method!((double) -> double),
        code: RawCode::native(NativeDoubleMethod(
            |_: &mut _, [left, right]: [u32; 2], _| {
                Ok(Some(
                    f64::from_bits((left as u64) << 32 | (right as u64))
                        .sqrt()
                        .to_bits(),
                ))
            },
        )),
        ..Default::default()
    };
    let mut math = RawClass::new(
        access!(public native),
        "java/lang/Math".into(),
        java_lang_object.clone(),
    );
    math.register_method(sqrt_double, method_area);

    throwable::add_native_methods(
        &java_lang_object,
        &java_lang_string,
        method_area,
        class_area,
    );
    collections::add_native_collections(
        class_area,
        method_area,
        &java_lang_object,
        &java_lang_string,
    );
    reflect::add_native_methods(
        method_area,
        class_area,
        &java_lang_object,
        &java_lang_string,
    );
    function::add_native_methods(
        method_area,
        class_area,
        &java_lang_object,
        &java_lang_string,
    );
    stream::add_native_methods(method_area, class_area, &java_lang_object);

    arrays.register_methods(array_methods, method_area);
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
        math,
    ]);
    drop((java_lang_object, java_lang_string));
}
