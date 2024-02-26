use std::{any::Any, sync::Arc};

use crate::{
    class::{Class, Code, FieldType, Method, MethodDescriptor},
    data::{Heap, SharedClassArea, SharedMethodArea},
};

use super::native;

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
    fields: Instance,
    class: Arc<str>,
    super_object: Option<Box<Object>>,
}

impl Object {
    /// # Panics
    pub fn from_class(class_area: &SharedClassArea, class: &Class) -> Self {
        Self {
            fields: Instance {
                fields: class
                    .fields
                    .iter()
                    .flat_map(|(field, _idx)| match &field.descriptor {
                        FieldType::Array(_) | FieldType::Object(_) => {
                            std::iter::repeat(u32::MAX).take(1)
                        }
                        other => std::iter::repeat(0).take(other.get_size()),
                    })
                    .collect::<Vec<_>>(),
                native_fields: Vec::new(),
            },
            class: class.this.clone(),
            super_object: if &*class.super_class == "java/lang/Object" {
                None
            } else {
                Some(Box::new(Self::from_class(
                    class_area,
                    &class_area.search(&class.super_class).unwrap(),
                )))
            },
        }
    }

    /// # Panics
    pub fn with_fields(class_area: &SharedClassArea, class: &Class, fields: Instance) -> Self {
        Self {
            fields,
            class: class.this.clone(),
            super_object: if &*class.super_class == "java/lang/Object" {
                None
            } else {
                Some(Box::new(Self::from_class(
                    class_area,
                    &class_area.search(&class.super_class).unwrap(),
                )))
            },
        }
    }

    /// Create a class with Object as its superclass
    /// # Panics
    pub fn orphan_with_fields(class: &Class, fields: Instance) -> Self {
        Self {
            fields,
            class: class.this.clone(),
            super_object: Some(Box::new(Self {
                fields: Instance::new(),
                class: "java/lang/Object".into(),
                super_object: None,
            })),
        }
    }

    pub fn class_mut(&mut self, class: &str) -> Option<&mut Instance> {
        if &*self.class == class {
            Some(&mut self.fields)
        } else {
            self.super_object.as_mut()?.class_mut(class)
        }
    }

    #[must_use]
    pub fn class(&self, class: &str) -> Option<&Instance> {
        if &*self.class == class {
            Some(&self.fields)
        } else {
            self.super_object.as_ref()?.class(class)
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
    ) -> Result<T, String> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception"))
            .and_then(|obj| self.extract(&obj.lock().unwrap(), func))
    }

    /// # Errors
    fn get_mut<T>(
        &self,
        heap: &Heap,
        index: usize,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        heap.get(index)
            .ok_or_else(|| String::from("Null pointer exception"))
            .and_then(|obj| self.extract_mut(&mut obj.lock().unwrap(), func))
    }

