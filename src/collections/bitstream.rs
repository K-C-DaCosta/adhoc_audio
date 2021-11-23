use std::mem;

use serde::{Deserialize, Serialize};

/// when unary bits is too high I store integer in a `CAPPED_BITS` integer
const CAPPED_BITS: usize = 14;
const CAPPED_SHIFT_FACTOR: usize = 128 - CAPPED_BITS;

// ---------------NEVER CHANGE THESE ---------------
const CHUNK_SIZE_IN_BITS: usize = 128;
const NUM_OF_BITS_IN_BYTES: usize = 8;
//--------------------------------------------------

/*

MY THOUGHTS ON BITSTREAM IMPLEMENTATION



DIAGRAM OF BITSTREAM:
  chunks:         0                     1
  bits:       127 126... 3 2 1 0 | 255 ... 129 128 |
  bit_cursor:      ^


  remaining_bits = 128 -chunk_bit_idx;


   0 1 0 1
   3 2 1 0


CASE 1: number fits within block

    bits_to_be_written = num_8 as u128;
    block |= bits_to_be_written << bit_idx
    self.offset(8)

CASE 2: number doesn't fit into a block ( 128-bit_index < bits_to_be_written )

    bits_to_be_written = num_8 as u128;
    nearly_full_block |= bits_to_be_written << bit_idx
    self.offset(128-bit_idx);
    bits_to_be_written >>= 128-bit_idx;

*/

#[allow(dead_code)]
pub const CAPPED_MAX: i16 = (1 << (CAPPED_BITS - 1)) - 1;

#[allow(dead_code)]
pub const CAPPED_MIN: i16 = -(CAPPED_MAX + 1);

#[derive(Serialize, Deserialize)]
pub struct BitStream {
    pub binary: Vec<u128>,
    bit_cursor: u128,
    capacity: u128,
}
impl BitStream {
    pub fn new() -> Self {
        Self {
            binary: Vec::new(),
            bit_cursor: 0,
            capacity: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.bit_cursor as usize
    }
    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }
    pub fn write_bits<T>(&mut self, val: T, num_bits: usize)
    where
        T: Copy,
        u128: From<T>,
    {
        let bits = u128::from(val);
        let bits_to_be_written = num_bits;
        let cur_chunk_idx = self.chunk_index();
        let nxt_chunk_idx = cur_chunk_idx + 1;
        let chunk_bit_idx = self.chunk_bit_index();
        self.allocate_if_needed(cur_chunk_idx);
        self.allocate_if_needed(nxt_chunk_idx);

        //clear everything left of the bit_idx
        self.binary[cur_chunk_idx] &= (1 << chunk_bit_idx) - 1;
        //write bits here
        self.binary[cur_chunk_idx] |= bits << chunk_bit_idx;

        let remaining_bits = CHUNK_SIZE_IN_BITS - chunk_bit_idx;
        if remaining_bits < bits_to_be_written {
            //clear
            self.binary[nxt_chunk_idx] &= (1 << (bits_to_be_written - remaining_bits)) - 1;
            //write
            self.binary[nxt_chunk_idx] |= bits >> remaining_bits;
        }

        self.offset_bit_cursor(bits_to_be_written as i128);
    }

    pub fn peek_bits(&mut self, num_bits: usize) -> u128 {
        let mut bits = 0;
        let bits_to_be_read = num_bits;

        let mask = 1u128
            .checked_shl(bits_to_be_read as u32)
            .map(|result| result - 1)
            .unwrap_or(!0);

        let cur_chunk_idx = self.chunk_index();
        let nxt_chunk_idx = cur_chunk_idx + 1;
        let chunk_bit_idx = self.chunk_bit_index();

        self.allocate_if_needed(cur_chunk_idx);
        self.allocate_if_needed(nxt_chunk_idx);

        // return 0 if oob
        if cur_chunk_idx >= self.binary.len() {
            return 0;
        }

        bits |= self.binary[cur_chunk_idx] >> chunk_bit_idx;
        let remaining_bits = 128 - chunk_bit_idx;
        if remaining_bits < bits_to_be_read {
            bits |= self.binary[nxt_chunk_idx] << remaining_bits
        }

        bits & mask
    }

    /// # Description
    /// Writes a single bit into the stream
    /// # Parameters
    /// - `val` - should either be `0` or `1`  
    pub fn write_bit(&mut self, val: u8) {
        let bit = (val as u128) & 1;
        let chunk_idx = self.chunk_index();
        let chunk_bit_idx = self.chunk_bit_index();
        self.allocate_if_needed(chunk_idx);
        self.binary[chunk_idx] &= (1 << chunk_bit_idx) - 1;
        self.binary[chunk_idx] |= bit << chunk_bit_idx;
        self.offset_bit_cursor(1);
    }

    /// # Description
    /// Reads a single bit into the stream
    /// # Parameters
    /// - `val` - should either be `0` or `1`  
    pub fn read_bit(&mut self) -> u128 {
        let chunk_idx = self.chunk_index();
        let bit_idx = self.chunk_bit_index();
        if chunk_idx >= self.binary.len() {
            return 0;
        }
        self.offset_bit_cursor(1);
        let extracted_bit = self.binary[chunk_idx] >> bit_idx;
        extracted_bit & 1
    }

