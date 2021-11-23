use super::{linked_list::LinkedList, *};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};

#[derive(Serialize, Deserialize, Debug)]
pub struct LruCache<K, V>
where
    K: Eq + Serialize + Hash,
{
    table: HashMap<K, Ptr>,
    list: LinkedList<(K, V)>,
    capacity: u32,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone + Serialize,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            table: HashMap::new(),
            list: LinkedList::new(),
            capacity: capacity as u32,
        }
    }
    pub fn len(&self) -> usize {
        self.list.len()
    }


    /// fetches key if it exists, **DOESN't** raise key in cache
    pub fn lookup(&self,key:&K)->Option<&V>{
        let table = & self.table;
        let list = & self.list;
        table.get(&key).and_then(move |&node| {
            list[node].data.as_ref().map(|(_, v)| v)
        })
    }

    /// fetches key if it exists, raises priority
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let table = &mut self.table;
        let list = &mut self.list;
        table.get_mut(&key).and_then(move |node_ref| {
            Self::raise_key(node_ref, list, None);
            list[list.front()].data.as_ref().map(|(_, v)| v)
        })
    }
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let table = &mut self.table;
        let list = &mut self.list;
        table
            .get_mut(&key)
            .and_then(move |&mut node| list.remove_at(node))
            .map(|(k, v)| {
                table.remove(&k);
                v
            })
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let table = &mut self.table;
        let list = &mut self.list;
        table.get_mut(&key).and_then(move |node_ref| {
            Self::raise_key(node_ref, list, None);
            let new_front = list.front();
            list[new_front].data.as_mut().map(|(_, v)| v)
        })
    }

    pub fn put(&mut self, key: K, val: V) {
        let capacity = self.capacity;
        let len = self.len();
        let table = &mut self.table;
        let list = &mut self.list;

        match table.get_mut(&key) {
            Some(node_ref) => {
                //raise key
                Self::raise_key(node_ref, list, Some(val));
            }
            None => {
                if len >= capacity as usize {
                    let rear_node = list.rear();
                    let (key, _) = list[rear_node].data.as_ref().unwrap();
                    table.remove(key);
                    list.pop_rear();
                }
                list.push_front((key.clone(), val));
                table.insert(key, list.front());
            }
        }
    }

    fn raise_key(table_node_ref: &mut Ptr, list: &mut LinkedList<(K, V)>, val: Option<V>) {
        let (key, old_value) = list.remove_at(*table_node_ref).unwrap();

        if let Some(val) = val {
            list.push_front((key, val));
        } else {
            list.push_front((key, old_value));
        }

        *table_node_ref = list.front();
    }
}
