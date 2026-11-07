use std::{println, sync::Arc};

use jvmrs_lib::{access, method, FieldType};

use crate::{
    class::{code::NativeVoid, Field},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    native_class,
    virtual_machine::{
        native::field::{Boolean, Char, Double, Float, Int, Long, StringField},
        object::{Array1, Array2, ArrayType, Object, ObjectFinder as _, StringObj},
        Thread,
    },
};

#[allow(clippy::too_many_lines)]
pub fn add_native_methods(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
) {
    class_area.extend(native_class!(
        use method_area;
        impl java::io::PrintStream {
            fn println(&_this, s: StringField) {
                use _thread, _verbose;
                println!("{s}");
                Ok(())
            }

            fn println(&_this, o: java::lang::Object) {
                use thread, verbose;
                super::resolve_and_invoke(thread,
                    o,
                    "toString",
                    &method!(() -> Object("java/lang/String".into())),
                    verbose,
                )?;
                thread.stackframe.locals[0] = o;
                super::run_until_return(thread, verbose)?;
                let ret = thread.stackframe.operand_stack.pop().unwrap();
                let str = StringObj::SELF.inspect(&thread.heap, ret as usize, |a| a.clone())?;
                println!("{str}");
                Ok(())
            }

            fn println(&_this, c: Char) {
                use _thread, _verbose;
                println!("{c}");
                Ok(())
            }

            fn println(&_this, b: Boolean) {
                use _thread, _verbose;
                println!("{b}");
                Ok(())
            }

            fn println(&_this, i: Int) {
                use _thread, _verbose;
                println!("{i}");
                Ok(())
            }

            fn println(&_this, d: Double) {
                use _thread, _verbose;
                println!("{d}");
                Ok(())
            }

            fn println(&_this, l: Long) {
                use _thread, _verbose;
                println!("{l}");
                Ok(())
            }

            fn println(&_this, f: Float) {
                use _thread, _verbose;
                println!("{f}");
                Ok(())
            }

            fn println(&_this) {
                use _thread, _verbose;
                println!();
                Ok(())
            }
        }
    ));
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
            Ok(())
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
                    Array1.inspect(&thread.heap, dest_idx as usize, |fields| {
                        for (i, value) in copied.into_iter().enumerate() {
                            fields.contents[start_dest as usize + i] = value;
                        }
                    })
                } else {
                    let copied = Array2.inspect(&thread.heap, src_idx as usize, |fields| {
                        fields.contents[(start as usize)..(start + count) as usize].to_vec()
                    })?;
                    Array2.inspect(&thread.heap, dest_idx as usize, |fields| {
                        for (i, value) in copied.into_iter().enumerate() {
                            fields.contents[start_dest as usize + i] = value;
                        }
                    })
                }
            },
        )),
        ..Default::default()
    };
    system.register_method(arraycopy, method_area);

    class_area.extend([system]);
}
