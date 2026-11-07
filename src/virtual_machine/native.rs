use std::{
    collections::HashMap,
    f64::consts::{E, PI, TAU},
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex, OnceLock},
};

use jvmrs_lib::{access, field, method, MethodDescriptor};
use rand::{rngs::StdRng, RngExt};

use crate::{
    class::{
        code::{native_property, NativeDoubleMethod, NativeSingleMethod, NativeVoid},
        Class, Field,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{Heap, SharedClassArea, WorkingClassArea, WorkingMethodArea},
    virtual_machine::{error, object::StringBuilder},
};

use super::{
    object::{AnyObj, Object, ObjectFinder},
    Thread,
};

use field::{Char, Int, ObjectField, StringField};
use primitives::make_primitives;

pub mod arrays;
pub mod character;
pub mod collections;
pub mod function;
pub mod primitives;
pub mod reflect;
pub mod stream;
pub mod string;
pub mod system;
pub mod throwable;

mod field;

pub static mut OBJECT_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_BUILDER_CLASS: Option<Arc<Class>> = None;
pub static mut ARRAY_CLASS: Option<Arc<Class>> = None;
pub static mut RANDOM_CLASS: Option<Arc<Class>> = None;

#[macro_export]
macro_rules! parameter_size {
    (Double, $($rest:tt,)*) => {{
        2 + $crate::parameter_size!($($rest,)*)
    }};

    (Long, $($rest:tt,)*) => {{
        2 + $crate::parameter_size!($($rest,)*)
    }};

    (this, $($rest:tt,)*) => {{
        $crate::parameter_size!($($rest,)*)
    }};

    ($pty:tt, $($rest:tt,)*) => {{
        1 + $crate::parameter_size!($($rest,)*)
    }};

    () => {0};
}

#[macro_export]
macro_rules! has_this {
    ($($any:tt)+) => {
        1
    };

    () => {
        0
    };
}

#[macro_export]
macro_rules! return_type {
    () => {
        ::core::option::Option::None
    };

    ($pty:expr) => {
        ::core::option::Option::Some($crate::virtual_machine::native::field::Field::as_field(
            $pty,
        ))
    };
}

#[macro_export]
macro_rules! ret {
    ($pty:expr) => {
        $pty
    };
    () => {
        $crate::virtual_machine::native::field::Void
    };
}

#[macro_export]
macro_rules! fn_name {
    ($name:ident) => {
        stringify!($name)
    };

    ($name:literal) => {
        $name
    };
}

#[macro_export]
macro_rules! maybe_static {
    () => {
        jvmrs_lib::access!()
    };

    ($($t:tt)+) => {
        jvmrs_lib::access!(static)
    };
}

#[macro_export]
macro_rules! native_class {
    (let $method_area:ident; $(impl$(($($access:tt)*))? $cls:literal $(($supcls:literal))?{$($clsbody:tt)*})*) => {[$({
        let mut class = $crate::class_loader::RawClass::new(access!(public native $($($access)*)?), $cls.into(), native_class!(@super $($supcls)?));
        class.register_methods(native_class!{
            @{$($clsbody)*}
            access!()[]
        }, $method_area);
        class
    }),*]};

    (@{
            fn $fn_name:tt($(&$this:ident$(,)?)?$($param:ident: $paramty:ident),*) $(->$ret:ident)?; $($clsbody:tt)*
        }
        access!($($access:tt)*)
        [$($prev:tt)*]
    ) => {
        native_class!{
            @{$($clsbody)*}
            access!()
            [$($prev)*
                $crate::class_loader::RawMethod {
                    access_flags: jvmrs_lib::access!(public abstract native $($access)*) | $crate::maybe_static!($($this)?),
                    name: $crate::fn_name!($fn_name).into(),
                    descriptor: jvmrs_lib::MethodDescriptor {
                        parameters: vec![$($crate::virtual_machine::native::field::Field::as_field($paramty)),*],
                        return_type: $crate::return_type!($($ret)?),
                        parameter_size: $crate::parameter_size!($($paramty,)*),
                    },
                    code: $crate::class_loader::RawCode::Abstract,
                    ..::core::default::Default::default()
                },
            ]
        }
    };

    (@{
            fn $fn_name:tt($(&$this:ident$(,)?)?$($param:ident: $paramty:ident),*) $(->$ret:ident)? {
                let $thread:ident, $verbose:ident;
                $($body:tt)*
            } $($clsbody:tt)*
        }
        access!($($access:tt)*)
        [$($prev:tt)*]
    ) => {
        native_class!{
            @{$($clsbody)*}
            access!()
            [$($prev)*
                $crate::class_loader::RawMethod {
                    access_flags: jvmrs_lib::access!(public native $($access)*) | $crate::maybe_static!($($this)?),
                    name: $crate::fn_name!($fn_name).into(),
                    descriptor: jvmrs_lib::MethodDescriptor {
                        parameters: vec![$($crate::virtual_machine::native::field::Field::as_field($paramty)),*],
                        return_type: $crate::return_type!($($ret)?),
                        parameter_size: $crate::parameter_size!($($paramty,)*),
                    },
                    code: $crate::class_loader::RawCode::native($crate::virtual_machine::native::field::Field::make_native($crate::ret!($($ret)?), move |$thread: &mut $crate::virtual_machine::Thread, args: [u32; $crate::parameter_size!($($paramty,)*) + $crate::has_this!($($this)?)], $verbose| {
                        let mut argc: usize = 0;
                        $(
                            let $this = args[0];
                            argc += 1;
                        )?
                        $(
                            let $param = $crate::virtual_machine::native::field::Field::get_parameter(&$paramty,$thread, argc, &args);
                            argc += $crate::parameter_size!($paramty,);
                        )*
                        // shut up the compiler warning for unused variable
                        let _ = argc;

                        $($body)*
                    })),
                    ..::core::default::Default::default()
                },
            ]
        }
    };

    (@{} access!()[$($prev:tt)*]) => {[$($prev)*]};

    (@super $supcls:literal) => {$supcls.into()};
    (@super) => {"java/lang/Object".into()};
}

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

/// first, resolve a concrete method for an object. Then, create a new stackframe for that method
/// # Errors
pub fn resolve_and_invoke(
    thread: &mut Thread,
    this_index: u32,
    method: &str,
    descriptor: &MethodDescriptor,
    verbose: bool,
) -> error::Result<()> {
    // Push a fake return address
    thread.stackframe.operand_stack.push(0);
    let (resolved_class, resolved_method) =
        AnyObj.inspect(&thread.heap, this_index as usize, |o| {
            o.resolve_method(
                &thread.method_area,
                &thread.class_area,
                method,
                descriptor,
                verbose,
            )
        })?;
    thread.invoke_method(resolved_method, resolved_class);
    Ok(())
}

/// return a Class object of the given name
pub fn get_class(heap: &Mutex<Heap>, class_area: &SharedClassArea, obj_class: Arc<Class>) -> u32 {
    static CLASS_CACHE: Mutex<OnceLock<HashMap<Arc<str>, u32>>> = Mutex::new(OnceLock::new());
    static CLASS: Mutex<OnceLock<Arc<Class>>> = Mutex::new(OnceLock::new());
    let mut binding = CLASS_CACHE.lock().unwrap();
    let class_cache = binding.get_or_init(HashMap::new);
    if let Some(&ptr) = class_cache.get(&obj_class.this) {
        return ptr;
    }
    let mut class_obj = Object::from_class(
        CLASS
            .lock()
            .unwrap()
            .get_or_init(|| class_area.search("java/lang/Class").unwrap())
            .clone(),
    );
    // hold onto an entry so we don't have to clone obj_class
    let entry = binding.get_mut().unwrap().entry(obj_class.this.clone());
    class_obj.native_fields.push(Box::new(obj_class));
    let ptr = heap.lock().unwrap().allocate(class_obj);
    entry.insert_entry(ptr);
    drop(binding);
    ptr
}

#[allow(clippy::too_many_lines)]
/// # Panics
pub fn add_native_methods(method_area: &mut WorkingMethodArea, class_area: &mut WorkingClassArea) {
    let java_lang_object: Arc<str> = Arc::from("java/lang/Object");
    let java_lang_string: Arc<str> = Arc::from("java/lang/String");

    let object_field = ObjectField("java/lang/Object");
    let class_field = ObjectField("java/lang/Class");

    class_area.extend(native_class!(
        let method_area;

        impl "java/lang/Object" {
            fn "<init>"(&_this) {
                let _thread, _verbose;
                Ok(())
            }

            fn getClass(&this) -> class_field {
                let thread, _verbose;
                let obj_class = AnyObj.inspect(&thread.heap, this as usize, |obj| obj.class.clone())?;
                Ok(get_class(&thread.heap, &thread.class_area, obj_class))
            }

            fn hashCode(&this) -> Int {
                let _thread, _verbose;
                let mut hasher = DefaultHasher::new();
                this.hash(&mut hasher);
                let value = hasher.finish() as i32;
                Ok(value)
            }

            fn toString(&this) -> StringField {
                let _thread, _verbose;
                Ok(Arc::from(format!("{this:0>8X}")))
            }
        }

        impl "java/util/Objects" {
            fn requireNonNull(object: object_field) -> object_field {
                let _thread, _verbose;
                assert!(object != 0);
                Ok(object)
            }
        }

        impl "java/lang/StringBuilder" {
            fn "<init>"(&this, string: StringField) {
                let thread, _verbose;
                let init_string = string.to_string();
                AnyObj.inspect(&thread.heap, this as usize, |heap_obj| {
                    heap_obj.native_fields.push(Box::new(init_string));
                })
            }

            fn setCharAt(&this, index: Int, character: Char) {
                let thread, verbose;
                let this = this as usize;
                let index = index as usize;
                if verbose {
                    println!("setting char at {index} to {character:?}");
                }
                StringBuilder::SELF.inspect(&thread.heap, this, |string_ref| {
                    if verbose {
                        println!("StringBuilder = {string_ref:?}");
                    }
                    string_ref.replace_range(
                        string_ref
                            .char_indices()
                            .nth(index)
                            .map(|(pos, ch)| pos..pos + ch.len_utf8())
                            .unwrap(),
                        &String::from(character),
                    );
                })
            }

            fn toString(&this) -> StringField {
                let thread, _verbose;
                let string = Arc::from(&*StringBuilder::inspect(
                    &thread.heap,
                    this as usize,
                    |a| a.clone(),
                )?);
                Ok(string)
            }
        }

        impl "java/util/Random" {
            fn "<init>"(&this) {
                let thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |o| {
                    o.native_fields.push(Box::new(rand::make_rng::<StdRng>()));
                })
            }

            fn nextInt(&this, right_bound: Int) -> Int {
                let thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |o| o.native_fields[0].downcast_mut::<StdRng>().unwrap().random_range(0..(right_bound as i32)))
            }
        }
    ));

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
                descriptor: field!(Object(java_lang_string.clone())),
                ..Default::default()
            },
            0,
        ),
        (
            Field {
                access_flags: access!(public native),
                name: "enum$id".into(),
                descriptor: field!(int),
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
    arrays::add_native_methods(class_area, method_area, &java_lang_object);
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

    class_area.extend([enum_class, math]);
    drop((java_lang_object, java_lang_string));
}
