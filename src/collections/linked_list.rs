use super::*;
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

#[derive(Copy, Clone, PartialEq)]
pub enum InsertDirection {
    Left = 0,
    Right = 1,
}

impl InsertDirection {
    pub fn as_usize(self) -> usize {
        self as usize
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinkedList<T> {
    memory: Vec<Node<T, 2>>,
    front: Ptr,
    rear: Ptr,
    pool: Ptr,
    len: u32,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            memory: Vec::new(),
            front: NULL,
            rear: NULL,
            pool: NULL,
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn front(&self) -> Ptr {
        self.front
    }

    pub fn rear(&self) -> Ptr {
        self.rear
    }

    pub fn push_front(&mut self, data: T) {
        self.insert_at(self.front, InsertDirection::Left, data)
    }
    pub fn push_rear(&mut self, data: T) {
        self.insert_at(self.rear, InsertDirection::Right, data)
    }

    pub fn pop_rear(&mut self) -> Option<T> {
        self.remove_at(self.rear)
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.remove_at(self.front)
    }

    pub fn remove_at(&mut self, node: Ptr) -> Option<T> {
        if self.len() == 0 {
            return None;
        } else if self.len() == 1 {
            let r = self.rear;
            let f = self.front;

            if f != r {
                panic!("this should NEVER be true if len ==1");
            }

            self.front = NULL;
            self.rear = NULL;
            self.len = 0;
            self.free(r)
        } else {
            let left_neighbour = self[node].children[0];
            let right_neighbour = self[node].children[1];
            self[left_neighbour].children[1] = right_neighbour;
            self[right_neighbour].children[0] = left_neighbour;

            if node == self.front {
                self.front = self[self.front].children[1];
            }

            if node == self.rear {
                self.rear = self[self.rear].children[0];
            }

            self.len -= 1;
            self.free(node)
        }
    }

    pub fn insert_at(&mut self, node: Ptr, direction: InsertDirection, data: T) {
        if self.len() == 0 {
            let new_node = self.allocate(data);
            self[new_node].children = [new_node; 2];
            self.front = new_node;
            self.rear = new_node;
            self.len += 1;
            return;
        }

        if node == NULL {
            return;
        }

        let raw_direction = direction.as_usize();
        let new_node = self.allocate(data);
        let adj_node = self[node].children[raw_direction];
        self[node].children[raw_direction] = new_node;
        self[adj_node].children[1 - raw_direction] = new_node;
        self[new_node].children[1 - raw_direction] = node;
        self[new_node].children[raw_direction] = adj_node;

        if node == self.front && direction == InsertDirection::Left {
            self.front = new_node;
        }

        if node == self.rear && direction == InsertDirection::Right {
            self.rear = new_node;
        }

        self.len += 1;
    }

    pub fn allocate(&mut self, item: T) -> Ptr {
        if self.pool == NULL {
            self.memory.push(Node::from(item));
            Ptr::from(self.memory.len() - 1)
        } else {
            let reused_node = self.pool;
            self.pool = self[reused_node].children[1];
            self[reused_node].set_data(item);
            self[reused_node].nullify();

            reused_node
        }
    }

    pub fn free(&mut self, node: Ptr) -> Option<T> {
        self[node].children = [NULL, self.pool];
        self.pool = node;
        self[node].data.take()
    }

    pub fn get(&self, node: Ptr) -> Option<&Node<T, 2>> {
        self.memory.get(node.as_usize())
    }

    pub fn get_mut(&mut self, node: Ptr) -> Option<&mut Node<T, 2>> {
        self.memory.get_mut(node.as_usize())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> {
        LinkedListIterator::new(self).map(move |node| self[node].data.as_ref().unwrap())
    }
    
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let self_ref: &mut Self = unsafe { std::mem::transmute(self as *mut Self) };
        LinkedListIterator::new(self).map(move |node| {
            let node = unsafe {
                &mut *self_ref
                    .memory
                    .as_mut_ptr()
                    .offset(node.as_usize() as isize)
            };
            node.data.as_mut().unwrap()
        })
    }
}

impl<T> Index<Ptr> for LinkedList<T> {
    type Output = Node<T, 2>;
    fn index(&self, index: Ptr) -> &Self::Output {
        &self.memory[index.as_usize()]
    }
}
impl<T> IndexMut<Ptr> for LinkedList<T> {
    fn index_mut(&mut self, index: Ptr) -> &mut Self::Output {
        &mut self.memory[index.as_usize()]
    }
}

pub struct LinkedListIterator<'a, T> {
    ll: &'a LinkedList<T>,
    items_remianing: usize,
    ptr: Ptr,
}
impl<'a, T> LinkedListIterator<'a, T> {
    pub fn new(ll: &'a LinkedList<T>) -> Self {
        Self {
            ll,
            items_remianing: ll.len(),
            ptr: ll.front,
        }
    }
}
impl<'a, T> std::iter::Iterator for LinkedListIterator<'a, T> {
    type Item = Ptr;
    fn next(&mut self) -> Option<Self::Item> {
        if self.items_remianing > 0 {
            let cur_node = self.ptr;
            self.items_remianing -= 1;
            self.ptr = self.ll[cur_node].children[1];
            Some(cur_node)
        } else {
            None
        }
    }
}
mod tests {

    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn sanity_test() {
        let mut ll = LinkedList::<i32>::new();
        ll.push_front(1);
        ll.push_front(2);
        ll.push_front(3);
        ll.push_rear(99);
        ll.pop_front();
        ll.pop_front();
        ll.pop_front();
        ll.pop_rear();
        ll.push_rear(1);
        ll.push_front(-99);
        ll.push_rear(3);

        let list = ll.iter().collect::<Vec<_>>();
        println!("{:?}", list);

        println!("front:{:?}", ll.get(ll.front()).and_then(|n| n.data()));
        println!("rear:{:?}", ll.get(ll.rear()).and_then(|n| n.data()));

        let serialized = serde_json::to_string(&ll).unwrap();
        println!("json:{}", serialized);

        let json_ll: LinkedList<i32> = serde_json::from_str(&serialized).unwrap();
        println!("\n\njson LL:\n\n{:?}", json_ll);

        let binary_ll = bincode::serialize(&ll).unwrap();
        let bincode_ll: LinkedList<i32> = bincode::deserialize(binary_ll.as_slice()).unwrap();
        println!("\n\nbincode ll:\n\n{:?}", bincode_ll);
    }
}
