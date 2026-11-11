use jvmrs_lib::method;

use crate::{
    data::{NULL, WorkingClassArea, WorkingMethodArea}, native_class, virtual_machine::{
        native::field::{Boolean, Char, Double, Float, Int, Long, StringField},
        object::{Array1, Array2, ArrayType, Object, ObjectFinder as _, StringObj},
    },
};

#[allow(clippy::too_many_lines)]
pub fn add_native_methods(class_area: &mut WorkingClassArea, method_area: &mut WorkingMethodArea) {
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

        impl java::lang::System {
            static out: java::io::PrintStream = NULL;

            fn "<clinit>"() {
                use thread, verbose;
                let system_class = thread.class_area.search("java/lang/System").unwrap();
                let out_ref = thread.heap.lock().unwrap().allocate(Object::from_class(
                    thread.class_area.search("java/io/PrintStream").unwrap(),
                ));
                system_class.static_data.lock().unwrap()[0] = out_ref;
                thread.rember(out_ref, verbose);
                Ok(())
            }

            fn "arraycopy"(src_idx: java::lang::Object, start: Int, dest_idx: java::lang::Object, start_dest: Int, count: Int) {
                use thread, _v;
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
            }
        }
    ));
}
