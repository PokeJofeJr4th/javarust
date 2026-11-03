use std::sync::Arc;

use jvmrs_lib::{access, method, FieldType};

use crate::{
    class::{code::NativeVoid, Field},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    virtual_machine::{
        native::string::native_println_object,
        object::{Array1, Array2, ArrayType, Object, ObjectFinder as _},
        Thread,
    },
};

pub fn add_native_methods(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
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
                thread.class_area.search("java/io/PrintStream").unwrap(),
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

    class_area.extend([system, printstream]);
}
