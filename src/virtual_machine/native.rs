use std::sync::{Arc, Mutex};

use rand::{
    rngs::{StdRng, ThreadRng},
    Rng, SeedableRng,
};

use crate::{
    class::{
        AccessFlags, Class, Code, Field, FieldType, Method, MethodDescriptor, NativeDoubleMethod,
        NativeSingleMethod, NativeStringMethod, NativeTodo, NativeVoid,
    },
    data::{Heap, WorkingClassArea, WorkingMethodArea},
};

use self::{
    arrays::deep_to_string,
    primitives::make_primitives,
    string::{
        native_println_object, native_string_char_at, native_string_len, NativeStringValueOf,
    },
};

use super::{
    object::{Object, ObjectFinder},
    StackFrame, Thread,
};

pub mod arrays;
pub mod primitives;
pub mod string;
pub mod string_builder;

pub static mut OBJECT_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_BUILDER_CLASS: Option<Arc<Class>> = None;
pub static mut ARRAY_CLASS: Option<Arc<Class>> = None;
pub static mut RANDOM_CLASS: Option<Arc<Class>> = None;

#[allow(clippy::too_many_lines)]
/// # Panics
pub fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    heap: &mut Heap,
) {
    let object_name: Arc<str> = Arc::from("java/lang/Object");
    let object_init = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(|_: &mut _, _: &_, _| Ok(()))),
    });
    let object_to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeStringMethod(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let obj_ref = stackframe.lock().unwrap().locals[0];
                // basically random bits
                let fake_addr = 3_141_592u32.wrapping_add(obj_ref);
                Ok(Arc::from(format!("{fake_addr:0>8X}")))
            },
        )),
    });

    let mut object = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        object_name.clone(),
        object_name.clone(),
    );
    object.methods.push(object_init.clone());
    object.methods.push(object_to_string.clone());
    let object = Arc::new(object);

    let array = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/Array".into(),
        object_name.clone(),
    );
    let array = Arc::new(array);

    let arrays_to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Int))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        code: Code::native(NativeStringMethod(arrays::to_string)),
        attributes: Vec::new(),
    });
    let arrays_to_string_obj_arr = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Object(
                "java/lang/Object".into(),
            )))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        code: Code::native(NativeStringMethod(arrays::to_string)),
        attributes: Vec::new(),
    });
    let deep_to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "deepToString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Object(
                object_name.clone(),
            )))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeStringMethod(deep_to_string)),
    });
    let mut arrays = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/util/Arrays".into(),
        object_name.clone(),
    );
    arrays.methods.extend([
        arrays_to_string.clone(),
        arrays_to_string_obj_arr.clone(),
        deep_to_string.clone(),
    ]);
    let array_methods = make_primitives(method_area, class_area, object_name.clone());
    arrays.methods.extend(array_methods.iter().cloned());
    let arrays = Arc::new(arrays);

    let string_length = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "length".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Int),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeSingleMethod(native_string_len)),
    });
    let char_at = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "charAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Char),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeSingleMethod(native_string_char_at)),
    });
    let string_value_of = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object(object_name.clone())],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: Code::native(NativeStringValueOf),
        signature: None,
        attributes: Vec::new(),
    });
    let string_to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: Code::native(NativeSingleMethod(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| Ok(stackframe.lock().unwrap().locals[0]),
        )),
        signature: None,
        attributes: Vec::new(),
    });
    let mut string = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/String".into(),
        object_name.clone(),
    );
    string.methods.extend([
        string_length.clone(),
        char_at.clone(),
        string_value_of.clone(),
        string_to_string.clone(),
    ]);
    let string = Arc::new(string);

    let builder_init = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(string_builder::init)),
    });
    let set_char_at = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "setCharAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Int, FieldType::Char],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(string_builder::set_char_at)),
    });
    let to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeStringMethod(string_builder::to_string)),
    });
    let mut string_builder = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/StringBuilder".into(),
        object_name.clone(),
    );
    string_builder
        .methods
        .extend([builder_init.clone(), set_char_at.clone(), to_string.clone()]);
    let string_builder = Arc::new(string_builder);

    let random_init = Arc::new(Method {
        max_locals: 0,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(
            |thread: &mut Thread, stackframe: &Mutex<StackFrame>, _verbose: bool| {
                let obj = stackframe.lock().unwrap().locals[0];
                unsafe { RANDOM_CLASS.as_ref().unwrap() }.as_ref().get_mut(
                    &thread.heap.lock().unwrap(),
                    obj as usize,
                    |instance| {
                        instance
                            .native_fields
                            .push(Box::new(StdRng::from_entropy()));
                    },
                )
            },
        )),
    });
    let next_int = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "nextInt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Int),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeSingleMethod(
            |thread: &mut Thread, stackframe: &Mutex<StackFrame>, verbose: bool| {
                let obj_ref = stackframe.lock().unwrap().locals[0];
                if verbose {
                    println!("java/util/Random.nextInt(int): obj_ref={obj_ref}");
                }
                let right_bound = stackframe.lock().unwrap().locals[1];
                if verbose {
                    println!("java/util/Random.nextInt(int): right_bound={right_bound}");
                }
                unsafe { RANDOM_CLASS.as_ref().unwrap() }.as_ref().get_mut(
                    &thread.heap.lock().unwrap(),
                    obj_ref as usize,
                    |random_obj| {
                        random_obj.native_fields[0]
                            .downcast_mut::<ThreadRng>()
                            .unwrap()
                            .gen_range(0..right_bound)
                    },
                )
            },
        )),
    });
    let mut random = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/util/Random".into(),
        object_name.clone(),
    );
    random
        .methods
        .extend([random_init.clone(), next_int.clone()]);
    let random = Arc::new(random);

    let println_string = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(native_println_object)),
    });
    let println_object = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/Object".into())],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(native_println_object)),
    });
    let println_char = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Char],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let char = char::from_u32(stackframe.lock().unwrap().locals[1])
                    .ok_or_else(|| String::from("Invalid Character code"))?;
                println!("{char}");
                Ok(())
            },
        )),
    });
    let println_bool = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Boolean],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let bool = stackframe.lock().unwrap().locals[1] != 0;
                println!("{bool}");
                Ok(())
            },
        )),
    });
    let println_int = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let int = stackframe.lock().unwrap().locals[1] as i32;
                println!("{int}");
                Ok(())
            },
        )),
    });
    let println_long = Arc::new(Method {
        max_locals: 3,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Long],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let locals = &stackframe.lock().unwrap().locals;
                let long = ((locals[1] as u64) << 32 | (locals[2] as u64)) as i64;
                println!("{long}");
                Ok(())
            },
        )),
    });
    let println_float = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Float],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let float = f32::from_bits(stackframe.lock().unwrap().locals[1]);
                println!("{float}");
                Ok(())
            },
        )),
    });
    let println_empty = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeVoid(|_: &mut _, _: &_, _| {
            println!();
            Ok(())
        })),
    });
    let mut printstream = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/io/PrintStream".into(),
        object_name.clone(),
    );
    printstream.methods.extend([
        println_string.clone(),
        println_object.clone(),
        println_empty.clone(),
        println_float.clone(),
        println_int.clone(),
        println_bool.clone(),
        println_char.clone(),
        println_long.clone(),
    ]);

    let printstream = Arc::new(printstream);

    let system_out = heap.allocate(Object::from_class(class_area, &printstream));

    let mut system = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/System".into(),
        object_name.clone(),
    );
    system.static_data.lock().unwrap().push(system_out);
    system.statics.push((
        Field {
            access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
            name: "out".into(),
            descriptor: FieldType::Object("java/io/PrintStream".into()),
            attributes: Vec::new(),
            signature: None,
            constant_value: None,
        },
        0,
    ));
    let system = Arc::new(system);

    let make_concat_with_constants = Arc::new(Method {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        max_locals: 5,
        name: "makeConcatWithConstants".into(),
        descriptor: MethodDescriptor {
            parameter_size: 5,
            parameters: vec![
                FieldType::Object("java/lang/invoke/MethodHandles$Lookup".into()),
                FieldType::Object("java/lang/String".into()),
                FieldType::Object("java/lang/invoke/MethodType".into()),
                FieldType::Object("java/lang/String".into()),
                FieldType::Array(Box::new(FieldType::Object("java/lang/Object".into()))),
            ],
            return_type: Some(FieldType::Object("java/lang/invoke/CallSite".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeTodo),
    });

    let mut string_concat_factory = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/StringConcatFactory".into(),
        object_name.clone(),
    );
    string_concat_factory
        .methods
        .push(make_concat_with_constants.clone());
    let string_concat_factory = Arc::new(string_concat_factory);

    let sqrt_double = Arc::new(Method {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "sqrt".into(),
        max_locals: 2,
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Double],
            return_type: Some(FieldType::Double),
        },
        signature: None,
        attributes: Vec::new(),
        code: Code::native(NativeDoubleMethod(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let stackframe = stackframe.lock().unwrap();
                let param = f64::from_bits(
                    (stackframe.locals[0] as u64) << 32 | (stackframe.locals[1] as u64),
                );
                drop(stackframe);
                Ok(param.sqrt().to_bits())
            },
        )),
    });
    let mut math = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/Math".into(),
        object_name,
    );
    math.methods.push(sqrt_double.clone());
    let math = Arc::new(math);

    unsafe {
        OBJECT_CLASS = Some(object.clone());
        STRING_CLASS = Some(string.clone());
        STRING_BUILDER_CLASS = Some(string_builder.clone());
        ARRAY_CLASS = Some(array.clone());
        RANDOM_CLASS = Some(random.clone());
    }
    method_area.extend([
        (object.clone(), object_init),
        (object.clone(), object_to_string),
        (arrays.clone(), arrays_to_string),
        (arrays.clone(), arrays_to_string_obj_arr),
        (arrays.clone(), deep_to_string),
        (string.clone(), string_length),
        (string.clone(), char_at),
        (string.clone(), string_value_of),
        (string.clone(), string_to_string),
        (string_builder.clone(), builder_init),
        (string_builder.clone(), to_string),
        (string_builder.clone(), set_char_at),
        (random.clone(), random_init),
        (random.clone(), next_int),
        (printstream.clone(), println_string),
        (printstream.clone(), println_object),
        (printstream.clone(), println_float),
        (printstream.clone(), println_int),
        (printstream.clone(), println_bool),
        (printstream.clone(), println_char),
        (printstream.clone(), println_long),
        (printstream.clone(), println_empty),
        (string_concat_factory.clone(), make_concat_with_constants),
        (math.clone(), sqrt_double),
    ]);
    method_area.extend(
        array_methods
            .into_iter()
            .map(|method| (arrays.clone(), method)),
    );
    class_area.extend([
        object,
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
}