    pub fn read_bits(&mut self, bit_count: usize) -> u128 {
        let peeked_val = self.peek_bits(bit_count);
        self.offset_bit_cursor(bit_count as i128);
        peeked_val
    }

    pub fn peek<T>(&mut self) -> u128
    where
        T: Copy,
    {
        self.peek_bits(mem::size_of::<T>() * NUM_OF_BITS_IN_BYTES)
    }

    pub fn write<T>(&mut self, val: T)
    where
        T: Copy,
        u128: From<T>,
    {
        self.write_bits(val, mem::size_of::<T>() * 8)
    }

    //read by fixed amount
    pub fn read<T>(&mut self) -> u128
    where
        T: Copy,
        u128: From<T>,
    {
        let bits_to_be_read = mem::size_of::<T>() * NUM_OF_BITS_IN_BYTES;
        let peeked_val = self.peek::<T>();
        self.offset_bit_cursor(bits_to_be_read as i128);
        peeked_val
    }

    fn allocate_if_needed(&mut self, chunk_idx: usize) {
        if chunk_idx >= self.binary.len() {
            self.binary.push(0);
        }
    }

    pub fn zero(&mut self) {
        self.binary.iter_mut().for_each(|e| *e = 0);
    }

    pub fn seek_start(&mut self) {
        self.bit_cursor = 0;
    }

    /// # Description
    /// Writes number `value`, but if unary is too large it will write a fixed signed integer of size `CAPPED_BITS`
    /// to the stream instead.
    /// ## Comments
    /// before calling this function make sure that:\
    /// `CAPPED_MIN` <=  `value` <= `CAPPED_MAX`  
    pub fn write_compressed_capped<const DIVISOR: i16>(&mut self, value: i16) {
        let quotient = value.abs() / DIVISOR;
        let capping_not_needed = quotient < 16;
        let is_capped_bit_flag = 1 - (capping_not_needed) as u8;
        self.write_bit(is_capped_bit_flag);
        if capping_not_needed {
            self.write_compressed_divisor(DIVISOR, value);
        } else {
            self.write_bits(value.clamp(CAPPED_MIN, CAPPED_MAX) as u16, CAPPED_BITS);
        }
    }

    pub fn read_compressed_capped(&mut self, divisor: i16) -> i16 {
        let is_capped = self.read_bit() as u8 == 1;
        if is_capped {
            let bits_read = self.read_bits(CAPPED_BITS) as i128;
            ((bits_read << CAPPED_SHIFT_FACTOR) >> CAPPED_SHIFT_FACTOR) as i16
        } else {
            self.read_compressed_divisor(divisor)
        }
    }
    pub fn write_compressed_divisor(&mut self, divisor: i16, entropy: i16) {
        self.write_compressed((divisor - 1).count_ones() as i16, entropy)
    }
    pub fn read_compressed_divisor(&mut self, divisor: i16) -> i16 {
        self.read_compressed((divisor - 1).count_ones() as i16)
    }

    pub fn write_compressed(&mut self, exponent: i16, entropy: i16) {
        let sign_bit = (entropy >> 15) & 1;
        let mut quotient = entropy.abs() >> exponent;
        let remainder = entropy.abs() & ((1 << exponent) - 1);
        let remainder_size_in_bits = exponent as usize;

        //write sign bit
        self.write_bit(sign_bit as u8);
        //write unary quotient
        while quotient > 0 && quotient % 16 != 0 {
            self.write_bit(1);
            quotient -= 1;
        }
        while quotient > 0 && quotient % 16 == 0 {
            self.write::<u16>(!0);
            quotient -= 16;
        }
        //zero bit denotes end of unary value
        self.write_bit(0);
        //write remainder
        self.write_bits(remainder as u32, remainder_size_in_bits)
    }

    pub fn read_compressed(&mut self, exponent: i16) -> i16 {
        //write sign bit
        let sign_bit = self.read_bit() as i16;
        let remainder_size_in_bits = exponent as u32;
        let divisor = 1 << exponent;

        //read zero(expected)
        let mut quotient = 0;
        let mut _temp = 0;

        while {
            _temp = self.peek::<u128>();
            _temp.count_zeros() == 0
        } {
            self.read::<u128>();
            quotient += 128;
        }
        while {
            _temp = self.peek::<u64>();
            _temp.count_zeros() == 0
        } {
            self.read::<u64>();
            quotient += 64;
        }
        while {
            _temp = self.peek::<u32>();
            _temp.count_zeros() == 0
        } {
            self.read::<u32>();
            quotient += 32;
        }

        while {
            _temp = self.peek::<u16>();
            _temp.count_zeros() == 0
        } {
            self.read::<u16>();
            quotient += 16;
        }

        while {
            _temp = self.peek::<u8>();
            _temp.count_zeros() == 0
        } {
            self.read::<u8>();
            quotient += 8;
        }
        while self.read_bit() != 0 {
            quotient += 1;
        }

        //read
        let remainder = self.read_bits(remainder_size_in_bits as usize) as i16;
        let unsigned_val = divisor * quotient + remainder;
        unsigned_val * (-sign_bit) + unsigned_val * (1 - sign_bit)
    }

