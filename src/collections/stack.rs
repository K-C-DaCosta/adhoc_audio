use serde::Deserialize;

use super::*;

use std::ops::{Index, IndexMut};

#[derive(Serialize, Deserialize,Clone,Copy)]
pub struct CircularStack<T> {
    data: [T; 16],
    cursor: u32,
    len: u32,
}

impl<T> CircularStack<T>
where
    T: Default + Copy,
{
    pub fn new() -> Self {
        Self {
            data: [T::default(); 16],
            cursor: 0,
            len: 0,
        }
    }
    
    pub fn len(&self)->usize{
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len >= self.data.len() as u32
    }

    pub fn push(&mut self, item: T) {
        let len = self.data.len() as u32;
        let idx = self.cursor % len;
        self[idx] = item;
        self.cursor = (self.cursor + 1) % len;
        self.len = (len + 1).min(len);
    }
    
    pub fn pop(&mut self) -> T {
        let len = self.data.len() as u32;
        self.cursor = ((self.cursor + len) - 1) % len;
        self.len = (len - 1).min(0);
        let item = self[self.cursor];
        item
    }

    /// fetches previously written element
    pub fn prev(&self, mut offset: u32) -> T {
        let len = self.data.len() as u32;
        offset = offset.clamp(1, len-1);
        let idx = ((self.cursor + len) - offset) % len;
        self[idx]
    }
}

impl<T> Index<u32> for CircularStack<T> {
    type Output = T;
    fn index(&self, index: u32) -> &Self::Output {
        &self.data[index as usize]
    }
}

impl<T> IndexMut<u32> for CircularStack<T> {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.data[index as usize]
    }
}

mod test {
    #[allow(unused_imports)]
    use super::CircularStack;

    #[test]
    fn sanity_test() {
        let mut queue: CircularStack<u32> = CircularStack::new();

        queue.push(1);
        queue.push(2);
        assert_eq!(2, queue.prev(1));
        assert_eq!(1, queue.prev(2));
    }

    #[test]
    fn sanity_test_2() {
        let mut queue: CircularStack<u32> = CircularStack::new();
        queue.push(1);
        assert_eq!(1, queue.prev(1));
        queue.push(2);
        assert_eq!(2, queue.prev(1));
        assert_eq!(1, queue.prev(2));
    }

    #[test]
    fn sanity_test_3() {
        let mut queue: CircularStack<u32> = CircularStack::new();
        queue.push(1);
        assert_eq!(1, queue.prev(1));
        queue.push(2);
        assert_eq!(2, queue.prev(1));
        assert_eq!(1, queue.prev(2));

        assert_eq!(2,queue.pop());
        assert_eq!(1,queue.pop());
        assert_eq!(0,queue.len());

    }
}
