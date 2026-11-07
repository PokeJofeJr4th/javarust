use crate::{
    data::{WorkingClassArea, WorkingMethodArea},
    native_class,
    virtual_machine::{
        native::field::{Boolean, StringField},
        object::ClassObj,
    },
};

pub fn add_native_methods(method_area: &mut WorkingMethodArea, class_area: &mut WorkingClassArea) {
    class_area.extend(native_class! {
        use method_area;

        impl "java/lang/Class" {
            fn getName(&this) -> StringField {
                use thread, _verbose;
                ClassObj::inspect(&thread.heap, this as usize, |c| c.this.clone())
            }

            fn desiredAssertionStatus(&_this) -> Boolean {
                use _thread, _verbose;
                Ok(true)
            }
        }
    });
}
