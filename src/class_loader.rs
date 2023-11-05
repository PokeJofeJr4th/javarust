use crate::class::{AccessFlags, Class, ClassAttributes, Constant};

pub fn load_class(bytes: &mut impl Iterator<Item = u8>) -> Result<Class, String> {
    let [0xCA, 0xFE, 0xBA, 0xBE] = bytes.take(4).collect::<Vec<_>>()[..] else { return Err(String::from("Invalid header")) };
    let attributes = ClassAttributes {
        minor_version: get_u16(bytes)?,
        major_version: get_u16(bytes)?,
    };
    let const_count = get_u16(bytes)?;

    let mut constants = Vec::new();
    for _ in 1..const_count {
        match bytes.next() {
            Some(1) => {
                let strlen = get_u16(bytes)?;
                let string = bytes.take(strlen as usize).collect::<Vec<u8>>();
                let string =
                    String::from_utf8(string).map_err(|err| format!("String Error - {err}"))?;
                constants.push(Constant::String(string));
            }
            Some(3) => {
                let bits = get_u32(bytes)? as i32;
                constants.push(Constant::Int(bits));
            }
            Some(4) => {
                let bits = f32::from_bits(get_u32(bytes)?);
                constants.push(Constant::Float(bits));
            }
            Some(5) => {
                let bits = get_u64(bytes)? as i64;
                constants.push(Constant::Long(bits));
            }
            Some(6) => {
                let bits = f64::from_bits(get_u64(bytes)?);
                constants.push(Constant::Double(bits));
            }
            Some(7) => {
                let string_addr = get_u16(bytes)?;
                constants.push(Constant::ClassRef { string_addr });
            }
            Some(8) => {
                let string_addr = get_u16(bytes)?;
                constants.push(Constant::StringRef { string_addr });
            }
            Some(9) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                constants.push(Constant::FieldRef {
                    class_ref_addr,
                    name_type_addr,
                })
            }
            Some(10) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                constants.push(Constant::MethodRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(11) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                constants.push(Constant::InterfaceRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(12) => {
                let name_desc_addr = get_u16(bytes)?;
                let type_addr = get_u16(bytes)?;
                constants.push(Constant::NameTypeDescriptor {
                    name_desc_addr,
                    type_addr,
                });
            }
            Some(15..=20) => {
                todo!()
            }
            other => return Err(format!("Ugh, {other:?}")),
        }
    }

    let access = AccessFlags(get_u16(bytes)?);
    let this_class = get_u16(bytes)?;
    let Some(Constant::String(this_class)) = constants.get(this_class as usize - 1).cloned() else {
        return Err(format!("Invalid `this` class pointer; {constants:?}[{this_class}]"))
    };

    let super_class = get_u16(bytes)?;
    let Some(Constant::String(super_class)) = constants.get(super_class as usize - 1).cloned() else {
        return Err(String::from("Invalid `super` class pointer"))
    };

    let interface_count = get_u16(bytes)?;
    let mut interfaces = Vec::new();
    for _ in 0..interface_count {
        interfaces.push(get_u16(bytes)?);
    }

    let field_count = get_u16(bytes)?;
    let mut fields = Vec::new();
    for _ in 0..field_count {
        todo!("Get the fields")
    }

    let method_count: u16 = get_u16(bytes)?;
    let mut methods = Vec::new();
    for _ in 0..method_count {
        todo!("Get the methods")
    }

    let attribute_count = get_u16(bytes)?;
    for _ in 0..attribute_count {
        todo!("Get the attributes")
    }

    Ok(Class {
        constants,
        access,
        this: this_class,
        super_class,
        interfaces,
        fields,
        methods,
        attributes,
    })
}

fn get_bytes<const N: usize>(bytes: &mut impl Iterator<Item = u8>) -> Result<[u8; N], String> {
    <[u8; N]>::try_from(bytes.take(N).collect::<Vec<_>>())
        .map_err(|_| String::from("Unexpected EOF"))
}

fn get_u16(bytes: &mut impl Iterator<Item = u8>) -> Result<u16, String> {
    let bytes = get_bytes::<2>(bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

fn get_u32(bytes: &mut impl Iterator<Item = u8>) -> Result<u32, String> {
    let bytes = get_bytes::<4>(bytes)?;
    Ok(u32::from_be_bytes(bytes))
}

fn get_u64(bytes: &mut impl Iterator<Item = u8>) -> Result<u64, String> {
    let bytes = get_bytes::<8>(bytes)?;
    Ok(u64::from_be_bytes(bytes))
}
