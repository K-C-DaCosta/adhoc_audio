use serde::Deserialize;

use super::*;
use std::ops::{Index, IndexMut};

#[derive(Serialize, Deserialize)]
pub struct FixedRingBuffer<BACKING> {
    backing: BACKING,
    front: u32,
    rear: u32,
    len: u32,
    capacity: u32,
}

impl<BACKING> FixedRingBuffer<BACKING> {
    pub fn new(backing: BACKING, capacity: u32) -> Self {
        if capacity.count_ones() != 1 {
            panic!("capacity must be a power of 2")
        }

        Self {
            backing,
            front: 0,
            rear: 0,
            len: 0,
            capacity,
        }
    }
}

impl<T, BACKING> FixedRingBuffer<BACKING>
where
    BACKING: Index<usize, Output = T> + IndexMut<usize>,
{
    pub fn len(&self) -> usize {
        self.len as usize
    }
    pub fn is_full(&self) -> bool {
        self.len >= self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push_front(&mut self, item: T) {
        if self.is_full() == false {
            self.front = (self.front + self.capacity - 1) & (self.capacity - 1);
            let front = self.front;
            self[front] = item;
            self.len += 1;
        }
    }

    pub fn pop_front(&mut self) -> Option<&T> {
        if self.is_empty() == false {
            let old_front = self.front;
            self.front = (self.front + 1) & (self.capacity - 1);
            self.len -= 1;
            let item_ref = &self[old_front];
            Some(item_ref)
        } else {
            None
        }
    }

    pub fn push_rear(&mut self, item: T) {
        if self.is_full() == false {
            let rear = self.rear;
            self[rear] = item;
            self.rear = (self.rear + 1) & (self.capacity - 1);
            self.len += 1;
        }
    }

    pub fn pop_rear(&mut self) -> Option<&T> {
        if self.is_empty() == false {
            let new_rear = (self.rear + self.capacity - 1) & (self.capacity - 1);
            self.rear = new_rear;
            self.len -= 1;
            let item_ref = &self[new_rear];
            Some(item_ref)
        } else {
            None
        }
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> + '_
    where
        T: 'a,
    {
        let len = self.len;
        let front = self.front;
        let capacity = self.capacity;
        (0..len).map(move |offset| &self[(front + offset) % capacity])
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.front = 0;
        self.rear = 0;
    }
}

impl<T, BACKING> Index<u32> for FixedRingBuffer<BACKING>
where
    BACKING: Index<usize, Output = T> + IndexMut<usize>,
{
    type Output = T;
    fn index(&self, index: u32) -> &Self::Output {
        &self.backing[index as usize]
    }
}

impl<T, BACKING> IndexMut<u32> for FixedRingBuffer<BACKING>
where
    BACKING: Index<usize, Output = T> + IndexMut<usize>,
{
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.backing[index as usize]
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::FixedRingBuffer;

    #[test]
    fn sanity() {
        let mut queue = FixedRingBuffer::new([0; 32], 32);

        queue.push_rear(0);
        queue.push_rear(1);
        queue.push_rear(2);
        queue.push_rear(3);

        assert_eq!(
            vec![0, 1, 2, 3],
            queue.iter().map(|&a| a).collect::<Vec<_>>()
        );
        assert_eq!(4, queue.len());

        let item = queue.pop_front().map(|&a| a);
        assert_eq!(Some(0), item);
        assert_eq!(vec![1, 2, 3], queue.iter().map(|&a| a).collect::<Vec<_>>());

        let item = queue.pop_front().map(|&a| a);
        assert_eq!(Some(1), item);
        assert_eq!(vec![2, 3], queue.iter().map(|&a| a).collect::<Vec<_>>());

        let item = queue.pop_front().map(|&a| a);
        assert_eq!(Some(2), item);
        assert_eq!(vec![3], queue.iter().map(|&a| a).collect::<Vec<_>>());
        assert_eq!(1, queue.len());

        let item = queue.pop_rear().map(|&a| a);
        assert_eq!(Some(3), item);
        assert_eq!(
            Vec::<i32>::new(),
            queue.iter().map(|&a| a).collect::<Vec<_>>()
        );
        assert_eq!(0, queue.len());

        let item = queue.pop_rear().map(|&a| a);
        assert_eq!(None, item);
        assert_eq!(
            Vec::<i32>::new(),
            queue.iter().map(|&a| a).collect::<Vec<_>>()
        );
        assert_eq!(0, queue.len());
    }
}
