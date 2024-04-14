use std::sync::Arc;

use crate::{
    access,
    class::code::NativeTodo,
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    method,
};

pub(super) fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
) {
    let mut stream = RawClass::new(
        access!(public abstract native),
        "java/util/stream/Stream".into(),
        java_lang_object.clone(),
    );

    let stream_next = RawMethod {
        name: "$next".into(),
        access_flags: access!(public abstract native),
        descriptor: method!(() -> Object(java_lang_object.clone())),
        code: RawCode::Abstract,
        ..Default::default()
    };
    let all_match = RawMethod {
        name: "allMatch".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object("java/util/Predicate".into()))) -> boolean),
        code: RawCode::native(NativeTodo),
        ..Default::default()
    };

    let any_match = RawMethod {
        name: "anyMatch".into(),
        access_flags: access!(public native),
        descriptor: method!(((Object("java/util/Predicate".into()))) -> boolean),
        code: RawCode::native(NativeTodo),
        ..Default::default()
    };
    // TODO: collect
    // TODO: concat
    // TODO: count
    // TODO: distinct
    // TODO: dropWhile
    // TODO: Empty
    // TODO: filter
    // TODO: findAny
    // TODO: findFirst
    // TODO: flatMap <toPrimitive>
    // TODO: forEach
    // TODO: forEachOrdered
    // TODO: generate
    // TODO: iterate
    // TODO: limit
    // TODO: map <toPrimitive>
    // TODO: mapMulti <toPrimitive>
    // TODO: max
    // TODO: min
    // TODO: noneMatch
    // TODO: of
    // TODO: ofNullable
    // TODO: peek
    // TODO: reduce
    // TODO: skip
    // TODO: sorted
    // TODO: takeWhile
    // TODO: toArray
    // TODO: toList
    stream.register_methods([all_match, any_match, stream_next], method_area);

    class_area.extend([stream]);
}
