use std::{
    any::Any,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use rand::rngs::StdRng;

use crate::{
    access,
    class::{
        code::{NativeMethod, NativeVoid},
        Class, Code, FieldType, Method, MethodDescriptor, MethodHandle,
    },
    class_loader::{RawCode, RawMethod},
    data::{BuildNonHasher, Heap, SharedClassArea, SharedMethodArea, NULL},
    method,
};

use super::{error, native, Thread};

#[derive(Debug)]
pub struct Instance {
    pub fields: Vec<u32>,
    pub native_fields: Vec<Box<dyn Any + Send + Sync>>,
}

impl Instance {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fields: Vec::new(),
            native_fields: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Object {
    pub fields: Vec<u32>,
    pub native_fields: Vec<Box<dyn Any + Send + Sync>>,
    pub class: Arc<str>,
}

impl Object {
    pub fn from_class(class: &Class) -> Self {
        Self {
            fields: class.initial_fields.clone(),
            native_fields: Vec::new(),
            class: class.this.clone(),
        }
    }

    /// # Panics
    #[must_use]
    pub fn resolve_method(
        &self,
        method_area: &SharedMethodArea,
        class_area: &SharedClassArea,
        method: &str,
        descriptor: &MethodDescriptor,
        verbose: bool,
    ) -> (Arc<Class>, Arc<Method>) {
        if verbose {
            println!("Resolving {descriptor:?} {method}");
        }
        let mut current_class = class_area.search(&self.class).unwrap();
        if let Ok(Some(lambda_override)) = LambdaObject::SELF.extract(self, |lambda_override| {
            if &*lambda_override.method_name == method
                && &lambda_override.method_descriptor == descriptor
            {
                Some(lambda_override.as_method())
            } else {
                None
            }
        }) {
            if verbose {
                println!(
                    "Resolved Lambda Override: {:#?}",
                    lambda_override.code.as_native().unwrap()
                );
            }
            return (current_class, Arc::new(lambda_override));
        }
        let mut class_list = vec![current_class.clone()];
        loop {
            if let Some(values) = method_area.search(&current_class.this, method, descriptor) {
                return values;
            }
            if verbose {
                println!("{}.{method} not found", current_class.this);
            }
            if &*current_class.this == "java/lang/Object" {
                if verbose {
                    println!("No method found in superclasses; looking in interfaces");
                }
                break;
            }
            current_class = class_area.search(&current_class.super_class).unwrap();
            class_list.push(current_class.clone());
        }
        for class in class_list {
            for interface in &class.interfaces {
                if let Some(values) = method_area.search(interface, method, descriptor) {
                    if !matches!(values.1.code, Code::Abstract) {
                        return values;
                    }
                }
                if verbose {
                    println!("{interface}.{method} not found");
                }
            }
        }
        panic!("Failed to find any implementation")
    }

    #[must_use]
    pub fn this_class(&self) -> Arc<str> {
        self.class.clone()
    }

    #[must_use]
    /// # Panics
    pub fn isinstance(&self, class_area: &SharedClassArea, class: &str, verbose: bool) -> bool {
        let mut current = class_area.search(class).unwrap();
        if verbose {
            println!("Checking if {} is an instance of {}", current.this, class);
        }
        while &*current.this != "java/lang/Object" {
            if &*current.this == class {
                return true;
            }
            for i in &current.interfaces {
                if verbose {
                    println!("Checking interface {i}");
                }
                if &**i == class {
                    return true;
                }
            }
            if verbose {
                println!("Checking {}", current.super_class);
            }
            current = class_area.search(&current.super_class).unwrap();
        }
        false
    }
}

pub trait ObjectFinder {
    type Target<'a>;
    type TargetMut<'a>;

    /// # Errors
    fn get<T>(
        &self,
        heap: &Heap,
        index: usize,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> error::Result<T> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception").into())
            .and_then(|obj| self.extract(&obj.lock().unwrap(), func))
    }

    /// # Errors
    fn get_mut<T>(
        &self,
        heap: &mut Heap,
        index: usize,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> error::Result<T> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception").into())
            .and_then(|obj| self.extract_mut(&mut obj.lock().unwrap(), func))
    }

    /// # Errors
    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> error::Result<T>;

    /// # Errors
    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> error::Result<T>;
}

pub struct NativeFieldObj<T, const I: usize = 0>(PhantomData<T>);

impl<T: Send + Sync + 'static, const I: usize> NativeFieldObj<T, I> {
    /// Make an initialization method with no parameters for a native field object
    /// # Panics
    pub fn make_init(func: impl Send + Sync + 'static + Fn() -> T) -> RawMethod {
        RawMethod {
            access_flags: access!(public native),
            name: "<init>".into(),
            descriptor: method!(()->void),
            code: RawCode::native(NativeVoid(
                move |thread: &mut Thread, [obj_pointer]: [u32; 1], _verbose| {
                    AnyObj
                        .get_mut(
                            &mut thread.heap.lock().unwrap(),
                            obj_pointer as usize,
                            |instance| {
                                instance.native_fields.push(Box::new(func()));
                            },
                        )
                        .map(Option::Some)
                },
            )),
            ..Default::default()
        }
    }

    #[must_use]
    /// # Panics
    pub fn default_init() -> RawMethod
    where
        T: Default,
    {
        RawMethod {
            access_flags: access!(public native),
            name: "<init>".into(),
            descriptor: method!(()->void),
            code: RawCode::native(NativeVoid(|thread: &mut Thread, [ptr]: [u32; 1], _| {
                AnyObj
                    .get_mut(&mut thread.heap.lock().unwrap(), ptr as usize, |obj| {
                        obj.native_fields.push(Box::<T>::default());
                    })
                    .map(Option::Some)
            })),
            ..Default::default()
        }
    }
}

impl<T, const I: usize> NativeFieldObj<T, I> {
    pub const SELF: Self = Self(PhantomData);
}

impl<E: 'static, const I: usize> NativeFieldObj<E, I> {
    /// # Errors
    pub fn get<O>(
        heap: &Heap,
        index: usize,
        func: impl FnOnce(<Self as ObjectFinder>::Target<'_>) -> O,
    ) -> error::Result<O> {
        Self::SELF.get(heap, index, func)
    }

    /// # Errors
    pub fn get_mut<O>(
        heap: &mut Heap,
        index: usize,
        func: impl FnOnce(<Self as ObjectFinder>::TargetMut<'_>) -> O,
    ) -> error::Result<O> {
        Self::SELF.get_mut(heap, index, func)
    }
}

impl<E: 'static, const I: usize> ObjectFinder for NativeFieldObj<E, I> {
    type Target<'b> = &'b E;

    type TargetMut<'b> = &'b mut E;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> error::Result<T> {
        object
            .native_fields
            .get(I)
            .ok_or_else(|| String::from("Native field is missing"))?
            .downcast_ref::<E>()
            .ok_or_else(|| String::from("Native field is wrong type").into())
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> error::Result<T> {
        object
            .native_fields
            .get_mut(I)
            .ok_or_else(|| String::from("Native field is missing"))?
            .downcast_mut::<E>()
            .ok_or_else(|| String::from("Native field is wrong type").into())
            .map(func)
    }
}

pub type StringObj = NativeFieldObj<Arc<str>>;

impl StringObj {
    #[allow(clippy::new_ret_no_self)]
    /// # Panics
    #[must_use]
    pub fn new(str: Arc<str>) -> Object {
        Object {
            class: unsafe { native::STRING_CLASS.as_ref().unwrap().this.clone() },
            fields: Vec::new(),
            native_fields: vec![Box::new(str)],
        }
    }
}

pub struct AnyObj;

impl ObjectFinder for AnyObj {
    type Target<'a> = &'a Object;
    type TargetMut<'a> = &'a mut Object;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> error::Result<T> {
        Ok(func(object))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> error::Result<T> {
        Ok(func(object))
    }
}

pub type StringBuilder = NativeFieldObj<String>;
pub type ArrayType = NativeFieldObj<FieldType>;
pub type HashMapObj = NativeFieldObj<HashMap<u32, u32, BuildNonHasher>>;
pub type HashSetObj = NativeFieldObj<HashSet<u32, BuildNonHasher>>;
pub type ArrayListObj = NativeFieldObj<Vec<u32>>;
pub type ClassObj = NativeFieldObj<Arc<Class>>;
pub type Random = NativeFieldObj<StdRng>;

impl StringBuilder {
    /// # Panics
    #[must_use]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(str: String, class_area: &SharedClassArea) -> Object {
        let mut obj = Object::from_class(&class_area.search("java/lang/StringBuilder").unwrap());
        obj.native_fields.push(Box::new(str));
        obj
    }
}

pub type LambdaObject = NativeFieldObj<LambdaOverride>;

/// # Lambda Override
/// On a function object created by a lambda expression or method reference, this will be the first native field.
#[derive(Clone, Debug)]
pub struct LambdaOverride {
    /// The name of the method implemented on the interface
    pub method_name: Arc<str>,
    /// The descriptor of the method implemented on the interface
    pub method_descriptor: MethodDescriptor,
    /// the method handle that the lambda invokes internally
    pub invoke: MethodHandle,
    /// any captured values from the lambda's scope
    pub captures: Vec<u32>,
}

impl NativeMethod for LambdaOverride {
    fn args(&self) -> u16 {
        self.method_descriptor.parameter_size as u16 + 1
    }

    fn run(&self, thread: &mut Thread, verbose: bool) -> error::Result<()> {
        match &self.invoke {
            MethodHandle::InvokeStatic {
                class: invoke_class,
                name: invoke_name,
                method_type: invoke_type,
            } => {
                match thread.pc_register {
                    0 => {
                        let (class_ref, method_ref) = thread
                        .method_area
                        .search(invoke_class, invoke_name, invoke_type)
                        .ok_or_else(|| {
                            format!("Error during Lambda Override InvokeStatic; {invoke_class}.{invoke_name}: {invoke_type:?}")
                        })?;

                        if thread.maybe_initialize_class(&class_ref) {
                            return Ok(());
                        }

                        if verbose {
                            println!(
                                "Lambda Override: Invoking Static Method {} on {}",
                                method_ref.name, class_ref.this,
                            );
                        }
                        let combined_args: Vec<u32> = self
                            .captures
                            .iter()
                            .copied()
                            .chain(
                                thread
                                    .stackframe
                                    .locals
                                    .iter()
                                    .skip(1)
                                    .take(self.method_descriptor.parameter_size)
                                    .copied(),
                            )
                            .collect();
                        // push the return address
                        thread.stackframe.operand_stack.push(1);
                        thread.invoke_method(method_ref, class_ref);
                        thread.stackframe.locals = combined_args;
                        if verbose {
                            println!("new locals: {:?}", thread.stackframe.locals);
                        }
                    }
                    1 => match &self.method_descriptor.return_type {
                        None => thread.return_void()?,
                        Some(t) if t.get_size() == 1 => thread.return_one(verbose),
                        _ => thread.return_two(verbose),
                    },
                    o => return Err(format!("Invalid opcode in Lambda Override: {o}").into()),
                }
                Ok(())
            }
            other => Err(format!("Unimplemented Lambda Override: {other:?}").into()),
        }
    }
}

impl LambdaOverride {
    #[must_use]
    pub fn as_method(&self) -> Method {
        Method {
            max_locals: self.method_descriptor.parameter_size as u16 + 1,
            access_flags: access!(),
            name: self.method_name.clone(),
            code: Code::native(self.clone()),
            descriptor: self.method_descriptor.clone(),
            ..Default::default()
        }
    }
}

pub struct ArrayFields<'a, T> {
    pub arr_type: &'a FieldType,
    pub contents: &'a [T],
}

pub struct ArrayFieldsMut<'a, T> {
    pub arr_type: &'a mut FieldType,
    pub contents: &'a mut Vec<T>,
}

