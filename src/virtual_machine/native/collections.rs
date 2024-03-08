use std::{collections::HashMap, sync::Arc};

use crate::{
    access,
    class::code::NativeVoid,
    class_loader::{RawClass, RawCode, RawMethod},
    data::{BuildNonHasher, WorkingClassArea, WorkingMethodArea},
    method,
    virtual_machine::{
        object::{AnyObj, ObjectFinder},
        Thread,
    },
};

pub fn add_native_collections(
    class_area: &mut WorkingClassArea,
    method_area: &mut WorkingMethodArea,
    java_lang_object: &Arc<str>,
) {
    let mut hash_map = RawClass::new(
        access!(public native),
        "java/util/HashMap".into(),
        java_lang_object.clone(),
    );

    let hash_map_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(()->void),
        code: RawCode::native(NativeVoid(
            |thread: &mut Thread, _: &_, [obj_pointer]: [u32; 1], _| {
                AnyObj
                    .get_mut(
                        &mut thread.heap.lock().unwrap(),
                        obj_pointer as usize,
                        |instance| {
                            instance.native_fields.push(Box::new(HashMap::<
                                u32,
                                u32,
                                BuildNonHasher,
                            >::with_hasher(
                                BuildNonHasher
                            )));
                        },
                    )
                    .map(Option::Some)
            },
        )),
        ..Default::default()
    };
    hash_map
        .methods
        .push(hash_map_init.name(hash_map.this.clone()));

    method_area.extend([(hash_map.this.clone(), hash_map_init)]);
    class_area.extend([hash_map]);
}
