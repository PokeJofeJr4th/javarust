use std::{
    collections::HashMap,
    f64::consts::{E, PI, TAU},
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex, OnceLock},
};

use jvmrs_lib::MethodDescriptor;
use rand::{rngs::StdRng, RngExt};

use crate::{
    class::Class,
    data::{Heap, SharedClassArea, WorkingClassArea, WorkingMethodArea},
    virtual_machine::{error, native::field::Double, object::StringBuilder},
};

use super::{
    object::{AnyObj, Object, ObjectFinder},
    Thread,
};

use field::{Char, Int, StringField};
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
macro_rules! native_class {
    (
        use $method_area:ident;
        $(
            impl
                $(($($access:tt)*))?
                $cls:ident$(::$classsegment:ident)*
                $(($supcls:ident$(::$supclasssegment:ident)*))?
            {
                $($clsbody:tt)*
            }
        )*
    ) => {[$({
        let mut class = $crate::class_loader::RawClass::new(
            jvmrs_lib::access!(public native $($($access)*)?),
            $crate::native_class!(@parse_class $cls$(::$classsegment)*).into(),
            native_class!(@super $($supcls$(::$supclasssegment)*)?)
        );
        native_class!{
            @{$($clsbody)*}
            []
            {}
            class
            $method_area
        }
        class
    }),*]};

    (@{
            fn $fn_name:tt($(&$this:ident$(,)?)?$($param:ident: $paramty:ident$(::$paramsegment:ident)*),*) $(->$retty:ident$(::$retsegment:ident)*)?; $($clsbody:tt)*
        }
        [$($methods:tt)*]
        {$($post:tt)*}
        $class:ident
        $method_area:ident
    ) => {
        native_class!{
            @{$($clsbody)*}
            [$($methods)*
                $crate::class_loader::RawMethod {
                    access_flags: jvmrs_lib::access!(public abstract native) | $crate::native_class!(@maybe_static $($this)?),
                    name: $crate::native_class!(@fn_name $fn_name).into(),
                    descriptor: jvmrs_lib::MethodDescriptor {
                        parameters: vec![$($crate::virtual_machine::native::field::Field::as_field($crate::native_class!(@parse_type $paramty$(::$paramsegment)*))),*],
                        return_type: $crate::native_class!(@return_type $($retty$(::$retsegment)*)?),
                        parameter_size: $crate::native_class!(@param_size $($paramty,)*),
                    },
                    code: $crate::class_loader::RawCode::Abstract,
                    ..::core::default::Default::default()
                },
            ]
            {$($post)*}
            $class
            $method_area
        }
    };

    (@{
            fn $fn_name:tt($(&$this:ident$(,)?)?$($param:ident: $paramty:ident$(::$paramsegment:ident)*),*) $(->$retty:ident$(::$retsegment:ident)*)? {
                use $thread:ident, $verbose:ident;
                $($body:tt)*
            } $($clsbody:tt)*
        }
        [$($methods:tt)*]
        {$($post:tt)*}
        $class:ident
        $method_area:ident
    ) => {
        native_class!{
            @{$($clsbody)*}
            [$($methods)*
                $crate::class_loader::RawMethod {
                    access_flags: jvmrs_lib::access!(public native) | $crate::native_class!(@maybe_static $($this)?),
                    name: $crate::native_class!(@fn_name $fn_name).into(),
                    descriptor: jvmrs_lib::MethodDescriptor {
                        parameters: vec![$($crate::virtual_machine::native::field::Field::as_field($crate::native_class!(@parse_type $paramty$(::$paramsegment)*))),*],
                        return_type: $crate::native_class!(@return_type $($retty$(::$retsegment)*)?),
                        parameter_size: $crate::native_class!(@param_size $($paramty,)*),
                    },
                    code: $crate::class_loader::RawCode::native($crate::virtual_machine::native::field::Field::make_native($crate::native_class!(@ret $($retty$(::$retsegment)*)?), move |$thread: &mut $crate::virtual_machine::Thread, args: [u32; $crate::native_class!(@param_size $($paramty,)*) + $crate::native_class!(@any $($this)?)], $verbose| {
                        #[allow(unused_mut)]
                        let mut argc: usize = 0;
                        $(
                            let $this = args[0];
                            argc += 1;
                        )?
                        $(
                            let $param = $crate::virtual_machine::native::field::Field::get_parameter(&$crate::native_class!(@parse_type $paramty$(::$paramsegment)*), $thread, argc, &args);
                            argc += $crate::native_class!(@param_size $paramty,);
                        )*
                        // shut up the compiler warning for unused variable
                        let _ = (argc, args);

                        $($body)*
                    })),
                    ..::core::default::Default::default()
                },
            ]
            {$($post)*}
            $class
            $method_area
        }
    };

    (@{
        static $varname:tt: $varty:ident$(::$varseg:ident)* = $val:expr;
        $($clsbody:tt)*
    }
    [$($methods:tt)*]{$($post:tt)*}$class:ident$method_area:ident) => {
        $crate::native_class!{@{
                $($clsbody)*
            }
            [$($methods)*]
            {
                $($post)*
                let size = $class.static_data.len();
                $class.statics.push(($crate::class::Field {
                    access_flags: jvmrs_lib::access!(public native),
                    name: $crate::native_class!(@fn_name $varname).into(),
                    descriptor: $crate::virtual_machine::native::field::Field::as_field($crate::native_class!(@ret $varty$(::$varseg)*)),
                    ..Default::default()
                }, size));
                $class.static_data.extend($crate::virtual_machine::native::field::Field::as_words(&$crate::native_class!(@ret $varty$(::$varseg)*), $val));
            }
            $class$method_area
        }
    };

    (@{
        let $varname:tt: $varty:ident$(::$varseg:ident)*;
        $($clsbody:tt)*
    }
    [$($methods:tt)*]{$($post:tt)*}$class:ident$method_area:ident) => {
        $crate::native_class!{@{
                $($clsbody)*
            }
            [$($methods)*]
            {
                $($post)*
                $class.fields.push(($crate::class::Field {
                    access_flags: jvmrs_lib::access!(public native),
                    name: $crate::native_class!(@fn_name $varname).into(),
                    descriptor: $crate::virtual_machine::native::field::Field::as_field($crate::native_class!(@ret $varty$(::$varseg)*)),
                    ..Default::default()
                }, $crate::native_class!(@param_size $varty,)));
            }
            $class$method_area
        }
    };

    (@{}[$($methods:tt)*]{$($post:tt)*}$class:ident$method_area:ident) => {
        $class.register_methods([$($methods)*], $method_area);
        $($post)*
    };

    (@super $($supcls:tt)+) => {$crate::native_class!(@parse_class $($supcls)*).into()};
    (@super) => {"java/lang/Object".into()};

    (@maybe_static) => {jvmrs_lib::access!()};
    (@maybe_static $($t:tt)+) => {jvmrs_lib::access!(static)};

    (@fn_name $name:ident) => {stringify!($name)};
    (@fn_name $name:literal) => {$name};

    (@ret) => {$crate::virtual_machine::native::field::Void};
    (@ret $($pty:tt)*) => {$crate::native_class!(@parse_type $($pty)*)};

    (@return_type ) => {::core::option::Option::None};
    (@return_type $($pty:tt)*) => {
        ::core::option::Option::Some($crate::virtual_machine::native::field::Field::as_field(
            $crate::native_class!(@parse_type $($pty)*),
        ))
    };

    (@any $($any:tt)+) => {1};
    (@any) => {0};

    (@param_size Double, $($rest:tt,)*) => {{
        2 + $crate::native_class!(@param_size $($rest,)*)
    }};
    (@param_size Long, $($rest:tt,)*) => {{
        2 + $crate::native_class!(@param_size $($rest,)*)
    }};
    (@param_size $pty:tt, $($rest:tt,)*) => {{
        1 + $crate::native_class!(@param_size $($rest,)*)
    }};
    (@param_size ) => {0};

    (@parse_type $ty:ident) => {$ty};
    (@parse_type $ty:ident$(::$segment:ident)+) => {
        $crate::virtual_machine::native::field::ObjectField($crate::native_class!(@parse_class $ty$(::$segment)*))
    };

    (@parse_class $ty:ident$(::$segment:ident)+) => {concat!(stringify!($ty), $("/", stringify!($segment),)*)};
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

    class_area.extend(native_class!(
        use method_area;

        impl java::lang::Object {
            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }

            fn getClass(&this) -> java::lang::Class {
                use thread, _verbose;
                let obj_class = AnyObj.inspect(&thread.heap, this as usize, |obj| obj.class.clone())?;
                Ok(get_class(&thread.heap, &thread.class_area, obj_class))
            }

            fn hashCode(&this) -> Int {
                use _thread, _verbose;
                let mut hasher = DefaultHasher::new();
                this.hash(&mut hasher);
                let value = hasher.finish() as i32;
                Ok(value)
            }

            fn toString(&this) -> StringField {
                use _thread, _verbose;
                Ok(Arc::from(format!("{this:0>8X}")))
            }
        }

        impl java::util::Objects {
            fn requireNonNull(object: java::lang::Object) -> java::lang::Object {
                use _thread, _verbose;
                assert!(object != 0);
                Ok(object)
            }
        }

        impl java::lang::StringBuilder {
            fn "<init>"(&this, string: StringField) {
                use thread, _verbose;
                let init_string = string.to_string();
                AnyObj.inspect(&thread.heap, this as usize, |heap_obj| {
                    heap_obj.native_fields.push(Box::new(init_string));
                })
            }

            fn setCharAt(&this, index: Int, character: Char) {
                use thread, verbose;
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
                use thread, _verbose;
                let string = Arc::from(&*StringBuilder::inspect(
                    &thread.heap,
                    this as usize,
                    |a| a.clone(),
                )?);
                Ok(string)
            }
        }

        impl java::util::Random {
            fn "<init>"(&this) {
                use thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |o| {
                    o.native_fields.push(Box::new(rand::make_rng::<StdRng>()));
                })
            }

            fn nextInt(&this, right_bound: Int) -> Int {
                use thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |o| o.native_fields[0].downcast_mut::<StdRng>().unwrap().random_range(0..(right_bound as i32)))
            }
        }

        impl java::lang::Enum {
            let "enum$name": java::lang::String;
            let "enum$id": Int;

            fn "<init>"(&obj_ref, string: java::lang::String, id: Int) {
                use thread, _verbose;
                AnyObj.inspect(&thread.heap, obj_ref as usize, |instance| {
                    instance.fields = vec![string, id as u32];
                })
            }

            fn toString(&this) -> java::lang::String {
                use thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |f| f.fields[0])
            }

            fn name(&this) -> java::lang::String {
                use thread, _verbose;
                AnyObj.inspect(&thread.heap, this as usize, |f| f.fields[0])
            }
        }

        impl java::lang::Math {
            static E: Double = E;
            static PI: Double = PI;
            static TAU: Double = TAU;

            fn sqrt(value: Double) -> Double {
                use _t, _v;
                Ok(value.sqrt())
            }
        }
    ));

    throwable::add_native_methods(method_area, class_area);
    collections::add_native_collections(
        class_area,
        method_area,
        &java_lang_object,
        &java_lang_string,
    );
    reflect::add_native_methods(method_area, class_area);
    function::add_native_methods(
        method_area,
        class_area,
        &java_lang_object,
        &java_lang_string,
    );
    stream::add_native_methods(method_area, class_area, &java_lang_object);
    arrays::add_native_methods(class_area, method_area, &java_lang_object);
    string::add_native_methods(class_area, method_area);
    system::add_native_methods(class_area, method_area);

    drop((java_lang_object, java_lang_string));
}