pub struct Array1;

impl Array1 {
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new(count: usize, arr_type: FieldType) -> Object {
        let default_value = if matches!(arr_type, FieldType::Object(_)) {
            NULL
        } else {
            0
        };
        Self::from_vec(vec![default_value; count], arr_type)
    }

    #[must_use]
    /// # Panics
    pub fn from_vec(contents: Vec<u32>, arr_type: FieldType) -> Object {
        Object {
            class: unsafe { native::ARRAY_CLASS.as_ref().unwrap().this.clone() },
            fields: Vec::new(),
            native_fields: vec![Box::new(arr_type), Box::new(contents)],
        }
    }
}

impl ObjectFinder for Array1 {
    type Target<'a> = ArrayFields<'a, u32>;
    type TargetMut<'a> = ArrayFieldsMut<'a, u32>;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> error::Result<T> {
        let [arr_type, contents] = &object.native_fields[..] else {
            return Err(String::from("Array class has wrong number of native fields").into());
        };
        let Some(arr_type) = arr_type.downcast_ref::<FieldType>() else {
            return Err(String::from("Native field has the wrong type").into());
        };
        let Some(contents) = contents.downcast_ref::<Vec<u32>>() else {
            return Err(String::from("Native field has the wrong type").into());
        };
        Ok(func(ArrayFields { arr_type, contents }))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> error::Result<T> {
        let [arr_type, contents] = &mut object.native_fields[..] else {
            return Err(String::from("Array class has wrong number of native fields").into());
        };
        let Some(arr_type) = arr_type.downcast_mut::<FieldType>() else {
            return Err(String::from("Native field has the wrong type").into());
        };
        let Some(contents) = contents.downcast_mut::<Vec<u32>>() else {
            return Err(String::from("Native field has the wrong type").into());
        };
        Ok(func(ArrayFieldsMut { arr_type, contents }))
    }
}

pub struct Array2;

impl Array2 {
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new(count: usize, arr_type: FieldType) -> Object {
        Self::from_vec(vec![0u64; count], arr_type)
    }

