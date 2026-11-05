use std::{
    collections::HashMap,
    f64::consts::{E, PI, TAU},
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex, OnceLock},
};

use jvmrs_lib::{access, field, method, FieldType};
use rand::RngExt;

use crate::{
    class::{
        code::{native_property, NativeDoubleMethod, NativeNoop, NativeSingleMethod, NativeVoid},
        Class, Field,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{Heap, SharedClassArea, WorkingClassArea, WorkingMethodArea, NULL},
};

use self::primitives::make_primitives;

use super::{
    object::{AnyObj, Object, ObjectFinder, Random},
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
pub mod system;
pub mod throwable;

pub static mut OBJECT_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_BUILDER_CLASS: Option<Arc<Class>> = None;
pub static mut ARRAY_CLASS: Option<Arc<Class>> = None;
pub static mut RANDOM_CLASS: Option<Arc<Class>> = None;

/// Execute instructions until the current stack frame returns
///
/// Should mainly be used within a native method body
/// # Errors
pub fn run_until_return(thread: &mut Thread, verbose: bool) -> super::error::Result<()> {
    let target_stack_size = thread.stack.len() - 1;
    while thread.stack.len() > target_stack_size {
        thread.tick(verbose)?;
    }
    Ok(())
}

/// return a Class object of the given name
pub fn get_class(
    heap: &Mutex<Heap>,
    class_area: &SharedClassArea,
    obj_class: Arc<Class>,
) -> Option<u32> {
    static CLASS_CACHE: Mutex<OnceLock<HashMap<Arc<str>, u32>>> = Mutex::new(OnceLock::new());
    let mut binding = CLASS_CACHE.lock().unwrap();
    let class_cache = binding.get_or_init(HashMap::new);
    if let Some(&ptr) = class_cache.get(&obj_class.this) {
        return Some(ptr);
    }
    let mut class_obj = Object::from_class(class_area.search("java/lang/Class")?);
    // hold onto an entry so we don't have to clone obj_class
    let entry = binding.get_mut().unwrap().entry(obj_class.this.clone());
    class_obj.native_fields.push(Box::new(obj_class));
    let ptr = heap.lock().unwrap().allocate(class_obj);
    entry.insert_entry(ptr);
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
                Ok(get_class(&thread.heap, &thread.class_area, obj_class).unwrap_or(NULL))
            },
        )),
        ..Default::default()
    };
    let object_to_string = RawMethod::to_string(|_: &mut _, [obj_ref]: [u32; 1], _| {
        Ok(Arc::from(format!("{obj_ref:0>8X}")))
    });
    let object_hash = RawMethod {
        access_flags: access!(public native),
        name: "hashCode".into(),
        descriptor: method!(() -> int),
        code: RawCode::native(NativeSingleMethod(|_: &mut _, [ptr]: [u32; 1], _| {
            let mut hasher = DefaultHasher::new();
            ptr.hash(&mut hasher);
            let value = hasher.finish() as u32;
            Ok(value)
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
    let mut util_objects = RawClass::new(
        access!(public native),
        "java/util/Objects".into(),
        java_lang_object.clone(),
    );
    let objects_req_non_null = RawMethod {
        access_flags: access!(public native),
        name: "requireNonNull".into(),
        descriptor: method!(((Object(java_lang_object.clone()))) -> Object(java_lang_object.clone())),
        code: RawCode::native(NativeSingleMethod(|_: &mut _, [ptr]: [u32; 1], _| {
            // TODO: Use an exception here
            assert!(ptr != 0);
            Ok(ptr)
        })),
        ..Default::default()
    };
    util_objects.register_method(objects_req_non_null, method_area);

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
                AnyObj.inspect(&thread.heap, obj_ref as usize, |instance| {
                    instance.fields = vec![string, id];
                })
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

    let random_init = Random::make_init(rand::make_rng);
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
                    random_obj.random_range(0..right_bound)
                })
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

    let sqrt_double = RawMethod {
        access_flags: access!(public static native),
        name: "sqrt".into(),
        descriptor: method!((double) -> double),
        code: RawCode::native(NativeDoubleMethod(
            |_: &mut _, [left, right]: [u32; 2], _| {
                Ok(f64::from_bits((left as u64) << 32 | (right as u64))
                    .sqrt()
                    .to_bits())
            },
        )),
        ..Default::default()
    };
    let mut math = RawClass::new(
        access!(public native),
        "java/lang/Math".into(),
        java_lang_object.clone(),
    );
    math.static_data.extend([
        E.to_bits() as u32,
        (E.to_bits() >> 32) as u32,
        PI.to_bits() as u32,
        (PI.to_bits() >> 32) as u32,
        TAU.to_bits() as u32,
        (TAU.to_bits() >> 32) as u32,
    ]);
    math.statics.extend([
        (
            Field {
                access_flags: access!(public static final),
                name: "E".into(),
                descriptor: field!(double),
                ..Default::default()
            },
            0,
        ),
        (
            Field {
                access_flags: access!(public static final),
                name: "PI".into(),
                descriptor: field!(double),
                ..Default::default()
            },
            2,
        ),
        (
            Field {
                access_flags: access!(public static final),
                name: "TAU".into(),
                descriptor: field!(double),
                ..Default::default()
            },
            4,
        ),
    ]);
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

    arrays::add_native_methods(
        class_area,
        method_area,
        &java_lang_object,
        &java_lang_string,
    );
    string::add_native_methods(
        class_area,
        method_area,
        &java_lang_object,
        &java_lang_string,
    );
    system::add_native_methods(
        class_area,
        method_area,
        &java_lang_object,
        &java_lang_string,
    );

    class_area.extend([
        object,
        util_objects,
        enum_class,
        string_builder,
        random,
        math,
    ]);
    drop((java_lang_object, java_lang_string));
}
