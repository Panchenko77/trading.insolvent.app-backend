use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;

type RegistryGetter = Box<dyn FnMut() -> Box<dyn Any>>;

/// Registry for task service
#[derive(Default)]
pub struct Registry {
    getters: HashMap<TypeId, RegistryGetter>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            getters: Default::default(),
        }
    }
    pub fn add_cloned<T: Any + Clone + 'static>(&mut self, item: T) {
        self.add_fn(move || item.clone());
    }
    pub fn add_taken<T: Any + 'static>(&mut self, item: T) {
        let mut item = Some(item);
        let type_id = type_name::<T>();
        self.add_fn(move || item.take().expect(type_id));
    }
    pub fn add_fn<T, F>(&mut self, mut getter: F)
    where
        T: Any + 'static,
        F: FnMut() -> T + 'static,
    {
        self.getters.insert(
            TypeId::of::<T>(),
            Box::new(move || {
                let item = getter();
                Box::new(item)
            }),
        );
    }

    pub fn get<T: Any + 'static>(&mut self) -> Option<T> {
        let getter = self.getters.get_mut(&TypeId::of::<T>())?;

        let item = getter();
        let item = *item.downcast::<T>().unwrap();
        Some(item)
    }
    pub fn get_unwrap<T: Any + 'static>(&mut self) -> T {
        self.get::<T>().unwrap_or_else(|| panic!("{}", type_name::<T>()))
    }
}
