use std::{
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasher, DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
};

use crate::{
    class::{Class, FieldType, Method, MethodDescriptor},
    class_loader::{RawClass, RawMethod},
    virtual_machine::object::{Array1, ArrayFields, Object, ObjectFinder, StringObj},
};

pub const NULL: u32 = 0;
pub const HEAP_START: u32 = 0x8000;

pub type SharedHeap = Arc<Mutex<Heap>>;

pub struct Heap {
    contents: Vec<Option<Arc<Mutex<Object>>>>,
    refcounts: Vec<u32>,
    string_cache: HashMap<Arc<str>, u32>,
    string_cache_mirror: HashMap<u32, Arc<str>>,
    class_area: SharedClassArea,
}

impl Heap {
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<Arc<Mutex<Object>>> {
        self.contents.get(idx - HEAP_START as usize)?.clone()
    }

    #[must_use]
    pub fn allocate(&mut self, obj: Object) -> u32 {
        self.contents.push(Some(Arc::new(Mutex::new(obj))));
        self.refcounts.push(0);
        (self.contents.len() - 1 + HEAP_START as usize) as u32
    }

    #[must_use]
    pub fn allocate_str(&mut self, string: Arc<str>) -> u32 {
        if let Some(&idx) = self.string_cache.get(&string) {
            return idx;
        }
        let idx = self.allocate(StringObj::new(string.clone()));
        self.string_cache.insert(string.clone(), idx);
        self.string_cache_mirror.insert(idx, string);
        idx
    }

    #[must_use]
    pub fn new(class_area: SharedClassArea) -> Self {
        Self {
            contents: Vec::new(),
            refcounts: Vec::new(),
            string_cache: HashMap::new(),
            string_cache_mirror: HashMap::new(),
            class_area,
        }
    }

    #[must_use]
    pub fn make_shared(self) -> SharedHeap {
        Arc::new(Mutex::new(self))
    }

    pub fn inc_ref(&mut self, ptr: u32) {
        if ptr == NULL {
            return;
        }
        self.refcounts[(ptr - HEAP_START) as usize] += 1;
    }

    pub fn dec_ref(&mut self, ptr: u32) {
        if ptr == NULL {
            return;
        }
        let idx = (ptr - HEAP_START) as usize;
        self.refcounts[idx] -= 1;
        if self.refcounts[idx] == 0 {
            // println!("Deallocating {idx}");
            self.deallocate(ptr, idx);
        }
    }

    fn deallocate(&mut self, ptr: u32, idx: usize) {
        // get rid of its cached string value
        if let Some(str) = self.string_cache_mirror.remove(&ptr) {
            self.string_cache.remove(&str);
        }
        let Some(obj) = core::mem::take(&mut self.contents[idx]) else {
            return;
        };
        let mut obj = obj.lock().unwrap();
        // deallocate any references that object had within it
        let obj_class = self.class_area.search(&obj.class).unwrap();
        for (field, idx) in &obj_class.fields {
            if field.descriptor.is_reference() {
                self.dec_ref(obj.fields[*idx]);
            }
        }
        // deallocate any references that an array had within it
        if let Ok(Some(contents)) = Array1.extract(&mut obj, |fields: ArrayFields<u32>| {
            if fields.arr_type.is_reference() {
                Some(fields.contents.to_vec())
            } else {
                None
            }
        }) {
            for c in contents {
                self.dec_ref(c);
            }
        }
        drop(obj);
    }
}

impl Debug for Heap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.contents.iter().map(|obj| {
                obj.as_ref()
                    .map_or_else(|| String::from("null"), |obj| format!("{obj:?}"))
            }))
            .finish()
    }
}

#[derive(Clone)]
pub struct SharedClassArea {
    classes: Arc<HashMap<Arc<str>, Arc<Class>>>,
}

impl SharedClassArea {
    #[must_use]
    pub fn search(&self, class: &str) -> Option<Arc<Class>> {
        self.classes.get(class).cloned()
    }
}

/// we have nothing to lose but our chains
pub struct WorkingClassArea {
    classes: HashMap<Arc<str>, RawClass>,
}

impl WorkingClassArea {
    #[must_use]
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    pub fn push(&mut self, class: RawClass) {
        self.classes.insert(class.this.clone(), class);
    }