    /// # Errors
    fn extract<T>(
        &self,
        object: &Object,
        func: impl FnOnce(Self::Target<'_>) -> T,
    ) -> Result<T, String>;

    /// # Errors
    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String>;
}

pub struct StringObj;

impl StringObj {
    #[allow(clippy::new_ret_no_self)]
    /// # Panics
    #[must_use]
    pub fn new(str: Arc<str>) -> Object {
        Object::orphan_with_fields(
            unsafe { native::STRING_CLASS.as_ref().unwrap() },
            Instance {
                fields: Vec::new(),
                native_fields: vec![Box::new(str)],
            },
        )
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
        object
            .class(unsafe { &native::STRING_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| String::from("Object is not an instance of java/lang/String"))?
            .native_fields
            .first()
            .ok_or_else(|| String::from("Native string binding missing"))?
            .downcast_ref::<Arc<str>>()
            .ok_or_else(|| String::from("Native string binding is the wrong type"))
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        object
            .class_mut(unsafe { &native::STRING_CLASS.as_ref().unwrap().this })
            .unwrap()
            .native_fields
            .get_mut(0)
            .ok_or_else(|| String::from("Native string binding missing"))?
            .downcast_mut::<Arc<str>>()
            .ok_or_else(|| String::from("Native string binding is the wrong type"))
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
            .class(&self.this)
            .ok_or_else(|| format!("Expected a(n) {}; got a(n) {}", self.this, object.class))
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        Ok(func(
            object
                .class_mut(&self.this)
                .ok_or_else(|| format!("Expected a(n) {}", self.this))?,
        ))
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
        object
            .class(unsafe { &native::STRING_BUILDER_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| String::from("Given object is not an instance of StringBuilder"))?
            .native_fields
            .first()
            .ok_or_else(|| String::from("StringBuilder native field missing"))?
            .downcast_ref::<String>()
            .ok_or_else(|| String::from("StringBuilder native field is the wrong type"))
            .map(func)
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        object
            .class_mut(unsafe { &native::STRING_BUILDER_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| "Expected a java/lang/StringBuilder".to_string())?
            .native_fields
            .get_mut(0)
            .ok_or_else(|| String::from("StringBuilder native field missing"))?
            .downcast_mut::<String>()
            .ok_or_else(|| String::from("StringBuilder native field is the wrong type"))
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
            .class(unsafe { &native::ARRAY_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| String::from("Object is not an array"))
            .and_then(|instance| {
                instance
                    .native_fields
                    .first()
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
            .class_mut(unsafe { &native::ARRAY_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| "Expected an array".to_string())?
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
    #[must_use]
    pub fn new(class_area: &SharedClassArea, count: usize, arr_type: FieldType) -> Object {
        let default_value = if matches!(arr_type, FieldType::Object(_)) {
            u32::MAX
        } else {
            0
        };
        Self::from_vec(class_area, vec![default_value; count], arr_type)
    }

    #[must_use]
    /// # Panics
    pub fn from_vec(
        class_area: &SharedClassArea,
        contents: Vec<u32>,
        arr_type: FieldType,
    ) -> Object {
        Object::with_fields(
            class_area,
            unsafe { native::ARRAY_CLASS.as_ref().unwrap() },
            Instance {
                fields: Vec::new(),
                native_fields: vec![Box::new(arr_type), Box::new(contents)],
            },
        )
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
        let [arr_type, contents] = &object
            .class(unsafe { &native::ARRAY_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| String::from("Provided object is not an Array"))?
            .native_fields[..]
        else {
            return Err(String::from(
                "Array class has wrong number of native fields",
            ));
        };
        let Some(arr_type) = arr_type.downcast_ref::<FieldType>() else {
            return Err(String::from("Native field has the wrong type"));
        };
        let Some(contents) = contents.downcast_ref::<Vec<u32>>() else {
            return Err(String::from("Native field has the wrong type"));
        };
        Ok(func(ArrayFields { arr_type, contents }))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        let [arr_type, contents] = &mut object
            .class_mut(unsafe { &native::ARRAY_CLASS.as_ref().unwrap().this })
            .unwrap()
            .native_fields[..]
        else {
            return Err(String::from(
                "Array class has wrong number of native fields",
            ));
        };
        let Some(arr_type) = arr_type.downcast_mut::<FieldType>() else {
            return Err(String::from("Native field has the wrong type"));
        };
        let Some(contents) = contents.downcast_mut::<Vec<u32>>() else {
            return Err(String::from("Native field has the wrong type"));
        };
        Ok(func(ArrayFieldsMut { arr_type, contents }))
    }
}

pub struct Array2;

impl Array2 {
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new(class_area: &SharedClassArea, count: usize, arr_type: FieldType) -> Object {
        Self::from_vec(class_area, vec![0u64; count], arr_type)
    }

    #[must_use]
    /// # Panics
    pub fn from_vec(
        class_area: &SharedClassArea,
        contents: Vec<u64>,
        arr_type: FieldType,
    ) -> Object {
        Object::with_fields(
            class_area,
            unsafe { native::ARRAY_CLASS.as_ref().unwrap() },
            Instance {
                fields: Vec::new(),
                native_fields: vec![Box::new(arr_type), Box::new(contents)],
            },
        )
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
        let [arr_type, contents] = &object
            .class(unsafe { &native::ARRAY_CLASS.as_ref().unwrap().this })
            .ok_or_else(|| String::from("Object isn't an array"))?
            .native_fields[..]
        else {
            return Err(String::from("Array class has wrong number of fields"));
        };
        let Some(arr_type) = arr_type.downcast_ref::<FieldType>() else {
            return Err(String::from("Native field has wrong type"));
        };
        let Some(contents) = contents.downcast_ref::<Vec<u64>>() else {
            return Err(String::from("Native field has wrong type"));
        };
        Ok(func(ArrayFields { arr_type, contents }))
    }

    fn extract_mut<T>(
        &self,
        object: &mut Object,
        func: impl FnOnce(Self::TargetMut<'_>) -> T,
    ) -> Result<T, String> {
        let [arr_type, contents] = &mut object
            .class_mut(unsafe { &native::ARRAY_CLASS.as_ref().unwrap().this })
            .unwrap()
            .native_fields[..]
        else {
            return Err(String::from(
                "Array object has the wrong number of native fields",
            ));
        };
        let Some(arr_type) = arr_type.downcast_mut::<FieldType>() else {
            return Err(String::from("Native field is the wrong type"));
        };
        let Some(contents) = contents.downcast_mut::<Vec<u64>>() else {
            return Err(String::from("Native field is the wrong type"));
        };
        Ok(func(ArrayFieldsMut { arr_type, contents }))
    }
}
