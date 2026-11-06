use std::sync::Arc;

use jvmrs_lib::{field, FieldType};

use crate::{
    class::code::{NativeMethod, NativeReturn, NativeSingleMethod, NativeStringMethod, NativeVoid},
    virtual_machine::{object::StringObj, Thread},
};

pub trait Field {
    type T;

    fn as_field(self) -> FieldType;

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod;

    fn get_parameter(&self, thread: &Thread, idx: usize, args: &[u32]) -> Self::T;
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

    fn get_parameter(&self, _: &Thread, _: usize, _: &[u32]) -> Self::T {}
}

#[derive(Clone, Copy)]
pub struct ArrayField<T>(pub T);

impl<T: Field> Field for ArrayField<T> {
    type T = u32;

    fn as_field(self) -> FieldType {
        FieldType::Array(Box::new(self.0.as_field()))
    }

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod {
        NativeSingleMethod(func)
    }

    fn get_parameter(&self, _: &Thread, idx: usize, args: &[u32]) -> Self::T {
        args[idx]
    }
}

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

    fn get_parameter(&self, _: &Thread, idx: usize, args: &[u32]) -> Self::T {
        args[idx]
    }
}

pub struct Char;

impl Field for Char {
    type T = char;

    fn as_field(self) -> FieldType {
        field!(char)
    }

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod {
        NativeSingleMethod(move |thread: &mut Thread, args: [u32; U], verbose| {
            func(thread, args, verbose).map(|c| c as u32)
        })
    }

    fn get_parameter(&self, _: &Thread, idx: usize, args: &[u32]) -> Self::T {
        char::try_from(args[idx]).unwrap()
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

    fn get_parameter(&self, _: &Thread, idx: usize, args: &[u32]) -> Self::T {
        args[idx] as i32
    }
}

pub struct StringField;

impl Field for StringField {
    type T = Arc<str>;

    fn as_field(self) -> FieldType {
        field!(Object("java/lang/String".into()))
    }

    fn make_native<const U: usize>(
        self,
        func: impl Fn(&mut Thread, [u32; U], bool) -> NativeReturn<Self::T> + Send + Sync + 'static,
    ) -> impl NativeMethod {
        NativeStringMethod(func)
    }

    fn get_parameter(&self, thread: &Thread, idx: usize, args: &[u32]) -> Self::T {
        StringObj::inspect(&thread.heap, args[idx] as usize, |o| o.clone()).unwrap()
    }
}
