use serde::{
    de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Serialize,
};
use std::{fmt, marker::PhantomData};

mod bitstream;
mod bitvec;
mod linked_list;
mod lru_cache;
mod nibble_list;
mod queue;
mod stack;

pub use bitstream::*;
pub use bitvec::*;
pub use linked_list::*;
pub use lru_cache::*;
pub use nibble_list::*;
pub use queue::*;
pub use stack::*;

type InternalPtr = u32;

#[derive(Copy, Clone, PartialEq, Serialize, serde::Deserialize, Debug)]
/// #Description
/// a pointer type for pointer-based data-structures
pub struct Ptr(InternalPtr);

impl Ptr {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for Ptr {
    fn from(idx: u32) -> Self {
        Self(idx as InternalPtr)
    }
}
impl From<u64> for Ptr {
    fn from(idx: u64) -> Self {
        Self(idx as InternalPtr)
    }
}
impl From<usize> for Ptr {
    fn from(idx: usize) -> Self {
        Self(idx as InternalPtr)
    }
}

pub const NULL: Ptr = Ptr(!0);

#[derive(Debug)]
pub struct Node<T, const N: usize> {
    data: Option<T>,
    children: [Ptr; N],
}

#[allow(dead_code)]
impl<T, const N: usize> Node<T, N> {
    pub fn new() -> Self {
        Self {
            data: None,
            children: [NULL; N],
        }
    }
    pub fn nullify(&mut self) {
        self.children = [NULL; N];
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn data_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    pub fn set_data(&mut self, item: T) {
        self.data = Some(item);
    }
}

impl<T, const N: usize> From<T> for Node<T, N> {
    fn from(item: T) -> Self {
        Self {
            data: Some(item),
            children: [NULL; N],
        }
    }
}

impl<T, const N: usize> Serialize for Node<T, N>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Node", 2)?;
        state.serialize_field("data", &self.data)?;
        state.serialize_field("children", &self.children.iter().collect::<Vec<_>>())?;
        state.end()
    }
}

impl<'de, T, const N: usize> Deserialize<'de> for Node<T, N>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &["data", "children"];

        enum Field {
            Data,
            Children,
        }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`data` or `children`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "data" => Ok(Field::Data),
                            "children" => Ok(Field::Children),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct NodeVisitor<T, const N: usize>(PhantomData<(T, [(); N])>);

        impl<'de, T, const N: usize> Visitor<'de> for NodeVisitor<T, N>
        where
            T: Deserialize<'de>,
        {
            type Value = Node<T, N>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Node<T,N>")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Node<T, N>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let data = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let heap_children: Vec<Ptr> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                let mut children = [NULL; N];
                heap_children
                    .iter()
                    .zip(children.iter_mut())
                    .for_each(|(&heap, stack)| {
                        *stack = heap;
                    });

                Ok(Node { data, children })
            }

            fn visit_map<V>(self, mut map: V) -> Result<Node<T, N>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut data = None;
                let mut children: Option<Vec<Ptr>> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Data => {
                            if data.is_some() {
                                return Err(de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                        Field::Children => {
                            if children.is_some() {
                                return Err(de::Error::duplicate_field("children"));
                            }
                            children = Some(map.next_value()?);
                        }
                    }
                }
                let data = data.ok_or_else(|| de::Error::missing_field("data"))?;
                let heap_children = children.ok_or_else(|| de::Error::missing_field("children"))?;
                let mut children = [NULL; N];
                heap_children
                    .iter()
                    .zip(children.iter_mut())
                    .for_each(|(&heap, stack)| {
                        *stack = heap;
                    });

                Ok(Node { data, children })
            }
        }

        deserializer.deserialize_struct("Node", FIELDS, NodeVisitor::<T, N>(PhantomData::default()))
    }
}
