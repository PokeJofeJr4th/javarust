use std::{
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
    classes: Arc<[Arc<Class>]>,
}

impl ClassArea for SharedClassArea {
    fn search(&self, class: &str) -> Option<Arc<Class>> {
        for possible_class in &*self.classes {
            if &*possible_class.this == class {
                return Some(possible_class.clone());
            }
        }
        None
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
            classes: Arc::from(self.classes),
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

pub struct SharedMethodArea {
    methods: Arc<[(Arc<Class>, Arc<Method>)]>,
}

impl SharedMethodArea {
    #[must_use]
    pub fn search(
        &self,
        class: &str,
        method: &str,
        method_type: &MethodDescriptor,
    ) -> Option<(Arc<Class>, Arc<Method>)> {
        for (possible_class, possible_method) in &*self.methods {
            if &*possible_class.this == class
                && &*possible_method.name == method
                && &possible_method.descriptor == method_type
            {
                return Some((possible_class.clone(), possible_method.clone()));
            }
        }
        None
    }
}

pub struct WorkingMethodArea {
    methods: Vec<(Arc<Class>, Arc<Method>)>,
}

impl WorkingMethodArea {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            methods: Vec::new(),
        }
    }

    pub fn push(&mut self, class: Arc<Class>, method: Arc<Method>) {
        self.methods.push((class, method));
    }

    #[must_use]
    pub fn to_shared(self) -> SharedMethodArea {
        SharedMethodArea {
            methods: Arc::from(self.methods),
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = (Arc<Class>, Arc<Method>)>) {
        self.methods.extend(iter);
    }
}