    #[must_use]
    pub fn to_shared(self) -> SharedClassArea {
        SharedClassArea {
            classes: Arc::from(
                self.classes
                    .iter()
                    .map(|(this, class)| (this.clone(), Arc::new(class.to_class(&self))))
                    .collect::<HashMap<_, _>>(),
            ),
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = RawClass>) {
        self.classes
            .extend(iter.into_iter().map(|c| (c.this.clone(), c)));
    }

    #[must_use]
    pub fn search(&self, class: &str) -> Option<&RawClass> {
        self.classes.get(class)
    }
}

impl Default for WorkingClassArea {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedMethodArea = Arc<MethodArea>;

#[allow(clippy::type_complexity)]
pub struct MethodArea {
    methods: HashMap<MethodHash, Vec<(Arc<Class>, Arc<Method>)>, BuildNonHasher>,
}

impl MethodArea {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::with_hasher(BuildNonHasher),
        }
    }

    pub fn push(&mut self, class: Arc<Class>, method: Arc<Method>) {
        self.methods
            .entry(hash_method(&class.this, &method.name, &method.descriptor))
            .or_default()
            .push((class, method));
    }

    #[must_use]
    pub fn to_shared(self) -> SharedMethodArea {
        Arc::new(self)
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = (Arc<Class>, Arc<Method>)>) {
        for (class, method) in iter {
            self.push(class, method);
        }
    }

    #[must_use]
    pub fn search(
        &self,
        class: &str,
        method: &str,
        method_type: &MethodDescriptor,
    ) -> Option<(Arc<Class>, Arc<Method>)> {
        self.methods
            .get(&hash_method(class, method, method_type))
            .and_then(|vec| {
                vec.iter().find(|(c_class, c_method)| {
                    &*c_class.this == class
                        && &*c_method.name == method
                        && &c_method.descriptor == method_type
                })
            })
            .cloned()
    }
}

impl Debug for MethodArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (class, method) in self.methods.values().flatten() {
            write!(f, "\n{} ", class.this)?;
            method.fmt(f)?;
        }
        Ok(())
    }
}

impl Default for MethodArea {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WorkingMethodArea {
    methods: HashMap<MethodHash, Vec<(Arc<str>, RawMethod)>, BuildNonHasher>,
}

impl WorkingMethodArea {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::with_hasher(BuildNonHasher),
        }
    }

    pub fn push(&mut self, class: Arc<str>, method: RawMethod) {
        self.methods
            .entry(hash_method(&class, &method.name, &method.descriptor))
            .or_default()
            .push((class, method));
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = (Arc<str>, RawMethod)>) {
        for (class, method) in iter {
            self.push(class, method);
        }
    }

    /// # Panics
    /// # Errors
    pub fn to_shared(
        self,
        class_area: &SharedClassArea,
        verbose: bool,
    ) -> Result<SharedMethodArea, String> {
        Ok(MethodArea {
            methods: self
                .methods
                .into_iter()
                .map(|(h, vec)| {
                    let vec = vec
                        .into_iter()
                        .map(|(class, method)| {
                            let class = class_area
                                .search(&class)
                                .ok_or_else(|| format!("Couldn't find class {class}"))?;
                            let cooked = method.cook(class_area, &class.constants, verbose)?;
                            Ok((class, Arc::new(cooked)))
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok((h, vec))
                })
                .collect::<Result<_, String>>()?,
        }
        .to_shared())
    }
}

impl Default for WorkingMethodArea {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Clone, Copy)]
pub struct BuildNonHasher;

impl BuildHasher for BuildNonHasher {
    type Hasher = NonHasher;

    fn build_hasher(&self) -> Self::Hasher {
        NonHasher(0)
    }
}

pub struct NonHasher(u64);

impl Hasher for NonHasher {
    fn write(&mut self, _bytes: &[u8]) {
        panic!("NonHasher should only be used to not hash a u64")
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct MethodHash(u64);

fn hash_method(class: &str, name: &str, signature: &MethodDescriptor) -> MethodHash {
    let mut state = DefaultHasher::new();
    class.hash(&mut state);
    name.hash(&mut state);
    signature.hash(&mut state);
    MethodHash(state.finish())
}

fn custom_hash(class: &str, name: &str, signature: &MethodDescriptor) -> MethodHash {
    let mut state: u64 = 0xCAFE_BABE;
    for c in class.bytes().rev().take(5) {
        state = state.rotate_left(17) ^ c as u64;
    }
    for c in name.bytes().rev().take(5) {
        state = state.rotate_left(15) ^ c as u64;
    }
    for p in signature.parameters.iter().chain(&signature.return_type) {
        state = state.rotate_left(9) ^ p.idx();
        match p {
            FieldType::Array(arr) => {
                state = state.rotate_left(11) ^ arr.idx();
            }
            FieldType::Object(obj) => {
                for c in obj.bytes().rev().take(5) {
                    state = state.rotate_right(19) ^ c as u64;
                }
            }
            _ => {}
        }
    }
    MethodHash(state)
}
