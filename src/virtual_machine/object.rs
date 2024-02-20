use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use crate::class::{Class, FieldType};

use super::native;

#[derive(Debug)]
pub struct Instance {
    pub fields: Vec<u32>,
    pub native_fields: Vec<Box<dyn Any>>,
}

#[derive(Debug)]
pub struct Object {
    fields: Vec<(Arc<str>, Instance)>,
}

impl Object {
    pub const fn new() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn class_mut_or_insert(&mut self, class: &Class) -> &mut Instance {
        let name = class.this.clone();
        &mut if self
            .fields
            .iter_mut()
            .any(|(class_name, _)| class_name == &name)
        {
            self.fields
                .iter_mut()
                .find(|(class_name, _)| class_name == &name)
                .unwrap()
        } else {
            let vec = vec![0; class.field_size];
            self.fields.push((
                class.this.clone(),
                Instance {
                    fields: vec,
                    native_fields: Vec::new(),
                },
            ));
            self.fields.last_mut().unwrap()
        }
        .1
    }

    pub fn class(&self, class: &Class) -> Option<&Instance> {
        let name = class.this.clone();
        self.fields
            .iter()
            .find(|(class_name, _)| **class_name == *name)
            .map(|(_, inst)| inst)
    }
}

pub trait ObjectFinder {
    type Target<'a>;
    type TargetMut<'a>;

    fn get<T>(
        &self,
        heap: &[Arc<Mutex<Object>>],
        index: usize,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception"))
            .and_then(|obj| self.extract(&obj.lock().unwrap(), func))
    }

    fn get_mut<T>(
        &self,
        heap: &[Arc<Mutex<Object>>],
        index: usize,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception"))
            .and_then(|obj| self.extract_mut(&mut obj.lock().unwrap(), func))
    }

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String>;

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String>;
}

pub struct StringObj;

impl StringObj {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(str: Arc<str>) -> Object {
        let mut obj = Object::new();
        obj.class_mut_or_insert(&unsafe { native::STRING_CLASS.clone() }.unwrap())
            .native_fields
            .push(Box::new(str));
        obj
    }
}

impl ObjectFinder for StringObj {
    type Target<'a> = &'a Arc<str>;
    type TargetMut<'a> = &'a mut Arc<str>;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::STRING_CLASS.clone() }
            .ok_or_else(|| String::from("String class not found"))
            .and_then(|string_class| {
                object
                    .class(&string_class)
                    .ok_or_else(|| String::from("Object is not an instance of java/lang/String"))?
                    .native_fields
                    .get(0)
                    .ok_or_else(|| String::from("Native string binding missing"))?
                    .downcast_ref::<Arc<str>>()
                    .ok_or_else(|| String::from("Native string binding is the wrong type"))
            })
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::STRING_CLASS.clone() }
            .ok_or_else(|| String::from("String class not found"))
            .and_then(|string_class| {
                object
                    .class_mut_or_insert(&string_class)
                    .native_fields
                    .get_mut(0)
                    .ok_or_else(|| String::from("Native string binding missing"))?
                    .downcast_mut::<Arc<str>>()
                    .ok_or_else(|| String::from("Native string binding is the wrong type"))
            })
            .map(func)
    }
}

impl ObjectFinder for &Class {
    type Target<'a> = &'a Instance;
    type TargetMut<'a> = &'a mut Instance;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        object
            .class(self)
            .ok_or_else(|| format!("Object is not an instance of {}", self.this))
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        Ok(func(object.class_mut_or_insert(self)))
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
    ) -> Result<T, String> {
        Ok(func(object))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        Ok(func(object))
    }
}

pub struct StringBuilder;

impl ObjectFinder for StringBuilder {
    type Target<'a> = &'a String;
    type TargetMut<'a> = &'a mut String;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::STRING_BUILDER_CLASS.clone() }
            .ok_or_else(|| String::from("Couldn't find StringBuilder class"))
            .and_then(|class| {
                object
                    .class(&class)
                    .ok_or_else(|| {
                        String::from("Given object is not an instance of StringBuilder")
                    })?
                    .native_fields
                    .get(0)
                    .ok_or_else(|| String::from("StringBuilder native field missing"))?
                    .downcast_ref::<String>()
                    .ok_or_else(|| String::from("StringBuilder native field is the wrong type"))
            })
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::STRING_BUILDER_CLASS.clone() }
            .ok_or_else(|| String::from("Couldn't find StringBuilder class"))
            .and_then(|class| {
                object
                    .class_mut_or_insert(&class)
                    .native_fields
                    .get_mut(0)
                    .ok_or_else(|| String::from("StringBuilder native field missing"))?
                    .downcast_mut::<String>()
                    .ok_or_else(|| String::from("StringBuilder native field is the wrong type"))
            })
            .map(func)
    }
}

pub struct ArrayType;

