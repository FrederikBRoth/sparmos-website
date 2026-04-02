use std::fmt::Display;
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

    pub fn to_string(&self) -> String {
        self.storage
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("")
    }
    pub fn clear(&mut self) {
        self.storage.clear();
    }
}
