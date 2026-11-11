use jvmrs_lib::method;

use crate::{
    data::{WorkingClassArea, WorkingMethodArea, NULL},
    native_class,
    virtual_machine::{
        native::field::{Boolean, Char, Int, StringField},
        object::{ObjectFinder, StringObj},
    },
};

pub fn add_native_methods(class_area: &mut WorkingClassArea, method_area: &mut WorkingMethodArea) {
    class_area.extend(native_class! {
        use method_area;

        impl java::lang::String {
            fn length(&this) -> Int {
                use thread, _verbose;
                StringObj::inspect(&thread.heap, this as usize, |s| s.len() as i32)
            }

            fn charAt(&this, idx: Int) -> Char {
                use thread, _v;
                StringObj::SELF.inspect(&thread.heap, this as usize, |str| {
                    str.chars().nth(idx as usize).unwrap()
                })
            }

            fn valueOf(obj: java::lang::Object) -> java::lang::String {
                use thread, verbose;
                if obj == NULL {
                    let str_ref = thread.heap.lock().unwrap().allocate_str("null".into());
                    return Ok(str_ref);
                }
                super::resolve_and_invoke(
                    thread,
                    obj,
                    "toString",
                    &method!(() -> Object("java/lang/String".into())),
                    verbose,
                )?;
                thread.stackframe.locals[0] = obj;
                super::run_until_return(thread, verbose)?;
                Ok(thread.stackframe.operand_stack.pop().unwrap())
            }

            fn toString(&this) -> java::lang::String {
                use _t, _v;
                Ok(this)
            }

            fn compareTo(&this, other: StringField) -> Int {
                use thread, _v;
                let this_str = StringObj::inspect(&thread.heap, this as usize, |a| a.clone())?;
                Ok(match this_str.cmp(&other) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                })
            }

            fn contains(&this, seq: java::lang::CharSequence) -> Boolean {
                use thread, _v;
                let this_str = StringObj::inspect(&thread.heap, this as usize, |s| s.clone())?;
                let seq_str = StringObj::inspect(&thread.heap, seq as usize, |s| s.clone())?;
                Ok(this_str.contains(&*seq_str))
            }
        }
    });
}
