use std::{
    fmt::Debug,
    sync::{Arc, Mutex, Once},
};

use crate::{
    class::{
        code::{ByteCode, NativeMethod, NativeTodo},
        AccessFlags, Attribute, BootstrapMethod, Class, ClassVersion, Code, Constant, Field,
        FieldType, InnerClass, Method, MethodDescriptor,
    },
    data::{SharedClassArea, WorkingClassArea, NULL},
};

use super::parse_code_attribute;

pub struct RawClass {
    /// class version (unused)
    pub version: ClassVersion,
    /// run-time constant pool
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    /// current class name
    pub this: Arc<str>,
    /// super class name
    pub super_class: Arc<str>,
    /// implemented interface names
    pub interfaces: Vec<Arc<str>>,
    /// number of fields
    pub field_size: usize,
    /// name, type, and index of fields
    pub fields: Vec<(Field, usize)>,
    /// static variables
    pub static_data: Vec<u32>,
    /// name, type, and index of statics
    pub statics: Vec<(Field, usize)>,
    /// list of associated methods
    pub methods: Vec<MethodName>,
    pub bootstrap_methods: Vec<BootstrapMethod>,
    pub source_file: Option<Arc<str>>,
    pub signature: Option<Arc<str>>,
    pub inner_classes: Vec<InnerClass>,
    pub attributes: Vec<Attribute>,
}

impl RawClass {
    #[must_use]
    /// Convert to a class that's ready to use in the JVM
    /// # Panics
    pub fn to_class(&self, class_area: &WorkingClassArea) -> Class {
        let mut methods = self.methods.clone();
        let mut fields = self.fields.clone();
        let mut field_size = self.field_size;
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
            for (field, _) in &class_ref.fields {
                fields.push((field.clone(), field_size));
                field_size += field.descriptor.get_size();
            }
            class = class_ref.super_class.clone();
        }
        let initial_fields = self
            .fields
            .iter()
            .flat_map(|(field, _idx)| match &field.descriptor {
                FieldType::Array(_) | FieldType::Object(_) => std::iter::repeat(NULL).take(1),
                other => std::iter::repeat(0).take(other.get_size()),
            })
            .collect::<Vec<_>>();
        // get methods through superclasses
        Class {
            initialized: Once::new(),
            version: self.version,
            constants: self.constants.clone(),
            access: self.access,
            this: self.this.clone(),
            super_class: self.super_class.clone(),
            interfaces: self.interfaces.clone(),
            field_size,
            fields,
            initial_fields,
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
    /// make a new barebones raw class
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

#[derive(Clone)]
pub enum RawCode {
    ByteCode(ByteCode, u16),
    Code(Vec<u8>),
    Native(Arc<Box<dyn NativeMethod>>, u16),
    Abstract,
}

impl RawCode {
    pub fn native(func: impl NativeMethod + 'static) -> Self {
        let args = func.args();
        Self::Native(Arc::new(Box::new(func)), args)
    }
}

#[derive(Clone)]
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
            RawCode::Native(native_method, args) => (Code::Native(native_method.clone()), *args),
            RawCode::Code(code) => {
                if verbose {
                    println!("Cooking method {:?} {}", self.descriptor, self.name);
                }
                let (bytecode, max_locals) =
                    parse_code_attribute(class_area, constants, code.clone(), verbose)?;
                (Code::Code(bytecode), max_locals)
            }
            RawCode::ByteCode(code, locals) => (Code::Code(code.clone()), *locals),
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
