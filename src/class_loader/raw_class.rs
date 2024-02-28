use std::{
    fmt::Debug,
    sync::{Arc, Mutex, Once},
};

use crate::{
    class::{
        AccessFlags, Attribute, BootstrapMethod, Class, ClassVersion, Code, Constant, Field,
        InnerClass, Method, MethodDescriptor, NativeMethod, NativeTodo,
    },
    data::{SharedClassArea, WorkingClassArea},
};

use super::parse_code_attribute;

pub struct RawClass {
    pub version: ClassVersion,
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    pub this: Arc<str>,
    pub super_class: Arc<str>,
    pub interfaces: Vec<Arc<str>>,
    pub field_size: usize,
    pub fields: Vec<(Field, usize)>,
    pub static_data: Vec<u32>,
    pub statics: Vec<(Field, usize)>,
    pub methods: Vec<MethodName>,
    pub bootstrap_methods: Vec<BootstrapMethod>,
    pub source_file: Option<Arc<str>>,
    pub signature: Option<Arc<str>>,
    pub inner_classes: Vec<InnerClass>,
    pub attributes: Vec<Attribute>,
}

impl RawClass {
    #[must_use]
    /// # Panics
    pub fn to_class(&self, class_area: &WorkingClassArea) -> Class {
        let mut methods = self.methods.clone();
        let mut class = self.super_class.clone();
        while &*class != "java/lang/Object" {
            let class_ref = class_area.search(&class).expect(&class);
            for method in &class_ref.methods {
                if !methods
                    .iter()
                    .any(|name| name.name == method.name && name.class == method.class)
                {
                    methods.push(method.clone());
                }
            }
            class = class_ref.super_class.clone();
        }
        // get methods through superclasses
        Class {
            initialized: Once::new(),
            version: self.version,
            constants: self.constants.clone(),
            access: self.access,
            this: self.this.clone(),
            super_class: self.super_class.clone(),
            interfaces: self.interfaces.clone(),
            field_size: self.field_size,
            fields: self.fields.clone(),
            static_data: Mutex::new(self.static_data.clone()),
            statics: self.statics.clone(),
            methods,
            bootstrap_methods: self.bootstrap_methods.clone(),
            source_file: self.source_file.clone(),
            signature: self.signature.clone(),
            inner_classes: self.inner_classes.clone(),
            attributes: self.attributes.clone(),
        }
    }

    #[must_use]
    pub fn new(access: AccessFlags, this: Arc<str>, super_class: Arc<str>) -> Self {
        Self {
            version: ClassVersion {
                minor_version: 0,
                major_version: 0,
            },
            constants: Vec::new(),
            access,
            this,
            super_class,
            interfaces: Vec::new(),
            field_size: 0,
            fields: Vec::new(),
            static_data: Vec::new(),
            statics: Vec::new(),
            methods: Vec::new(),
            bootstrap_methods: Vec::new(),
            signature: None,
            source_file: None,
            inner_classes: Vec::new(),
            attributes: Vec::new(),
        }
    }
}

impl Debug for RawClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.access, self.this)?;
        if &*self.super_class != "java/lang/Object" {
            write!(f, " extends {}", self.super_class)?;
        }
        if !self.interfaces.is_empty() {
            write!(f, " implements {}", self.interfaces.join(", "))?;
        }
        let mut s = f.debug_struct("");
        if let Some(signature) = &self.signature {
            s.field("signature", signature);
        }
        s.field("version", &self.version)
            .field("constants", &self.constants)
            .field("field_size", &self.field_size)
            .field("fields", &self.fields)
            .field("static_data", &self.static_data)
            .field("statics", &self.statics)
            .field("methods", &self.methods)
            .field("bootstrap_methods", &self.bootstrap_methods);
        if !self.inner_classes.is_empty() {
            s.field("inner_classes", &self.inner_classes);
        }
        if let Some(source_file) = &self.source_file {
            s.field("source_file", source_file);
        }
        for Attribute { name, data } in &self.attributes {
            s.field(name, &data);
        }
        s.finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MethodName {
    pub class: Arc<str>,
    pub name: Arc<str>,
    pub descriptor: MethodDescriptor,
}

impl Debug for MethodName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {}.{}", self.descriptor, self.class, self.name)
    }
}

pub enum RawCode {
    Code(Vec<u8>),
    Native(Arc<Box<dyn NativeMethod>>),
    Abstract,
}

impl RawCode {
    pub fn native(func: impl NativeMethod + 'static) -> Self {
        Self::Native(Arc::new(Box::new(func)))
    }
}

pub struct RawMethod {
    pub access_flags: AccessFlags,
    pub name: Arc<str>,
    pub exceptions: Vec<Arc<str>>,
    pub descriptor: MethodDescriptor,
    pub code: RawCode,
    pub signature: Option<Arc<str>>,
    pub attributes: Vec<Attribute>,
}

impl RawMethod {
    #[must_use]
    pub fn name(&self, class: Arc<str>) -> MethodName {
        MethodName {
            class,
            name: self.name.clone(),
            descriptor: self.descriptor.clone(),
        }
    }

    /// # Errors
    pub fn cook(
        self,
        class_area: &SharedClassArea,
        constants: &[Constant],
        verbose: bool,
    ) -> Result<Method, String> {
        let (code, max_locals) = match &self.code {
            RawCode::Abstract => (Code::Abstract, self.descriptor.parameter_size as u16),
            RawCode::Native(native_method) => (
                Code::Native(native_method.clone()),
                self.descriptor.parameter_size as u16,
            ),
            RawCode::Code(code) => {
                if verbose {
                    println!("Cooking method {:?} {}", self.descriptor, self.name);
                }
                let (bytecode, max_locals) =
                    parse_code_attribute(class_area, constants, code.clone(), verbose)?;
                (Code::Code(bytecode), max_locals)
            }
        };
        Ok(Method {
            max_locals,
            access_flags: self.access_flags,
            name: self.name,
            exceptions: self.exceptions,
            descriptor: self.descriptor,
            code,
            signature: self.signature,
            attributes: self.attributes,
        })
    }
}

impl Default for RawMethod {
    fn default() -> Self {
        Self {
            access_flags: AccessFlags::ACC_PUBLIC,
            name: "<>".into(),
            exceptions: Vec::new(),
            descriptor: MethodDescriptor {
                parameter_size: 0,
                parameters: Vec::new(),
                return_type: None,
            },
            code: RawCode::native(NativeTodo),
            signature: None,
            attributes: Vec::new(),
        }
    }
}
