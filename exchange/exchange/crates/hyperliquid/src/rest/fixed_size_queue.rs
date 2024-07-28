use std::collections::VecDeque;
pub struct FixedSizeDeque<T> {
    elements: VecDeque<T>,
    max_size: usize,
}

impl<T> FixedSizeDeque<T> {
    // Create a new fixed-size deque with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        FixedSizeDeque {
            elements: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    // front
    pub fn front(&self) -> Option<&T> {
        self.elements.front()
    }

    // back
    pub fn back(&self) -> Option<&T> {
        self.elements.back()
    }

    // Add an element to the back of the deque
    pub fn push_back(&mut self, element: T) {
        if self.elements.len() == self.max_size {
            self.elements.pop_front(); // Remove from the front to maintain the fixed size
        }
        self.elements.push_back(element);
    }

    // Add an element to the front of the deque
    pub fn push_front(&mut self, element: T) {
        if self.elements.len() == self.max_size {
            self.elements.pop_back(); // Remove from the back to maintain the fixed size
        }
        self.elements.push_front(element);
    }

    // Remove an element from the front of the deque
    pub fn pop_front(&mut self) -> Option<T> {
        self.elements.pop_front()
    }

    // Remove an element from the back of the deque
    pub fn pop_back(&mut self) -> Option<T> {
        self.elements.pop_back()
    }

    // Check if the deque is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    // Get the number of elements in the deque
    pub fn len(&self) -> usize {
        self.elements.len()
    }
}
