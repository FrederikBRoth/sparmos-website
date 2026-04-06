use core::fmt;
use std::fmt::{Display, Formatter};
pub struct CircularBuffer<T> {
    storage: Vec<T>,
    size: usize,
}

impl<T: Display> CircularBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            storage: Vec::with_capacity(size),
            size,
        }
    }

    pub fn insert(&mut self, item: T) {
        self.storage.push(item);
        if self.storage.len() > self.size {
            self.storage.remove(0);
        }
    }

    pub fn clear(&mut self) {
        self.storage.clear();
    }
}

impl<T: Display> Display for CircularBuffer<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for item in &self.storage {
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}
