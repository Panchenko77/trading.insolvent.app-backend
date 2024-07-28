use hashbrown::Equivalent;
use std::mem::transmute;

pub trait VecExt {
    type Item;
    fn get_eq<Q: Equivalent<Self::Item>>(&self, key: &Q) -> Option<&Self::Item>;
    fn get_eq_mut<Q: Equivalent<Self::Item>>(&mut self, key: &Q) -> Option<&mut Self::Item>;
    fn entry<Q: Equivalent<Self::Item>>(&mut self, key: &Q) -> VecEntry<Self::Item>;
}

impl<T> VecExt for Vec<T> {
    type Item = T;

    fn get_eq<Q: Equivalent<Self::Item>>(&self, key: &Q) -> Option<&Self::Item> {
        self.iter().find(|x| key.equivalent(x))
    }
    fn get_eq_mut<Q: Equivalent<Self::Item>>(&mut self, key: &Q) -> Option<&mut Self::Item> {
        self.iter_mut().find(|x| key.equivalent(x))
    }
    fn entry<Q: Equivalent<Self::Item>>(&mut self, key: &Q) -> VecEntry<Self::Item> {
        let v = match self.iter_mut().find(|x| key.equivalent(x)) {
            Some(v) => VecEntry::Occupied(OccupiedEntry::new(v)),
            None => VecEntry::Vacant(VacantEntry::new(self)),
        };
        // safety: we are returning a reference to the element in the vector, or a reference to the vector itself
        // this will be accepted by rustc in the future
        unsafe { transmute(v) }
    }
}

pub enum VecEntry<'a, T> {
    Occupied(OccupiedEntry<'a, T>),
    Vacant(VacantEntry<'a, T>),
}

pub struct OccupiedEntry<'a, T> {
    value: &'a mut T,
}

impl<'a, T> OccupiedEntry<'a, T> {
    pub fn new(value: &'a mut T) -> Self {
        Self { value }
    }
    pub fn get(&self) -> &T {
        self.value
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.value
    }
    pub fn into_mut(self) -> &'a mut T {
        self.value
    }
    pub fn insert(&mut self, value: T) -> T {
        std::mem::replace(self.value, value)
    }
}

pub struct VacantEntry<'a, T> {
    value: &'a mut Vec<T>,
}

impl<'a, T> VacantEntry<'a, T> {
    pub fn new(value: &'a mut Vec<T>) -> Self {
        Self { value }
    }
    pub fn push(self, value: T) -> &'a mut T {
        self.value.push(value);
        self.value.last_mut().unwrap()
    }
}
