use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use crate::class::Class;

use super::native;

#[derive(Debug)]
pub struct Instance {
    pub fields: Vec<u32>,
    pub native_fields: Vec<Box<dyn Any>>,
}

impl Instance {
    pub const fn new() -> Self {
        Self {
            fields: Vec::new(),
            native_fields: Vec::new(),
        }
    }
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
            .find(|(class_name, _)| &**class_name == &*name)
            .map(|(_, inst)| inst)
    }
}

pub trait ObjectFinder {
    type Target;

    fn get<T>(
        &self,
        heap: &[Arc<Mutex<Object>>],
        index: usize,
        func: impl FnOnce(&Self::Target) -> T,
    ) -> Result<T, String> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception"))
            .and_then(|obj| self.extract(&obj.lock().unwrap(), func))
    }

    fn get_mut<T>(
        &self,
        heap: &[Arc<Mutex<Object>>],
        index: usize,
        func: impl FnOnce(&mut Self::Target) -> T,
    ) -> Result<T, String> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception"))
            .and_then(|obj| self.extract_mut(&mut obj.lock().unwrap(), func))
    }

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(&Self::Target) -> T,
    ) -> Result<T, String>;

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(&mut Self::Target) -> T,
    ) -> Result<T, String>;
}

pub struct StringObj;

impl StringObj {
    pub fn new(str: Arc<str>) -> Object {
        let mut obj = Object::new();
        obj.class_mut_or_insert(&unsafe { native::string_class.clone() }.unwrap())
            .native_fields
            .push(Box::new(str));
        obj
    }
}

impl ObjectFinder for StringObj {
    type Target = Arc<str>;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(&Self::Target) -> T,
    ) -> Result<T, String> {
        unsafe { native::string_class.clone() }
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
        func: impl FnOnce(&mut Self::Target) -> T,
    ) -> Result<T, String> {
        unsafe { native::string_class.clone() }
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
    type Target = Instance;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(&Self::Target) -> T,
    ) -> Result<T, String> {
        todo!()
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(&mut Self::Target) -> T,
    ) -> Result<T, String> {
        Ok(func(object.class_mut_or_insert(self)))
    }
}

pub struct AnyObj;

impl ObjectFinder for AnyObj {
    type Target = Object;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(&Self::Target) -> T,
    ) -> Result<T, String> {
        Ok(func(object))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(&mut Self::Target) -> T,
    ) -> Result<T, String> {
        Ok(func(object))
    }
}

pub struct StringBuilder;

impl ObjectFinder for StringBuilder {
    type Target = String;

    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(&Self::Target) -> T,
    ) -> Result<T, String> {
        unsafe { native::string_builder_class.clone() }
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
        func: impl FnOnce(&mut Self::Target) -> T,
    ) -> Result<T, String> {
        unsafe { native::string_builder_class.clone() }
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
