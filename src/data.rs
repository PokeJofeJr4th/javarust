use std::{
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasher, DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
};

use crate::{
    class::{Class, Method, MethodDescriptor},
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

pub trait ClassArea {
    fn search(&self, class: &str) -> Option<Arc<Class>>;
}

#[derive(Clone)]
pub struct SharedClassArea {
    classes: Arc<HashMap<Arc<str>, Arc<Class>>>,
}

impl ClassArea for SharedClassArea {
    fn search(&self, class: &str) -> Option<Arc<Class>> {
        self.classes.get(class).cloned()
    }
}

pub struct WorkingClassArea {
    classes: Vec<Arc<Class>>,
}

impl WorkingClassArea {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            classes: Vec::new(),
        }
    }

    pub fn push(&mut self, class: Arc<Class>) {
        self.classes.push(class);
    }

    #[must_use]
    pub fn to_shared(self) -> SharedClassArea {
        SharedClassArea {
            classes: Arc::from(
                self.classes
                    .into_iter()
                    .map(|class| (class.this.clone(), class))
                    .collect::<HashMap<_, _>>(),
            ),
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = Arc<Class>>) {
        self.classes.extend(iter);
    }
}

impl ClassArea for WorkingClassArea {
    fn search(&self, class: &str) -> Option<Arc<Class>> {
        for possible_class in &*self.classes {
            if &*possible_class.this == class {
                return Some(possible_class.clone());
            }
        }
        None
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

impl Default for MethodArea {
    fn default() -> Self {
        Self::new()
    }
}

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
