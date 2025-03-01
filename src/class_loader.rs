use std::{iter::Peekable, sync::Arc};

use crate::{
    class::{
        code::{
            ByteCode, ExceptionTableEntry, LineTableEntry, LocalVarEntry, LocalVarTypeEntry,
            StackMapFrame, VerificationTypeInfo,
        },
        Attribute, BootstrapMethod, Field, InnerClass,
    },
    data::{SharedClassArea, WorkingClassArea, WorkingMethodArea},
    virtual_machine::{add_native_methods, hydrate_code},
};

mod raw_class;

use jvmrs_lib::{AccessFlags, ClassVersion, Constant, FieldType, MethodDescriptor, MethodHandle};
pub use raw_class::{MethodName, RawClass, RawCode, RawMethod};

// TODO: Attributes: EnclosingMethod, NestHost, NestMembers

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodHandleKind {
    GetField,
    GetStatic,
    PutField,
    PutStatic,
    InvokeVirtual,
    InvokeStatic,
    InvokeSpecial,
    NewInvokeSpecial,
    InvokeInterface,
}

/// A member of the constant pool
#[derive(Debug, Clone)]
pub enum RawConstant {
    String(Arc<str>),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    ClassRef {
        /// index in constant pool for a String value (internally-qualified class name)
        string_addr: u16,
    },
    StringRef {
        /// index in constant pool for a String value
        string_addr: u16,
    },
    FieldRef {
        /// index in constant pool for a `ClassRef` value
        class_ref_addr: u16,
        /// index in constant pool for a `NameTypeDescriptor` value
        name_type_addr: u16,
    },
    MethodRef {
        /// index in constant pool for a `ClassRef` value
        class_ref_addr: u16,
        /// index in constant pool for a `NameTypeDescriptor` value
        name_type_addr: u16,
    },
    InterfaceRef {
        /// index in constant pool for a `ClassRef` value
        class_ref_addr: u16,
        /// index in constant pool for a `NameTypeDescriptor` value
        name_type_addr: u16,
    },
    NameTypeDescriptor {
        /// index in constant pool for a String value for the name
        name_desc_addr: u16,
        /// index in constant pool for a String value - specially encoded type
        type_addr: u16,
    },
    MethodHandle {
        descriptor: MethodHandleKind,
        index: u16,
    },
    MethodType {
        index: u16,
    },
    // Dynamic {
    //     constant: u32,
    // },
    InvokeDynamic {
        bootstrap_index: u16,
        name_type_index: u16,
    },
    // Module {
    //     identity: u16,
    // },
    // Package {
    //     identity: u16,
    // },
    /// Index taken up by the second part of a long or double
    Placeholder,
}

#[must_use]
pub fn load_environment() -> (WorkingMethodArea, WorkingClassArea) {
    let mut method_area = WorkingMethodArea::new();
    let mut class_area = WorkingClassArea::new();
    add_native_methods(&mut method_area, &mut class_area);
    (method_area, class_area)
}

