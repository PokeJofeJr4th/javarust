use std::{
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasher, DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
};

use crate::{
    class::{Class, Method, MethodDescriptor},
    class_loader::{RawClass, RawMethod},
    virtual_machine::object::{Object, StringObj},
};

pub type SharedHeap = Arc<Mutex<Heap>>;

pub struct Heap {
    contents: Vec<Arc<Mutex<Object>>>,
    string_cache: HashMap<Arc<str>, u32>,
}

impl Heap {
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<Arc<Mutex<Object>>> {
        self.contents.get(idx).cloned()
    }

    #[must_use]
    pub fn allocate(&mut self, obj: Object) -> u32 {
        self.contents.push(Arc::new(Mutex::new(obj)));
        (self.contents.len() - 1) as u32
    }

    #[must_use]
    pub fn allocate_str(&mut self, string: Arc<str>) -> u32 {
        if let Some(idx) = self.string_cache.get(&string) {
            return *idx;
        }
        let idx = self.allocate(StringObj::new(string.clone()));
        self.string_cache.insert(string, idx);
        idx
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            string_cache: HashMap::new(),
        }
    }

    #[must_use]
    pub fn make_shared(self) -> SharedHeap {
        Arc::new(Mutex::new(self))
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Heap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.contents
                    .iter()
                    .map(|obj| format!("{:?}", &obj.lock().unwrap())),
            )
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

pub struct MethodArea {
    methods: HashMap<MethodHash, (Arc<Class>, Arc<Method>), BuildNonHasher>,
}

impl MethodArea {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::with_hasher(BuildNonHasher),
        }
    }

    pub fn push(&mut self, class: Arc<Class>, method: Arc<Method>) {
        self.methods.insert(
            hash_method(&class.this, &method.name, &method.descriptor),
            (class, method),
        );
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
            .cloned()
    }
}

impl Debug for MethodArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (class, method) in self.methods.values() {
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
    methods: HashMap<MethodHash, (Arc<str>, RawMethod), BuildNonHasher>,
}

impl WorkingMethodArea {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::with_hasher(BuildNonHasher),
        }
    }

    pub fn push(&mut self, class: Arc<str>, method: RawMethod) {
        self.methods.insert(
            hash_method(&class, &method.name, &method.descriptor),
            (class, method),
        );
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = (Arc<str>, RawMethod)>) {
        self.methods.extend(iter.into_iter().map(|(class, method)| {
            (
                hash_method(&class, &method.name, &method.descriptor),
                (class, method),
            )
        }));
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
                .map(|(h, (class, method))| {
                    let class = class_area
                        .search(&class)
                        .ok_or_else(|| format!("Couldn't find class {class}"))?;
                    let cooked = method.cook(class_area, &class.constants, verbose)?;
                    Ok((h, (class, Arc::new(cooked))))
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
struct BuildNonHasher;

impl BuildHasher for BuildNonHasher {
    type Hasher = NonHasher;

    fn build_hasher(&self) -> Self::Hasher {
        NonHasher(0)
    }
}

struct NonHasher(u64);

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