    fn chunk_index(&self) -> usize {
        (self.bit_cursor / 128) as usize
    }
    fn chunk_bit_index(&self) -> usize {
        (self.bit_cursor % 128) as usize
    }
    pub fn offset_bit_cursor(&mut self, offset: i128) {
        self.bit_cursor = (self.bit_cursor as i128 + offset) as u128;
        self.capacity = self.bit_cursor.max(self.capacity);
    }

    pub fn set_bit_cursor(&mut self, idx: u128) {
        self.bit_cursor = idx;
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::BitStream;
    #[allow(unused_imports)]
    use super::{CAPPED_MAX, CAPPED_MIN};

    #[test]
    fn compressed_capped() {
        let mut bit_stream = BitStream::new();
        for k in CAPPED_MIN + 1..CAPPED_MAX - 1 {
            bit_stream.write_compressed_capped::<2>(k as i16);
            bit_stream.seek_start();
            let result = bit_stream.read_compressed_capped(2);
            bit_stream.seek_start();
            assert_eq!(k, result, "{}!={} but should be equal", k, result);
        }
    }

    #[test]
    fn compressed_test_negative() {
        let mut bit_stream = BitStream::new();
        for k in -31000..=-1 {
            bit_stream.write_compressed_divisor(2, k);
            bit_stream.seek_start();
            let result = bit_stream.read_compressed_divisor(2);
            bit_stream.seek_start();
            assert_eq!(k, result);
        }
    }

    #[test]
    fn compressed_test_positive() {
        let mut bit_stream = BitStream::new();
        for k in 0..31000 {
            bit_stream.write_compressed_divisor(2, k);
            bit_stream.seek_start();
            let result = bit_stream.read_compressed_divisor(2);
            bit_stream.seek_start();

            assert_eq!(k, result, "k == {}", k);
        }
    }

    #[test]
    fn sanity() {
        let mut bit_stream = BitStream::new();

        let write_numbers = vec![1u32, 10, 15, 20, 25];
        let mut read_numbers: Vec<u32> = vec![];
        for &x in write_numbers.iter() {
            bit_stream.write(x);
        }
        bit_stream.bit_cursor = 0;
        for _ in 0..write_numbers.len() {
            read_numbers.push(bit_stream.read::<u32>() as u32);
        }
        assert_eq!(write_numbers, read_numbers);
    }

    #[test]
    fn boundary() {
        let mut bit_stream = BitStream::new();
        bit_stream.bit_cursor = 126;
        bit_stream.write::<u8>(7);
        bit_stream.bit_cursor = 126;
        let val = bit_stream.read::<u8>();
        assert_eq!(val, 7);
    }

    #[test]
    #[cfg(feature = "desktop")]
    fn shotgun_aligned() {
        let mut bit_stream = BitStream::new();

        let mut write_numbers: Vec<u32> = vec![];
        let mut read_numbers: Vec<u32> = vec![];

        for trial in 0..1000 {
            let length = rand::random::<u32>() % 1000;
            write_numbers.clear();
            read_numbers.clear();
            bit_stream.seek_start();

            // write_numbers.push(1);
            for _ in 0..length {
                write_numbers.push(rand::random::<u32>() % 10u32);
            }

            bit_stream.seek_start();
            // bit_stream.write_bit(1);
            for &x in write_numbers.iter() {
                bit_stream.write::<u32>(x);
            }

            bit_stream.seek_start();

            // read_numbers.push(bit_stream.read_bit() as u32);
            for _ in 0..write_numbers.len() {
                read_numbers.push(bit_stream.read::<u32>() as u32);
            }
            assert_eq!(write_numbers, read_numbers, "trial number: {}", trial + 1);
        }
    }

    #[test]
    #[cfg(feature = "desktop")]
    fn shotgun_unaligned() {
        let mut bit_stream = BitStream::new();

        let mut write_numbers: Vec<u32> = vec![];
        let mut read_numbers: Vec<u32> = vec![];

        for trial in 0..1000 {
            let length = rand::random::<u32>() % 1000;
            write_numbers.clear();
            read_numbers.clear();
            bit_stream.seek_start();

            write_numbers.push(1);
            for _ in 0..length {
                write_numbers.push(rand::random::<u32>() % 10u32);
            }

            bit_stream.seek_start();

            bit_stream.write_bit(1);
            for &x in write_numbers.iter().skip(1) {
                bit_stream.write::<u32>(x);
            }

            bit_stream.seek_start();

            read_numbers.push(bit_stream.read_bit() as u32);
            for _ in 0..write_numbers.len() - 1 {
                read_numbers.push(bit_stream.read::<u32>() as u32);
            }
            assert_eq!(write_numbers, read_numbers, "trial number: {}", trial + 1);
        }
    }
}
