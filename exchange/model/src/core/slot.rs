use std::fmt::{Debug, Display};

#[derive(Clone, Debug, Default)]
pub enum Slot<T> {
    #[default]
    None,
    Unique(T),
    Multiple(Vec<T>),
}

impl<T: Debug> Slot<T> {
    pub fn new() -> Self {
        Self::None
    }
    pub fn is_none(&self) -> bool {
        matches!(self, Slot::None)
    }
    pub fn push(&mut self, value: T) {
        match std::mem::take(self) {
            Slot::None => *self = Slot::Unique(value),
            Slot::Unique(v) => {
                *self = Slot::Multiple(vec![v, value]);
                // warn!("Slot::Unique is converted to Slot::Multiple: {:?}", self);
            }
            Slot::Multiple(mut v) => {
                v.push(value);
                *self = Slot::Multiple(v);
                // warn!("Slot::Multiple is pushed: {:?}", self);
            }
        }
    }
    pub fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        for value in values {
            self.push(value);
        }
    }
    pub fn get_first(&self) -> Option<&T> {
        match self {
            Slot::None => None,
            Slot::Unique(v) => Some(v),
            Slot::Multiple(v) => {
                // tracing::warn!("Slot::Multiple is converted to Slot::Unique: {:?}", self);
                v.first()
            }
        }
    }
    pub fn get_first_by(&self, _key: impl Display) -> Option<&T> {
        match self {
            Slot::None => None,
            Slot::Unique(v) => Some(v),
            Slot::Multiple(v) => {
                // tracing::warn!(
                //     "Slot::Multiple is converted to Slot::Unique by {}: {:?}",
                //     _key, self
                // );
                v.first()
            }
        }
    }
    pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
        match self {
            Slot::None => {}
            Slot::Unique(v) => {
                if !f(v) {
                    *self = Slot::None;
                }
            }
            Slot::Multiple(v) => {
                v.retain(f);
                if v.len() == 1 {
                    *self = Slot::Unique(v.pop().unwrap());
                }
            }
        }
    }
    pub fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self {
            Slot::None => Box::new([].iter()),
            Slot::Unique(v) => Box::new([v].into_iter()),
            Slot::Multiple(v) => Box::new(v.iter()),
        }
    }
}
