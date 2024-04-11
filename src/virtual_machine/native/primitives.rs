use std::{fmt::Display, sync::Arc};

use crate::{
    access,
    class::{
        code::{NativeSingleMethod, NativeStringMethod, NativeTodo, NativeVoid},
        AccessFlags, Field, FieldType, MethodDescriptor,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    method,
    virtual_machine::{
        native::character::Char,
        object::{AnyObj, Object, ObjectFinder},
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
            "Byte".into(),
            |i, _| i as u8,
        ),
        make_primitive_class::<i16>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Short,
            "Short".into(),
            |i, _| i as i16,
        ),
        make_primitive_class::<i32>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Int,
            "Integer".into(),
            |i, _| i as i32,
        ),
        make_primitive_class::<i64>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Long,
            "Long".into(),
            |u, l| (((u as u64) << 32) | (l as u64)) as i64,
        ),
        make_primitive_class::<f32>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Float,
            "Float".into(),
            |i, _| f32::from_bits(i),
        ),
        make_primitive_class::<f64>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Double,
            "Double".into(),
            |u, l| f64::from_bits(((u as u64) << 32) | (l as u64)),
        ),
        make_primitive_class::<bool>(
            method_area,
            class_area,
            object_class.clone(),
            FieldType::Boolean,
            "Boolean".into(),
            |i, _| i != 0,
        ),
        make_primitive_class::<Char>(
            method_area,
            class_area,
            object_class,
            FieldType::Float,
            "Float".into(),
            |i, _| Char(i as u16),
        ),
    ]
}

#[allow(clippy::too_many_lines)]
fn make_primitive_class<T: Stackable<u32> + Display + 'static>(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
    primitive: FieldType,
    primitive_class: Arc<str>,
    from_parameter: fn(u32, u32) -> T,
) -> RawMethod {
    let mut class = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        primitive_class.clone(),
        object_class,
    );
    let primitive_size = primitive.get_size();
    class.fields.push((
        Field {
            access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
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
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
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
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive_size,
            parameters: vec![primitive],
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
    let to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object("java/lang/String".into())),
        code: RawCode::native(NativeStringMethod(
            move |thread: &mut Thread, [this]: [u32; 1], _verbose| {
                AnyObj
                    .inspect(&thread.heap, this as usize, |o| {
                        format!(
                            "{}",
                            from_parameter(
                                o.fields[0],
                                o.fields.get(1).copied().unwrap_or_default()
                            )
                        )
                        .into()
                    })
                    .map(Option::Some)
            },
        )),
        ..Default::default()
    };

    class.methods.extend([
        value_of.name(class.this.clone()),
        init.name(class.this.clone()),
        to_string.name(class.this.clone()),
    ]);

    method_area.extend([
        (class.this.clone(), value_of),
        (class.this.clone(), init),
        (class.this.clone(), to_string),
    ]);
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
