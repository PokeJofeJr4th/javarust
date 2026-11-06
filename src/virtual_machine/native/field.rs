use jvmrs_lib::{field, FieldType};

use crate::{
    class::code::{NativeMethod, NativeReturn, NativeSingleMethod, NativeVoid},
    virtual_machine::Thread,
};

pub trait Field {
    type T;

    fn as_field(self) -> FieldType;
    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod;
}

pub struct Void;

impl Field for Void {
    type T = ();

    fn as_field(self) -> FieldType {
        FieldType::Boolean
    }

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod {
        NativeVoid(func)
    }
}

pub struct ArrayField<T>(pub T);

#[derive(Clone, Copy)]
pub struct ObjectField(pub &'static str);

impl Field for ObjectField {
    type T = u32;

    fn as_field(self) -> FieldType {
        FieldType::Object(self.0.into())
    }

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod {
        NativeSingleMethod(func)
    }
}

pub struct Int;

impl Field for Int {
    type T = i32;

    fn as_field(self) -> FieldType {
        field!(int)
    }

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod {
        NativeSingleMethod(move |thread: &mut Thread, args: [u32; U], verbose| {
            func(thread, args, verbose).map(|i| i as u32)
        })
    }
}
