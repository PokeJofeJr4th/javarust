use std::sync::{Arc, Mutex};

use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::{
    class::{
        AccessFlags, Class, Field, FieldType, MethodDescriptor, NativeDoubleMethod,
        NativeSingleMethod, NativeStringMethod, NativeTodo, NativeVoid,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
};

use self::{
    arrays::deep_to_string,
    primitives::make_primitives,
    string::{
        native_println_object, native_string_char_at, native_string_len, NativeStringValueOf,
    },
};

use super::{object::ObjectFinder, StackFrame, Thread};

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
pub fn add_native_methods(method_area: &mut WorkingMethodArea, class_area: &mut WorkingClassArea) {
    let object_name: Arc<str> = Arc::from("java/lang/Object");
    let object_init = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, _| Ok(()))),
        ..Default::default()
    };
    let object_to_string = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeStringMethod(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let obj_ref = stackframe.lock().unwrap().locals[0];
                // basically random bits
                let fake_addr = 3_141_592u32.wrapping_add(obj_ref);
                Ok(Arc::from(format!("{fake_addr:0>8X}")))
            },
        )),
        ..Default::default()
    };

    let mut object = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        object_name.clone(),
        object_name.clone(),
    );
    object.methods.push(object_init.name(object_name.clone()));
    object
        .methods
        .push(object_to_string.name(object_name.clone()));

    let array = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/Array".into(),
        object_name.clone(),
    );

    let arrays_to_string = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Int))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeStringMethod(arrays::to_string)),
        ..Default::default()
    };
    let arrays_to_string_obj_arr = RawMethod {
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
        code: RawCode::native(NativeStringMethod(arrays::to_string)),
        attributes: Vec::new(),
        ..Default::default()
    };
    let deep_to_string = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "deepToString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Object(
                object_name.clone(),
            )))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeStringMethod(deep_to_string)),
        ..Default::default()
    };
    let mut arrays = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/util/Arrays".into(),
        object_name.clone(),
    );
    arrays.methods.extend([
        arrays_to_string.name(arrays.this.clone()),
        arrays_to_string_obj_arr.name(arrays.this.clone()),
        deep_to_string.name(arrays.this.clone()),
    ]);
    let array_methods = make_primitives(method_area, class_area, object_name.clone());
    arrays.methods.extend(
        array_methods
            .iter()
            .map(|method| method.name(arrays.this.clone())),
    );

    let string_length = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "length".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Int),
        },
        code: RawCode::native(NativeSingleMethod(native_string_len)),
        ..Default::default()
    };
    let char_at = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "charAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Char),
        },
        code: RawCode::native(NativeSingleMethod(native_string_char_at)),
        ..Default::default()
    };
    let string_value_of = RawMethod {
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object(object_name.clone())],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeStringValueOf),
        ..Default::default()
    };
    let string_to_string = RawMethod {
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeSingleMethod(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| Ok(stackframe.lock().unwrap().locals[0]),
        )),
        ..Default::default()
    };
    let mut string = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/String".into(),
        object_name.clone(),
    );
    string.methods.extend([
        string_length.name(string.this.clone()),
        char_at.name(string.this.clone()),
        string_value_of.name(string.this.clone()),
        string_to_string.name(string.this.clone()),
    ]);

    let builder_init = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(string_builder::init)),
        ..Default::default()
    };
    let set_char_at = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "setCharAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Int, FieldType::Char],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(string_builder::set_char_at)),
        ..Default::default()
    };
    let to_string = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeStringMethod(string_builder::to_string)),
        ..Default::default()
    };
    let mut string_builder = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/StringBuilder".into(),
        object_name.clone(),
    );
    string_builder.methods.extend([
        builder_init.name(string_builder.this.clone()),
        set_char_at.name(string_builder.this.clone()),
        to_string.name(string_builder.this.clone()),
    ]);

    let random_init = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        code: RawCode::native(NativeVoid(
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
        ..Default::default()
    };
    let next_int = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "nextInt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Int),
        },
        code: RawCode::native(NativeSingleMethod(
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
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/util/Random".into(),
        object_name.clone(),
    );
    random.methods.extend([
        random_init.name(random.this.clone()),
        next_int.name(random.this.clone()),
    ]);

    let println_string = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(native_println_object)),
        ..Default::default()
    };
    let println_object = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/Object".into())],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(native_println_object)),
        ..Default::default()
    };
    let println_char = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Char],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let char = char::from_u32(stackframe.lock().unwrap().locals[1])
                    .ok_or_else(|| String::from("Invalid Character code"))?;
                println!("{char}");
                Ok(())
            },
        )),
        ..Default::default()
    };
    let println_bool = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Boolean],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let bool = stackframe.lock().unwrap().locals[1] != 0;
                println!("{bool}");
                Ok(())
            },
        )),
        ..Default::default()
    };
    let println_int = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let int = stackframe.lock().unwrap().locals[1] as i32;
                println!("{int}");
                Ok(())
            },
        )),
        ..Default::default()
    };
    let println_long = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Long],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let locals = &stackframe.lock().unwrap().locals;
                let long = ((locals[1] as u64) << 32 | (locals[2] as u64)) as i64;
                println!("{long}");
                Ok(())
            },
        )),
        ..Default::default()
    };
    let println_float = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Float],
            return_type: None,
        },
        code: RawCode::native(NativeVoid(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let float = f32::from_bits(stackframe.lock().unwrap().locals[1]);
                println!("{float}");
                Ok(())
            },
        )),
        ..Default::default()
    };
    let println_empty = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        code: RawCode::native(NativeVoid(|_: &mut _, _: &_, _| {
            println!();
            Ok(())
        })),
        ..Default::default()
    };
    let mut printstream = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/io/PrintStream".into(),
        object_name.clone(),
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
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/System".into(),
        object_name.clone(),
    );
    // system.static_data.lock().unwrap().push(system_out);
    system.static_data.push(u32::MAX);
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

    let make_concat_with_constants = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
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
        code: RawCode::native(NativeTodo),
        ..Default::default()
    };

    let mut string_concat_factory = RawClass::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/StringConcatFactory".into(),
        object_name.clone(),
    );
    string_concat_factory
        .methods
        .push(make_concat_with_constants.name(string_concat_factory.this.clone()));

    let sqrt_double = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "sqrt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Double],
            return_type: Some(FieldType::Double),
        },
        code: RawCode::native(NativeDoubleMethod(
            |_: &mut _, stackframe: &Mutex<StackFrame>, _| {
                let stackframe = stackframe.lock().unwrap();
                let param = f64::from_bits(
                    (stackframe.locals[0] as u64) << 32 | (stackframe.locals[1] as u64),
                );
                drop(stackframe);
                Ok(param.sqrt().to_bits())
            },
        )),
        ..Default::default()
    };
    let mut math = RawClass::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/Math".into(),
        object_name,
    );
    math.methods.push(sqrt_double.name(math.this.clone()));

    // unsafe {
    //     OBJECT_CLASS = Some(object.clone());
    //     STRING_CLASS = Some(string.clone());
    //     STRING_BUILDER_CLASS = Some(string_builder.clone());
    //     ARRAY_CLASS = Some(array.clone());
    //     RANDOM_CLASS = Some(random.clone());
    // }
    method_area.extend([
        (object.this.clone(), object_init),
        (object.this.clone(), object_to_string),
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