impl ObjectFinder for ArrayType {
    type Target<'a> = &'a FieldType;
    type TargetMut<'a> = &'a mut FieldType;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        object
            .class(unsafe { native::ARRAY_CLASS.as_ref() }.unwrap())
            .ok_or_else(|| String::from("Object is not an array"))
            .and_then(|instance| {
                instance
                    .native_fields
                    .get(0)
                    .ok_or_else(|| String::from("Native fields missing"))?
                    .downcast_ref::<FieldType>()
                    .ok_or_else(|| String::from("Native feld is wrong type"))
            })
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        object
            .class_mut_or_insert(unsafe { native::ARRAY_CLASS.as_ref() }.unwrap())
            .native_fields
            .get_mut(0)
            .ok_or_else(|| String::from("Native field is missing"))?
            .downcast_mut::<FieldType>()
            .ok_or_else(|| String::from("Native field is wrong type"))
            .map(func)
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
    pub fn new(count: usize, arr_type: FieldType) -> Object {
        Self::from_vec(vec![0u32; count], arr_type)
    }

    pub fn from_vec(contents: Vec<u32>, arr_type: FieldType) -> Object {
        let mut obj = Object::new();
        let fields = &mut obj
            .class_mut_or_insert(&unsafe { native::ARRAY_CLASS.clone() }.unwrap())
            .native_fields;
        fields.push(Box::new(arr_type));
        fields.push(Box::new(contents));
        obj
    }
}

impl ObjectFinder for Array1 {
    type Target<'a> = ArrayFields<'a, u32>;
    type TargetMut<'a> = ArrayFieldsMut<'a, u32>;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::ARRAY_CLASS.clone() }
            .ok_or_else(|| String::from("Array class isn't defined"))
            .and_then(|array| {
                let [arr_type, contents] = &object
                    .class(&array).ok_or_else(|| String::from("Provided object is not an Array"))?
                    .native_fields[..] else {
                        return Err(String::from("Array class has wrong number of native fields"));
                    };
                let Some(arr_type) = arr_type.downcast_ref::<FieldType>() else {
                    return Err(String::from("Native field has the wrong type"));
                };
                let Some(contents) = contents.downcast_ref::<Vec<u32>>() else {
                    return Err(String::from("Native field has the wrong type"));
                };
                Ok(ArrayFields { arr_type, contents })
            })
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::ARRAY_CLASS.clone() }
            .ok_or_else(|| String::from("Array class isn't defined"))
            .and_then(|array| {
                let [arr_type, contents] = &mut object
                    .class_mut_or_insert(&array)
                    .native_fields[..] else {
                        return Err(String::from("Array class has wrong number of native fields"));
                    };
                let Some(arr_type) = arr_type.downcast_mut::<FieldType>() else {
                    return Err(String::from("Native field has the wrong type"));
                };
                let Some(contents) = contents.downcast_mut::<Vec<u32>>() else {
                    return Err(String::from("Native field has the wrong type"));
                };
                Ok(ArrayFieldsMut { arr_type, contents })
            })
            .map(func)
    }
}

pub struct Array2;

impl Array2 {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(count: usize, arr_type: FieldType) -> Object {
        Self::from_vec(vec![0u64; count], arr_type)
    }

    pub fn from_vec(contents: Vec<u64>, arr_type: FieldType) -> Object {
        let mut obj = Object::new();
        let fields = &mut obj
            .class_mut_or_insert(&unsafe { native::ARRAY_CLASS.clone() }.unwrap())
            .native_fields;
        fields.push(Box::new(arr_type));
        fields.push(Box::new(contents));
        obj
    }
}

impl ObjectFinder for Array2 {
    type Target<'a> = ArrayFields<'a, u64>;
    type TargetMut<'a> = ArrayFieldsMut<'a, u64>;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::ARRAY_CLASS.clone() }
            .ok_or_else(|| String::from("Array class isn't defined"))
            .and_then(|array| {
                let [arr_type, contents] = &object
                    .class(&array)
                    .ok_or_else(|| String::from("Object isn't an array"))?
                    .native_fields[..] else {
                        return Err(String::from("Array class has wrong number of fields"))
                    };
                let Some(arr_type) = arr_type.downcast_ref::<FieldType>() else {
                        return Err(String::from("Native field has wrong type"))
                    };
                let Some(contents) = contents.downcast_ref::<Vec<u64>>() else {
                        return Err(String::from("Native field has wrong type"))
                    };
                Ok(ArrayFields { arr_type, contents })
            })
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        unsafe { native::ARRAY_CLASS.clone() }
            .ok_or_else(|| String::from("Array class isn't defined"))
            .and_then(|array| {
                let [arr_type, contents] = &mut object
                    .class_mut_or_insert(&array)
                    .native_fields[..] else {
                        return Err(String::from("Array object has the wrong number of native fields"))
                    };
                let Some(arr_type) = arr_type.downcast_mut::<FieldType>() else {
                        return Err(String::from("Native field is the wrong type"))
                    };
                let Some(contents) = contents.downcast_mut::<Vec<u64>>() else {
                        return Err(String::from("Native field is the wrong type"))
                    };
                Ok(ArrayFieldsMut { arr_type, contents })
            })
            .map(func)
    }
}
