use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use crate::{
    class::{Class, Method, MethodDescriptor},
    virtual_machine::object::Object,
};

pub type SharedHeap = Arc<Mutex<Heap>>;

pub struct Heap {
    contents: Vec<Arc<Mutex<Object>>>,
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
    pub const fn new() -> Self {
        Self {
            contents: Vec::new(),
        }
    }

    #[must_use]
    pub fn make_shared(self) -> SharedHeap {
        Arc::new(Mutex::new(self))
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
    methods: HashMap<Arc<str>, (Arc<Class>, ClassTable)>,
}

type ClassTable = HashMap<Arc<str>, HashMap<MethodDescriptor, Arc<Method>>>;

impl MethodArea {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    pub fn push(&mut self, class: Arc<Class>, method: Arc<Method>) {
        let signature = method.descriptor.clone();
        self.methods
            .entry(class.this.clone())
            .or_insert_with(|| (class, HashMap::new()))
            .1
            .entry(method.name.clone())
            .or_default()
            .insert(signature, method);
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
        // for (possible_class, possible_method) in &*self.methods {
        //     if &*possible_class.this == class
        //         && &*possible_method.name == method
        //         && &possible_method.descriptor == method_type
        //     {
        //         return Some((possible_class.clone(), possible_method.clone()));
        //     }
        // }
        // None
        let (class, class_table) = self.methods.get(class)?;
        let method_table = class_table.get(method)?;
        let method = method_table.get(method_type)?;
        Some((class.clone(), method.clone()))
    }
}

impl Default for MethodArea {
    fn default() -> Self {
        Self::new()
    }
}