#[allow(clippy::too_many_lines)]
/// # Errors
/// # Panics
pub fn load_class(
    method_area: &mut WorkingMethodArea,
    bytes: &mut impl Iterator<Item = u8>,
    verbose: bool,
) -> Result<RawClass, String> {
    let 0xCAFE_BABE = get_u32(bytes)? else {
        return Err(String::from("Invalid header"));
    };
    let [minor_version, major_version, mut const_count] = get_u16_array(bytes)?;
    let version = ClassVersion {
        minor_version,
        major_version,
    };

    let mut raw_constants = Vec::new();
    while const_count > 1 {
        const_count -= 1;
        match bytes.next() {
            Some(1) => {
                let strlen = get_u16(bytes)?;
                let string = bytes.take(strlen as usize).collect::<Vec<u8>>();
                let string = parse_java_string(string)?;
                raw_constants.push(RawConstant::String(string.into()));
            }
            Some(3) => {
                let bits = get_u32(bytes)? as i32;
                raw_constants.push(RawConstant::Int(bits));
            }
            Some(4) => {
                let bits = f32::from_bits(get_u32(bytes)?);
                raw_constants.push(RawConstant::Float(bits));
            }
            Some(5) => {
                const_count -= 1;
                let bits = get_u64(bytes)? as i64;
                raw_constants.push(RawConstant::Long(bits));
                raw_constants.push(RawConstant::Placeholder);
            }
            Some(6) => {
                const_count -= 1;
                let bits = f64::from_bits(get_u64(bytes)?);
                raw_constants.push(RawConstant::Double(bits));
                raw_constants.push(RawConstant::Placeholder);
            }
            Some(7) => {
                let string_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::ClassRef { string_addr });
            }
            Some(8) => {
                let string_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::StringRef { string_addr });
            }
            Some(9) => {
                let [class_ref_addr, name_type_addr] = get_u16_array(bytes)?;
                raw_constants.push(RawConstant::FieldRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(10) => {
                let [class_ref_addr, name_type_addr] = get_u16_array(bytes)?;
                raw_constants.push(RawConstant::MethodRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(11) => {
                let [class_ref_addr, name_type_addr] = get_u16_array(bytes)?;
                raw_constants.push(RawConstant::InterfaceRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(12) => {
                let [name_desc_addr, type_addr] = get_u16_array(bytes)?;
                raw_constants.push(RawConstant::NameTypeDescriptor {
                    name_desc_addr,
                    type_addr,
                });
            }
            Some(15) => {
                let Some(descriptor) = bytes.next() else {
                    return Err(String::from("Unexpected EOF"));
                };
                let descriptor = match descriptor {
                    1 => MethodHandleKind::GetField,
                    2 => MethodHandleKind::GetStatic,
                    3 => MethodHandleKind::PutField,
                    4 => MethodHandleKind::PutStatic,
                    5 => MethodHandleKind::InvokeVirtual,
                    6 => MethodHandleKind::InvokeStatic,
                    7 => MethodHandleKind::InvokeSpecial,
                    8 => MethodHandleKind::NewInvokeSpecial,
                    9 => MethodHandleKind::InvokeInterface,
                    _ => return Err(format!("Invalid MethodHandleKind: {descriptor}")),
                };
                let index = get_u16(bytes)?;
                raw_constants.push(RawConstant::MethodHandle { descriptor, index });
            }
            Some(16) => {
                let index = get_u16(bytes)?;
                raw_constants.push(RawConstant::MethodType { index });
            }
            Some(18) => {
                let [bootstrap_index, name_type_index] = get_u16_array(bytes)?;
                raw_constants.push(RawConstant::InvokeDynamic {
                    bootstrap_index,
                    name_type_index,
                });
            }
            other => {
                println!("{raw_constants:?}");
                println!("{}", raw_constants.len());
                return Err(format!("Ugh, {other:?}"));
            }
        }
    }

    let constants = raw_constants
        .iter()
        .map(|constant| cook_constant(&raw_constants, constant))
        .collect::<Result<Vec<_>, _>>()?;
    if verbose {
        println!("{constants:?}");
    }

    let [access, this_class, super_class, interface_count] = get_u16_array(bytes)?;

    let access = AccessFlags(access);
    let this_class = raw_class_index(&raw_constants, this_class as usize)?;
    let super_class = raw_class_index(&raw_constants, super_class as usize)?;
    let mut interfaces = Vec::new();
    for _ in 0..interface_count {
        interfaces.push(raw_class_index(&raw_constants, get_u16(bytes)? as usize)?);
    }

    let field_count = get_u16(bytes)?;
    let mut fields = Vec::new();
    for _ in 0..field_count {
        let [access_flags, name_idx, descriptor_idx, attrs_count] = get_u16_array(bytes)?;
        let access_flags = AccessFlags(access_flags);
        let name = raw_str_index(&raw_constants, name_idx as usize)?;
        let descriptor = raw_str_index(&raw_constants, descriptor_idx as usize)?;
        let descriptor = parse_field_type(&mut descriptor.chars().peekable())?;
        let mut attributes = Vec::new();
        for _ in 0..attrs_count {
            attributes.push(get_attribute(&constants, bytes)?);
        }

        let (constant_value, attributes) = single_attribute(attributes, "ConstantValue")?;
        let constant_value = if access_flags.is_static() {
            match constant_value {
                Some(const_idx) => {
                    let [b0, b1] = const_idx[..] else {
                        return Err(String::from(
                            "`ConstantValue` attribute must have exactly two bytes",
                        ));
                    };
                    let Some(constant) = constants.get((b0 as usize) << 8 | b1 as usize) else {
                        return Err(String::from(
                            "`ConstantValue` attribute has invalid constant index",
                        ));
                    };
                    Some(constant.clone())
                }
                None => None,
            }
        } else {
            None
        };

        let (signature, attributes) = get_signature(&constants, attributes)?;

        fields.push(Field {
            access_flags,
            name,
            descriptor,
            constant_value,
            signature,
            attributes,
        });
    }

    let method_count: u16 = get_u16(bytes)?;
    let mut methods = Vec::new();
    // println!("{method_count} methods");
    for _ in 0..method_count {
        let [access_flags, name_idx, descriptor_idx, attrs_count] = get_u16_array(bytes)?;
        let access_flags = AccessFlags(access_flags);
        let name = raw_str_index(&raw_constants, name_idx as usize)?;
        let descriptor = raw_str_index(&raw_constants, descriptor_idx as usize)?;
        let descriptor = parse_method_descriptor(&descriptor)?;
        let mut attributes = Vec::new();
        for _ in 0..attrs_count {
            attributes.push(get_attribute(&constants, bytes)?);
        }

        let (code_attributes, attributes) = single_attribute(attributes, "Code")?;
        // println!("Method {name}: {descriptor}; {attrs_count} attrs");
        // println!("{attributes:?}");
        // println!("{access:?}");
        let code = match code_attributes {
            None if access_flags.is_abstract() => RawCode::Abstract,
            Some(_) if access_flags.is_abstract() => {
                return Err(format!(
                    "Abstract method {descriptor:?} {this_class}.{name} must not contain code"
                ));
            }
            Some(bytes) => RawCode::Code(bytes),
            None => {
                return Err(format!(
                    "Non-Abstract method {descriptor:?} {this_class}.{name} must contain code"
                ))
            }
        };

        let (exceptions, attributes) = single_attribute(attributes, "Exceptions")?;
        let exceptions = match exceptions {
            Some(exceptions) => {
                let mut bytes = exceptions.into_iter().peekable();
                let exc_count = get_u16(&mut bytes)?;
                (0..exc_count)
                    .map(|_| {
                        let idx = get_u16(&mut bytes)?;
                        class_index(&constants, idx as usize)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => Vec::new(),
        };

        let (signature, attributes) = get_signature(&constants, attributes)?;

        methods.push(RawMethod {
            access_flags,
            name,
            exceptions,
            descriptor,
            code,
            signature,
            attributes,
        });
    }

    let attribute_count = get_u16(bytes)?;
    let mut attributes = Vec::new();
    for _ in 0..attribute_count {
        attributes.push(get_attribute(&constants, bytes)?);
    }

    let (bootstrap_methods, attributes) = single_attribute(attributes, "BootstrapMethods")?;

    let bootstrap_methods = match bootstrap_methods {
        Some(bootstrap) => {
            let mut bytes = bootstrap.into_iter().peekable();
            let num_bootstrap_methods = get_u16(&mut bytes)?;
            let mut bootstrap_methods = Vec::new();
            for _ in 0..num_bootstrap_methods {
                let method_ref = get_u16(&mut bytes)?;
                let Constant::MethodHandle(method_handle) =
                    constants[method_ref as usize - 1].clone()
                else {
                    println!("{method_ref}: {:?}", constants[method_ref as usize - 1]);
                    return Err(String::from(
                        "Bootstrap method needs to lead to a MethodHandle",
                    ));
                };
                if verbose {
                    println!("{method_handle:?}");
                }
                let num_args = get_u16(&mut bytes)?;
                let mut args = Vec::new();
                for _ in 0..num_args {
                    let arg_index = get_u16(&mut bytes)?;
                    args.push(constants[arg_index as usize - 1].clone());
                }
                bootstrap_methods.push(BootstrapMethod {
                    method: method_handle,
                    args,
                });
            }
            bootstrap_methods
        }
        None => Vec::new(),
    };

    let (signature, attributes) = get_signature(&constants, attributes)?;

    let (source_file, attributes) = single_attribute(attributes, "SourceFile")?;

    let source_file = match source_file {
        Some(source_file) => Some(str_index(
            &constants,
            get_u16(&mut source_file.into_iter())? as usize,
        )?),
        None => None,
    };

    let (inner_classes, attributes) = single_attribute(attributes, "InnerClasses")?;

    let inner_classes = if let Some(inner_classes) = inner_classes {
        let mut bytes = inner_classes.into_iter();
        let count = get_u16(&mut bytes)?;
        (0..count)
            .map(|_| {
                let [this_idx, outer_idx, name_idx, flags] = get_u16_array(&mut bytes)?;
                let this_idx = this_idx as usize;
                let outer_idx = outer_idx as usize;
                let name_idx = name_idx as usize;
                let flags = AccessFlags(flags);
                Ok(InnerClass {
                    this: if let Constant::ClassRef(class) = constants[this_idx - 1].clone() {
                        class
                    } else {
                        return Err(format!(
                            "Expected class ref for InnerClass.inner_class_info_index; got {:?}",
                            constants[this_idx - 1]
                        ));
                    },
                    outer: if outer_idx == 0 {
                        None
                    } else if let Constant::ClassRef(class) = constants[outer_idx - 1].clone() {
                        Some(class)
                    } else {
                        return Err(format!(
                            "Expected class ref for InnerClass.outer_class_info_index; got {:?}",
                            constants[outer_idx - 1]
                        ));
                    },
                    name: if name_idx == 0 {
                        None
                    } else {
                        Some(str_index(&constants, name_idx)?)
                    },
                    flags,
                })
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    let (nest_host, attributes) = single_attribute(attributes, "NestHost")?;

    let _nest_host = match nest_host {
        Some(vec) => {
            let idx = get_u16(&mut vec.into_iter())?;
            let class = class_index(&constants, idx as usize)?;
            Some(class)
        }
        None => None,
    };

    let (statics, fields): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .partition(|field| field.access_flags.is_static());

    let mut field_size = 0;
    let fields = fields
        .into_iter()
        .map(|field| {
            let field_location = field_size;
            field_size += field.descriptor.get_size();
            (field, field_location)
        })
        .collect();

    let mut statics_size = 0;
    let mut static_data = Vec::new();
    let statics = statics
        .into_iter()
        .map(|field| {
            let field_location = statics_size;
            statics_size += field.descriptor.get_size();
            if let Some(constant) = &field.constant_value {
                // include any constant values
                static_data.extend(constant.bytes());
            } else {
                // put zeroes otherwise
                static_data.extend(std::iter::repeat(0).take(field.descriptor.get_size()));
            }
            (field, field_location)
        })
        .collect();

    if static_data.len() != statics_size {
        return Err(String::from("Static data size error"));
    }

    let class = RawClass {
        constants,
        access,
        this: this_class.clone(),
        super_class,
        interfaces,
        field_size,
        fields,
        statics,
        static_data,
        methods: methods
            .iter()
            .map(|m| MethodName {
                class: this_class.clone(),
                name: m.name.clone(),
                descriptor: m.descriptor.clone(),
            })
            .collect(),
        bootstrap_methods,
        version,
        signature,
        inner_classes,
        source_file,
        attributes,
    };

    for method in methods {
        method_area.push(class.this.clone(), method);
    }
    Ok(class)
}

fn single_attribute(
    attributes: Vec<Attribute>,
    compare: &str,
) -> Result<(Option<Vec<u8>>, Vec<Attribute>), String> {
    let (mut single_attribute, attributes) = split_attributes(attributes, compare);
    match &mut single_attribute[..] {
        [] => Ok((None, attributes)),
        [attr] => Ok((Some(core::mem::take(&mut attr.data)), attributes)),
        _ => Err(format!(
            "A class should have at most one {compare} attribute"
        )),
    }
}

fn split_attributes(attributes: Vec<Attribute>, compare: &str) -> (Vec<Attribute>, Vec<Attribute>) {
    attributes
        .into_iter()
        .partition(|attr| &*attr.name == compare)
}

fn get_signature(
    constants: &[Constant],
    attributes: Vec<Attribute>,
) -> Result<(Option<Arc<str>>, Vec<Attribute>), String> {
    let (signature_attrs, attributes) = single_attribute(attributes, "Signature")?;
    let signature = match signature_attrs {
        Some(signature) => {
            let signature_ref = get_u16(&mut signature.into_iter())? as usize;
            let Constant::String(signature) = constants[signature_ref - 1].clone() else {
                return Err(format!(
                    "Expected string for signature; got {:?}",
                    constants[signature_ref - 1]
                ));
            };
            Some(signature)
        }
        None => None,
    };
    Ok((signature, attributes))
}

fn get_attribute(
    constants: &[Constant],
    bytes: &mut impl Iterator<Item = u8>,
) -> Result<Attribute, String> {
    let name_idx = get_u16(bytes)? as usize;
    let name = str_index(constants, name_idx)
        .map_err(|err| format!("While getting attribute name: {err}"))?;
    let attr_length = get_u32(bytes)? as usize;
    Ok(Attribute {
        name,
        data: bytes.take(attr_length).collect::<Vec<_>>(),
    })
}

#[allow(clippy::too_many_lines)]
fn parse_code_attribute(
    class_area: &SharedClassArea,
    constants: &[Constant],
    bytes: Vec<u8>,
    verbose: bool,
) -> Result<(ByteCode, u16), String> {
    let mut bytes = bytes.into_iter();
    let [max_stack, max_locals] = get_u16_array(&mut bytes)?;
    let code_length = get_u32(&mut bytes)?;
    let code = (&mut bytes).take(code_length as usize).collect::<Vec<_>>();

    let exception_table_length = get_u16(&mut bytes)?;
    let mut exception_table = Vec::new();
    for _ in 0..exception_table_length {
        let [start_pc, end_pc, handler_pc, catch_type] = get_u16_array(&mut bytes)?;
        let catch_type = if catch_type == 0 {
            None
        } else {
            Some(class_index(constants, catch_type as usize)?)
        };
        exception_table.push(ExceptionTableEntry {
            start_pc,
            end_pc,
            handler_pc,
            catch_type,
        });
    }

    if verbose {
        println!("{exception_table:?}");
        println!("Getting attributes...");
    }

    let attrs_count = get_u16(&mut bytes)?;
    let mut attributes = Vec::new();
    for _ in 0..attrs_count {
        attributes.push(get_attribute(constants, &mut bytes)?);
    }

    let (stack_map_attrs, attributes) = single_attribute(attributes, "StackMapTable")?;
    let stack_map = match stack_map_attrs {
        Some(attr) => {
            if verbose {
                println!("Parsing stack map...");
            }
            let mut stack_map = Vec::new();
            let mut bytes = attr.into_iter();
            let frame_count = get_u16(&mut bytes)?;
            for _ in 0..frame_count {
                stack_map.push(match bytes.next() {
                    Some(offset_delta @ 0..=63) => StackMapFrame::Same { offset_delta },
                    Some(offset_delta @ 64..=127) => StackMapFrame::SameLocals1Stack {
                        offset_delta: offset_delta & 0x3F,
                        verification: parse_verification_type(constants, &mut bytes)?,
                    },
                    Some(247) => {
                        let offset_delta = get_u16(&mut bytes)?;
                        StackMapFrame::SameLocals1StackExtended {
                            offset_delta,
                            verification: parse_verification_type(constants, &mut bytes)?,
                        }
                    }
                    Some(chop @ 248..=250) => {
                        let chop = 251 - chop;
                        let offset_delta = get_u16(&mut bytes)?;
                        StackMapFrame::Chop { chop, offset_delta }
                    }
                    Some(251) => {
                        let offset_delta = get_u16(&mut bytes)?;
                        StackMapFrame::SameExtended { offset_delta }
                    }
                    Some(append @ 252..=254) => {
                        let append = append - 251;
                        let offset_delta = get_u16(&mut bytes)?;
                        let mut locals = Vec::new();
                        for _ in 0..append {
                            locals.push(parse_verification_type(constants, &mut bytes)?);
                        }
                        StackMapFrame::Append {
                            offset_delta,
                            locals,
                        }
                    }
                    Some(255) => {
                        let [offset_delta, locals_count] = get_u16_array(&mut bytes)?;
                        let mut locals = Vec::new();
                        for _ in 0..locals_count {
                            locals.push(parse_verification_type(constants, &mut bytes)?);
                        }
                        let stack_count = get_u16(&mut bytes)?;
                        let mut stack = Vec::new();
                        for _ in 0..stack_count {
                            stack.push(parse_verification_type(constants, &mut bytes)?);
                        }
                        StackMapFrame::Full {
                            offset_delta,
                            locals,
                            stack,
                        }
                    }
                    other => return Err(format!("Bad stackmap discriminator; {other:?}")),
                });
            }
            stack_map
        }
        None => Vec::new(),
    };

    let (line_table, attributes) = single_attribute(attributes, "LineNumberTable")?;

    let line_number_table = match line_table {
        Some(line_table) => {
            let mut bytes = line_table.into_iter();
            let table_count = get_u16(&mut bytes)?;
            (0..table_count)
                .map(|_| {
                    let [line, pc] = get_u16_array(&mut bytes)?;
                    Ok::<_, String>(LineTableEntry { line, pc })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        None => Vec::new(),
    };

    let (local_ty_table, attributes) = single_attribute(attributes, "LocalVariableTypeTable")?;

    if verbose {
        println!("Loading LocalVariableTypeTable...");
    }
    let local_type_table = match local_ty_table {
        Some(ty_table) => {
            let mut bytes = ty_table.into_iter();
            let table_count = get_u16(&mut bytes)?;
            (0..table_count)
                .map(|_| {
                    let [pc, length, name_idx, ty_idx, index] = get_u16_array(&mut bytes)?;
                    Ok::<LocalVarTypeEntry, String>(LocalVarTypeEntry {
                        pc,
                        length,
                        name: str_index(constants, name_idx as usize)?,
                        ty: str_index(constants, ty_idx as usize)?,
                        index,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        None => Vec::new(),
    };
    if verbose {
        println!("Loading LocalVariableTable...");
    }
    let (local_var_table, attributes) = single_attribute(attributes, "LocalVariableTable")?;

    let local_var_table = match local_var_table {
        Some(var_table) => {
            let mut bytes = var_table.into_iter();
            let table_count = get_u16(&mut bytes)?;
            (0..table_count)
                .map(|_| {
                    let [pc, length, name_idx, ty_idx, index] = get_u16_array(&mut bytes)?;
                    Ok::<LocalVarEntry, String>(LocalVarEntry {
                        pc,
                        length,
                        name: str_index(constants, name_idx as usize)?,
                        ty: parse_field_type(
                            &mut str_index(constants, ty_idx as usize)?.chars().peekable(),
                        )?,
                        index,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        None => Vec::new(),
    };

    if verbose {
        println!("Hydrating code...");
    }

    let code = hydrate_code(class_area, constants, code, &mut exception_table, verbose)?;

    Ok((
        ByteCode {
            max_stack,
            code,
            exception_table,
            line_number_table,
            local_type_table,
            local_var_table,
            stack_map,
            attributes,
        },
        max_locals,
    ))
}

fn parse_verification_type(
    constants: &[Constant],
    bytes: &mut impl Iterator<Item = u8>,
) -> Result<VerificationTypeInfo, String> {
    match bytes.next() {
        Some(0) => Ok(VerificationTypeInfo::Top),
        Some(1) => Ok(VerificationTypeInfo::Integer),
        Some(2) => Ok(VerificationTypeInfo::Float),
        Some(5) => Ok(VerificationTypeInfo::Null),
        Some(6) => Ok(VerificationTypeInfo::UninitializedThis),
        Some(7) => {
            let index = get_u16(bytes)?;
            let class_name = class_index(constants, index as usize)?;
            Ok(VerificationTypeInfo::Object { class_name })
        }
        Some(8) => {
            let offset = get_u16(bytes)?;
            Ok(VerificationTypeInfo::Uninitialized { offset })
        }
        Some(4) => Ok(VerificationTypeInfo::Long),
        Some(3) => Ok(VerificationTypeInfo::Double),
        other => Err(format!("Invalid verification type info: `{other:?}`")),
    }
}

fn get_bytes<const N: usize>(bytes: &mut impl Iterator<Item = u8>) -> Result<[u8; N], String> {
    <[u8; N]>::try_from(bytes.take(N).collect::<Vec<_>>())
        .map_err(|_| String::from("Unexpected EOF"))
}

fn get_u16(bytes: &mut impl Iterator<Item = u8>) -> Result<u16, String> {
    let bytes = get_bytes::<2>(bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

fn get_u16_array<const N: usize>(bytes: &mut impl Iterator<Item = u8>) -> Result<[u16; N], String> {
    let mut arr = [0; N];
    for i in &mut arr {
        *i = get_u16(bytes)?;
    }
    Ok(arr)
}

fn get_u32(bytes: &mut impl Iterator<Item = u8>) -> Result<u32, String> {
    let bytes = get_bytes::<4>(bytes)?;
    Ok(u32::from_be_bytes(bytes))
}

fn get_u64(bytes: &mut impl Iterator<Item = u8>) -> Result<u64, String> {
    let bytes = get_bytes::<8>(bytes)?;
    Ok(u64::from_be_bytes(bytes))
}

fn raw_str_index(constants: &[RawConstant], idx: usize) -> Result<Arc<str>, String> {
    match constants.get(idx - 1) {
        Some(RawConstant::String(str)) => Ok(str.clone()),
        Some(other) => Err(format!("Expected a string; got `{other:?}`")),
        None => Err(String::from("Constant index out of range")),
    }
}

fn raw_class_index(constants: &[RawConstant], idx: usize) -> Result<Arc<str>, String> {
    match constants.get(idx - 1) {
        Some(RawConstant::ClassRef { string_addr }) => {
            raw_str_index(constants, *string_addr as usize)
        }
        Some(other) => Err(format!("Expected a class reference; got `{other:?}`")),
        None => Err(String::from("Constant index out of range")),
    }
}
fn str_index(constants: &[Constant], idx: usize) -> Result<Arc<str>, String> {
    match constants.get(idx - 1) {
        Some(Constant::String(str)) => Ok(str.clone()),
        Some(other) => Err(format!("Expected a string; got `{other:?}`")),
        None => Err(String::from("Constant index out of range")),
    }
}

fn class_index(constants: &[Constant], idx: usize) -> Result<Arc<str>, String> {
    match constants.get(idx - 1) {
        Some(Constant::ClassRef(str)) => Ok(str.clone()),
        Some(other) => Err(format!("Expected a class reference; got `{other:?}`")),
        None => Err(String::from("Constant index out of range")),
    }
}

fn parse_java_string(bytes: Vec<u8>) -> Result<String, String> {
    let mut bytes = bytes.into_iter();
    let bytes = &mut bytes;
    let mut str = String::new();
    while let Some(b) = bytes.next() {
        if b == 0 {
            return Err(String::from("No byte can have the value zero"));
        } else if b < 128 {
            str.push(b as char);
        } else if b & 0b1110_0000 == 0b1100_0000 {
            let Some(y) = bytes.next() else {
                return Err(String::from("Unexpected end of string"));
            };
            let chr = ((b as u16 & 0x1f) << 6) | (y as u16 & 0x3f);
            str.push(
                char::from_u32(chr as u32).ok_or_else(|| String::from("Invalid character code"))?,
            );
        } else if b == 0b1110_1101 {
            let [v, w, _x, y, z] = get_bytes(bytes)?;
            let chr = 0x10000
                | ((v as u32 & 0x0f) << 16)
                | ((w as u32 & 0x3f) << 10)
                | ((y as u32 & 0x0f) << 6)
                | (z as u32 & 0x3f);
            let chr = char::from_u32(chr).ok_or_else(|| String::from("Invalid character code"))?;
            str.push(chr);
        } else if b & 0b1111_0000 == 0b1110_0000 {
            let Some(y) = bytes.next() else {
                return Err(String::from("Unexpected end of string"));
            };
            let Some(z) = bytes.next() else {
                return Err(String::from("Unexpected end of string"));
            };
            let chr = ((b as u32 & 0xf) << 12) | ((y as u32 & 0x3f) << 6) | (z as u32 & 0x3f);
            let chr = char::from_u32(chr).ok_or_else(|| String::from("Invalid character code"))?;
            str.push(chr);
        }
    }
    Ok(str)
}

#[allow(clippy::too_many_lines)]
fn cook_constant(constants: &[RawConstant], constant: &RawConstant) -> Result<Constant, String> {
    Ok(match constant {
        RawConstant::ClassRef { string_addr } => {
            Constant::ClassRef(raw_str_index(constants, *string_addr as usize)?)
        }
        RawConstant::Double(d) => Constant::Double(*d),
        // RawConstant::Dynamic { constant } => Constant::Dynamic {
        //     constant: *constant,
        // },
        RawConstant::FieldRef {
            class_ref_addr,
            name_type_addr,
        } => {
            let class = raw_class_index(constants, *class_ref_addr as usize)?;
            let (name, field_type) = raw_name_type_index(constants, *name_type_addr as usize)?;
            let field_type = parse_field_type(&mut field_type.chars().peekable())?;
            Constant::FieldRef {
                class,
                name,
                field_type,
            }
        }
        RawConstant::Float(f) => Constant::Float(*f),
        RawConstant::Int(i) => Constant::Int(*i),
        RawConstant::InterfaceRef {
            class_ref_addr,
            name_type_addr,
        } => {
            let class = raw_class_index(constants, *class_ref_addr as usize)?;
            let (name, interface_type) = raw_name_type_index(constants, *name_type_addr as usize)?;
            let interface_type = parse_method_descriptor(&interface_type)?;
            Constant::InterfaceRef {
                class,
                name,
                interface_type,
            }
        }
        &RawConstant::InvokeDynamic {
            bootstrap_index,
            name_type_index: name,
        } => {
            let (method_name, method_type) = raw_name_type_index(constants, name as usize)?;
            let method_type = parse_method_descriptor(&method_type)?;
            Constant::InvokeDynamic {
                bootstrap_index,
                method_name,
                method_type,
            }
        }
        RawConstant::Long(l) => Constant::Long(*l),
        &RawConstant::MethodHandle { descriptor, index } => {
            match (descriptor, &constants[index as usize - 1]) {
                (
                    handle_kind @ (MethodHandleKind::GetField
                    | MethodHandleKind::GetStatic
                    | MethodHandleKind::PutField
                    | MethodHandleKind::PutStatic),
                    RawConstant::FieldRef {
                        class_ref_addr,
                        name_type_addr,
                    },
                ) => {
                    let class = raw_class_index(constants, *class_ref_addr as usize)?;
                    let (name, field_type) =
                        raw_name_type_index(constants, *name_type_addr as usize)?;
                    let field_type = parse_field_type(&mut field_type.chars().peekable())?;
                    Constant::MethodHandle(match handle_kind {
                        MethodHandleKind::GetField => MethodHandle::GetField {
                            class,
                            name,
                            field_type,
                        },
                        MethodHandleKind::GetStatic => MethodHandle::GetStatic {
                            class,
                            name,
                            field_type,
                        },
                        MethodHandleKind::PutField => MethodHandle::PutField {
                            class,
                            name,
                            field_type,
                        },
                        MethodHandleKind::PutStatic => MethodHandle::PutStatic {
                            class,
                            name,
                            field_type,
                        },
                        _ => unreachable!(),
                    })
                }
                (
                    handle_kind
                    @ (MethodHandleKind::InvokeStatic | MethodHandleKind::InvokeSpecial),
                    RawConstant::MethodRef {
                        class_ref_addr,
                        name_type_addr,
                    },
                ) => {
                    let class = raw_class_index(constants, *class_ref_addr as usize)?;
                    let (name, method_type) =
                        raw_name_type_index(constants, *name_type_addr as usize)?;
                    let method_type = parse_method_descriptor(&method_type)?;
                    Constant::MethodHandle(match handle_kind {
                        MethodHandleKind::InvokeStatic => MethodHandle::InvokeStatic {
                            class,
                            name,
                            method_type,
                        },
                        MethodHandleKind::InvokeSpecial => MethodHandle::InvokeSpecial {
                            class,
                            name,
                            method_type,
                        },
                        _ => unreachable!(),
                    })
                }
                (descriptor, constant) => {
                    return Err(format!(
                        "Invalid constant {constant:?} for method handle {descriptor:?}"
                    ))
                }
            }
        }
        RawConstant::MethodRef {
            class_ref_addr,
            name_type_addr,
        } => {
            let class = raw_class_index(constants, *class_ref_addr as usize)?;
            let (name, method_type) = raw_name_type_index(constants, *name_type_addr as usize)?;
            let method_type = parse_method_descriptor(&method_type)?;
            Constant::MethodRef {
                class,
                name,
                method_type,
            }
        }
        &RawConstant::MethodType { index } => {
            let type_descriptor = raw_str_index(constants, index as usize)?;
            Constant::MethodType(parse_method_descriptor(&type_descriptor)?)
        }
        // &RawConstant::Module { identity } => Constant::Module { identity },
        RawConstant::NameTypeDescriptor {
            name_desc_addr,
            type_addr,
        } => {
            let name = raw_str_index(constants, *name_desc_addr as usize)?;
            let type_descriptor = raw_str_index(constants, *type_addr as usize)?;
            Constant::NameTypeDescriptor {
                name,
                type_descriptor,
            }
        }
        // &RawConstant::Package { identity } => Constant::Package { identity },
        RawConstant::String(string) => Constant::String(string.clone()),
        RawConstant::StringRef { string_addr } => {
            let string = raw_str_index(constants, *string_addr as usize)?;
            Constant::StringRef(string)
        }
        RawConstant::Placeholder => Constant::Placeholder,
    })
}

fn raw_name_type_index(
    constants: &[RawConstant],
    idx: usize,
) -> Result<(Arc<str>, Arc<str>), String> {
    let Some(RawConstant::NameTypeDescriptor {
        name_desc_addr,
        type_addr,
    }) = constants.get(idx - 1)
    else {
        return Err(String::from("Invalid NameTypeDescriptor"));
    };
    let name = raw_str_index(constants, *name_desc_addr as usize)?;
    let type_name = raw_str_index(constants, *type_addr as usize)?;
    Ok((name, type_name))
}

/// # Errors
pub fn parse_method_descriptor(src: &str) -> Result<MethodDescriptor, String> {
    let mut chars = src.chars().peekable();
    let chars = &mut chars;
    let Some('(') = chars.next() else {
        return Err(String::from("Expected `(`"));
    };
    let mut parameters = Vec::new();
    while chars.peek() != Some(&')') {
        parameters.push(parse_field_type(chars)?);
    }
    chars.next();
    let return_type = match chars.peek() {
        Some('V') => None,
        _ => Some(parse_field_type(chars)?),
    };
    let parameter_size = parameters.iter().map(FieldType::get_size).sum();
    Ok(MethodDescriptor {
        parameter_size,
        parameters,
        return_type,
    })
}

/// # Panics
/// # Errors
pub fn parse_field_type(
    chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<FieldType, String> {
    match chars.next() {
        Some('B') => Ok(FieldType::Byte),
        Some('C') => Ok(FieldType::Char),
        Some('D') => Ok(FieldType::Double),
        Some('F') => Ok(FieldType::Float),
        Some('I') => Ok(FieldType::Int),
        Some('J') => Ok(FieldType::Long),
        Some('L') => {
            let mut class_buf = String::new();
            while let Some(char) = chars.peek() {
                if char == &';' {
                    chars.next();
                    break;
                }
                class_buf.push(chars.next().unwrap());
            }
            Ok(FieldType::Object(class_buf.into()))
        }
        Some('S') => Ok(FieldType::Short),
        Some('Z') => Ok(FieldType::Boolean),
        Some('[') => Ok(FieldType::Array(Box::new(parse_field_type(chars)?))),
        other => Err(format!("bad field type {other:?}")),
    }
}