    #[must_use]
    /// # Panics
    pub fn from_vec(contents: Vec<u64>, arr_type: FieldType) -> Object {
        Object {
            class: unsafe { native::ARRAY_CLASS.as_ref().unwrap().this.clone() },
            fields: Vec::new(),
            native_fields: vec![Box::new(arr_type), Box::new(contents)],
        }
    }
}

impl ObjectFinder for Array2 {
    type Target<'a> = ArrayFields<'a, u64>;
    type TargetMut<'a> = ArrayFieldsMut<'a, u64>;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> error::Result<T> {
        let [arr_type, contents] = &object.native_fields[..] else {
            return Err(String::from("Array class has wrong number of fields").into());
        };
        let Some(arr_type) = arr_type.downcast_ref::<FieldType>() else {
            return Err(String::from("Native field has wrong type").into());
        };
        let Some(contents) = contents.downcast_ref::<Vec<u64>>() else {
            return Err(String::from("Native field has wrong type").into());
        };
        Ok(func(ArrayFields { arr_type, contents }))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> error::Result<T> {
        let [arr_type, contents] = &mut object.native_fields[..] else {
            return Err(String::from("Array object has the wrong number of native fields").into());
        };
        let Some(arr_type) = arr_type.downcast_mut::<FieldType>() else {
            return Err(String::from("Native field is the wrong type").into());
        };
        let Some(contents) = contents.downcast_mut::<Vec<u64>>() else {
            return Err(String::from("Native field is the wrong type").into());
        };
        Ok(func(ArrayFieldsMut { arr_type, contents }))
    }
}
