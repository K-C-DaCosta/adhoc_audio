use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NibbleList {
    data: Vec<u8>,
    nibble_count: u32,
}
impl NibbleList {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            nibble_count: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.nibble_count as usize
    }

    pub fn push(&mut self, val: u8) {
        let block_idx = self.nibble_count as usize / 2;
        let nibble_idx = self.nibble_count % 2;
        if block_idx >= self.data.len() {
            self.data.push(0);
        }
        self.data[block_idx] |= val << 4 * (nibble_idx);
        self.nibble_count += 1;
    }

    pub fn get(&self, idx: usize) -> u8 {
        let block_idx = idx / 2;
        let nibble_idx = idx % 2;
        if block_idx >= self.data.len() {
            return 0;
        }
        (self.data[block_idx] >> 4 * (nibble_idx)) & 0x0f
    }

    pub fn set(&mut self, idx: usize, val: u8) {
        let block_idx = idx / 2;
        let nibble_idx = (idx % 2) as u8;
        if block_idx >= self.data.len() {
            return;
        }
        self.data[block_idx] &= (0x0f * nibble_idx) | (0xf0 * (1 - nibble_idx));
        self.data[block_idx] |= val << 4 * (nibble_idx);
    }

    pub fn clear(&mut self) {
        self.nibble_count = 0;
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = u8> + 'a {
        (0..self.len()).map(move |idx| self.get(idx))
    }
}

mod tests {

    #[allow(unused_imports)]
    use super::NibbleList;

    #[test]
    fn sanity() {
        let mut list = NibbleList::new();
        let input = [1u8, 2, 3, 4, 5, 6, 7];
        input.iter().for_each(|&b| list.push(b as u8));
        assert_eq!(&input, list.iter().collect::<Vec<_>>().as_slice());
        assert_eq!(input.len(), list.len());

        list.set(6, 10);
        assert_eq!(
            vec![1, 2, 3, 4, 5, 6, 10],
            list.iter().collect::<Vec<_>>().as_slice()
        );

        list.set(5, 1);
        assert_eq!(
            vec![1, 2, 3, 4, 5, 1, 10],
            list.iter().collect::<Vec<_>>().as_slice()
        );

        list.set(0, 10);
        assert_eq!(
            vec![10, 2, 3, 4, 5, 1, 10],
            list.iter().collect::<Vec<_>>().as_slice()
        );
    }
}
