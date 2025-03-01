use std::{fmt::Display, sync::Arc};

use jvmrs_lib::{access, method, FieldType, MethodDescriptor};

use crate::{
    class::{
        code::{
            native_property, NativeDoubleMethod, NativeMethod, NativeSingleMethod, NativeTodo,
            NativeVoid,
        },
        Field,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    virtual_machine::{
        native::character::Char,
        object::{AnyObj, Object, ObjectFinder, StringObj},
        thread::stacking::Stackable,
        Thread,
    },
};

pub(super) fn make_primitives(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
) -> Vec<RawMethod> {
    vec![
        make_primitive_class::<u8>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Byte,
            "java/lang/Byte".into(),
            "byte",
            |i, _| i as u8,
            NativeSingleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<u8>().unwrap() as u32
            })),
        ),
        make_primitive_class::<i16>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Short,
            "java/lang/Short".into(),
            "short",
            |i, _| i as i16,
            NativeSingleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<i16>().unwrap() as u32
            })),
        ),
        make_primitive_class::<i32>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Int,
            "java/lang/Integer".into(),
            "int",
            |i, _| i as i32,
            NativeSingleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<i32>().unwrap() as u32
            })),
        ),
        make_primitive_class::<i64>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Long,
            "java/lang/Long".into(),
            "long",
            |u, l| (((u as u64) << 32) | (l as u64)) as i64,
            NativeDoubleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<i64>().unwrap() as u64
            })),
        ),
        make_primitive_class::<f32>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Float,
            "java/lang/Float".into(),
            "float",
            |i, _| f32::from_bits(i),
            NativeSingleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<f32>().unwrap().to_bits()
            })),
        ),
        make_primitive_class::<f64>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Double,
            "java/lang/Double".into(),
            "double",
            |u, l| f64::from_bits(((u as u64) << 32) | (l as u64)),
            NativeDoubleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<f64>().unwrap().to_bits()
            })),
        ),
        make_primitive_class::<bool>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Boolean,
            "java/lang/Boolean".into(),
            "boolean",
            |i, _| i != 0,
            NativeSingleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<bool>().unwrap() as u32
            })),
        ),
        make_primitive_class::<Char>(
            method_area,
            class_area,
            object_class,
            FieldType::Float,
            "java/lang/Character".into(),
            "char",
            |i, _| Char(i as u16),
            NativeSingleMethod(native_property(StringObj::SELF, |s| {
                s.parse::<char>().unwrap() as u32
            })),
        ),
    ]
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
fn make_primitive_class<T: Stackable<u32> + Display + 'static>(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
    primitive: FieldType,
    primitive_class: Arc<str>,
    primitive_name: &str,
    from_parameter: fn(u32, u32) -> T,
    parse_fn: impl NativeMethod + 'static,
) -> RawMethod {
    let mut class = RawClass::new(
        access!(public native),
        primitive_class.clone(),
        object_class,
    );
    let primitive_size = primitive.get_size();
    class.fields.push((
        Field {
            access_flags: access!(public native),
            name: "value".into(),
            descriptor: primitive.clone(),
            constant_value: None,
            signature: None,
            attributes: Vec::new(),
        },
        class.field_size,
    ));
    class.field_size += primitive_size;

    let init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1 + primitive_size,
            parameters: vec![primitive.clone()],
            return_type: Some(FieldType::Object(primitive_class.clone())),
        },
        code: RawCode::native(NativeVoid(
            move |thread: &mut Thread, [this, upper, lower]: [u32; 3], _verbose| {
                AnyObj.inspect(&thread.heap, this as usize, |obj| {
                    obj.fields[0] = upper;
                    if primitive_size == 2 {
                        obj.fields[1] = lower;
                    }
                })?;
                Ok(Some(()))
            },
        )),
        ..Default::default()
    };

    let value_of = RawMethod {
        access_flags: access!(public static native),
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive_size,
            parameters: vec![primitive.clone()],
            return_type: Some(FieldType::Object(primitive_class.clone())),
        },
        code: RawCode::native(NativeSingleMethod(
            move |thread: &mut Thread, [upper, lower]: [u32; 2], _verbose| {
                let mut obj =
                    Object::from_class(&thread.class_area.search(&primitive_class).unwrap());
                obj.fields[0] = upper;
                if primitive_size == 2 {
                    obj.fields[1] = lower;
                }
                let ptr = thread.heap.lock().unwrap().allocate(obj);
                Ok(Some(ptr))
            },
        )),
        ..Default::default()
    };
    let to_string = RawMethod::to_string(move |thread: &mut Thread, [this]: [u32; 1], _verbose| {
        AnyObj
            .inspect(&thread.heap, this as usize, |o| {
                format!(
                    "{}",
                    from_parameter(o.fields[0], o.fields.get(1).copied().unwrap_or_default())
                )
                .into()
            })
            .map(Option::Some)
    });
    let primitive_value = RawMethod {
        access_flags: access!(public native),
        name: format!("{primitive_name}Value").into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive_size,
            parameters: Vec::new(),
            return_type: Some(primitive.clone()),
        },
        code: if primitive_size == 2 {
            RawCode::native(NativeDoubleMethod(native_property(AnyObj, |obj| {
                (obj.fields[0] as u64) << 32 | obj.fields[1] as u64
            })))
        } else {
            RawCode::native(NativeSingleMethod(native_property(AnyObj, |obj| {
                obj.fields[0]
            })))
        },
        ..Default::default()
    };
    let parse = RawMethod {
        access_flags: access!(public static native),
        name: format!(
            "parse{}{}",
            &primitive_name[0..1].to_uppercase(),
            &primitive_name[1..]
        )
        .into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: Some(primitive),
        },
        code: RawCode::native(parse_fn),
        ..Default::default()
    };

    class.register_methods(
        [value_of, init, to_string, primitive_value, parse],
        method_area,
    );

    class_area.push(class);

    // this is the array to string
    // `this` = primitive[]
    RawMethod {
        access_flags: access!(public static native),
        name: "toString".into(),
        descriptor: method!(() -> Object("java/lang/String".into())),
        code: RawCode::native(NativeTodo),
        ..Default::default()
    }
}
