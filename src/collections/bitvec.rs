/// # Description
/// a vector of bits
pub struct BitVec {
    binary: Vec<u64>,
    bit_cursor: u128,
}

impl BitVec {
    pub fn new() -> Self {
        Self {
            binary: Vec::new(),
            bit_cursor: 0,
        }
    }

    pub fn clear(&mut self) {
        self.bit_cursor = 0;
    }

    /// # Description
    /// return number of bits written to vector
    pub fn len(&self) -> usize {
        self.bit_cursor as usize
    }

    /// # Description
    /// returns number of bits allocated in memory
    pub fn capacity(&self) -> usize {
        self.binary.len() * 64
    }

    ///  # Description
    ///  Pushes a `bit` into the bitvec
    ///  ## Parameters
    ///  - `bit` is expected to be '0' or '1' , and is forced into that range if `bit` isnt 0 or 1
    pub fn push(&mut self, bit: u64) {
        let bit = bit & 1;
        let (chunk_idx, chunk_bit_idx) = self.cursor_index_pair();

        //allocate if needed
        if chunk_idx >= self.binary.len() {
            self.binary.push(0);
        }

        self.binary[chunk_idx] |= bit << chunk_bit_idx;
        self.offset_cursor(1);
    }

    /// # Description
    /// gets a bit at `idx`
    /// ## Returns
    /// bit value at `idx` and if  `idx` invalid then it returns 0
    pub fn get(&self, idx: usize) -> u64 {
        let idx = idx as u128;
        let chunk_idx = Self::chunk_index(idx);
        let chunk_bit_idx = Self::chunk_bit_index(idx);

        if chunk_idx >= self.binary.len() {
            return 0;
        }

        (self.binary[chunk_idx] >> chunk_bit_idx) & 1
    }

    /// #Description
    /// sets a bit at `idx`, does nothing when `idx` is invalid
    pub fn set(&mut self, idx: usize, bit: u64) {
        let idx = idx as u128;
        let chunk_idx = Self::chunk_index(idx);
        let chunk_bit_idx = Self::chunk_bit_index(idx);

        if chunk_idx >= self.binary.len() {
            return;
        }

        //read chunk into register
        let mut chunk = self.binary[chunk_idx];

        //clear destination bit
        chunk &= !(1 << chunk_bit_idx);

        //write bit to destination
        chunk |= (bit & 1) << chunk_bit_idx;

        //write chunk to list
        self.binary[chunk_idx] = chunk;
    }

    fn cursor_index_pair(&self) -> (usize, u64) {
        let idx = self.bit_cursor;
        (Self::chunk_index(idx), Self::chunk_bit_index(idx))
    }

    fn chunk_index(idx: u128) -> usize {
        (idx / 64) as usize
    }

    fn chunk_bit_index(idx: u128) -> u64 {
        (idx % 64) as u64
    }

    fn offset_cursor(&mut self, offset: i64) {
        self.bit_cursor = (self.bit_cursor as i128 + offset as i128) as u128;
    }

    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        (0..self.len()).map(|idx| self.get(idx))
    }

    ///computes the number of bits required to be allocated to keep `num_bits` worth of information
    pub fn compute_bits_required(num_bits: u64) -> usize {
        (((num_bits / 64) + (num_bits % 64).clamp(0, 1)) * 64) as usize
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn sanity() {
        let mut bit_list = BitVec::new();

        //push tests
        bit_list.push(0);
        bit_list.push(1);
        bit_list.push(1);
        bit_list.push(0);
        assert_eq!(vec![0, 1, 1, 0], bit_list.iter().collect::<Vec<_>>());

        bit_list.push(1);
        assert_eq!(vec![0, 1, 1, 0, 1], bit_list.iter().collect::<Vec<_>>());
        assert_eq!(5, bit_list.len());
        assert_eq!(64, bit_list.capacity());

        //check if Self::set works
        bit_list.set(1, 0);
        bit_list.set(4, 0);
        assert_eq!(vec![0, 0, 1, 0, 0], bit_list.iter().collect::<Vec<_>>());

        //check if Self::clear works
        bit_list.clear();
        assert_eq!(Vec::<u64>::new(), bit_list.iter().collect::<Vec<u64>>());
        assert_eq!(64, bit_list.capacity());
    }

    #[test]
    /// generetate sequence of 0s and 1s, write it into bitvec, then read it back
    fn read_back_sequence() {
        let seq_len = 100_000;
        let bit_sequence = (0u64..seq_len).map(|i| i % 2).collect::<Vec<_>>();

        let mut bit_list = BitVec::new();
        bit_sequence.iter().for_each(|&b| {
            bit_list.push(b);
        });
        assert_eq!(bit_sequence, bit_list.iter().collect::<Vec<_>>());
        assert_eq!(seq_len as usize, bit_list.len());
        assert_eq!(BitVec::compute_bits_required(seq_len), bit_list.capacity());
    }
}
