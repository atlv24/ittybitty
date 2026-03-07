//! [`IttyBitty<N>`] is a dynamically sized bit set that behaves akin to a `SmallVec<[usize; N]>`
//!
//! It holds `N * size_of::<usize>() - 1` bits inline. If a bit is set beyond that range, it will
//! allocate a buffer on the heap and stop using the inline bits.
//!
//! `N` must be 1 or greater.
//!
//! # Example
//!
//! ```
//! use ittybitty::IttyBitty;
//!
//! let mut v = IttyBitty::<2>::new();
//! v.set(4, true);
//!
//! assert_eq!(v.get(0), false);
//! assert_eq!(v.get(4), true);
//! ```

#![no_std]
#![doc(html_root_url = "https://docs.rs/ittybitty")]
#![crate_name = "ittybitty"]
#![warn(
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    clippy::shadow_unrelated
)]
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod test;

use alloc::vec::Vec;
use core::fmt;

const BITS_PER_WORD: usize = core::mem::size_of::<usize>() * 8;
const BITS_PER_WORD_POT: usize = BITS_PER_WORD.trailing_zeros() as usize;
const BITS_PER_WORD_MASK: usize = BITS_PER_WORD - 1;
/// Bit 0 of data[0]: 1 = inline, 0 = spilled (aligned pointer)
const INLINE_TAG: usize = 1;

/// A memory-access optimized dynamically sized bitset.
///
/// `IttyBitty<N>` is backed by a `[usize; N]`, but can spill to heap allocation.
pub struct IttyBitty<const N: usize> {
    data: [usize; N],
}

impl<const N: usize> IttyBitty<N> {
    /// Inline capacity in bits: N words * bits_per_word - 1 (tag bit)
    const INLINE_CAPACITY: usize = BITS_PER_WORD * N - 1;

    #[inline(always)]
    const fn words_needed(bits: usize) -> usize {
        (bits + BITS_PER_WORD_MASK) >> BITS_PER_WORD_POT
    }

    /// Create an empty inline `IttyBitty`
    #[inline]
    pub const fn new() -> Self {
        const { assert!(N > 0) }
        // Set the inline tag bit
        let mut data = [0usize; N];
        data[0] = INLINE_TAG;
        Self { data }
    }

    #[inline]
    fn from_heap(ptr: *mut usize, cap: usize) -> Self {
        let mut data = [0; N];
        // Bit 0 = 0 means spilled; the pointer is aligned so bit 0 is naturally 0
        data[0] = ptr as usize;
        if N > 1 {
            data[1] = cap;
        }
        Self { data }
    }

    /// Create an empty inline `IttyBitty` with enough capacity to hold `bits`
    #[inline]
    pub fn with_capacity(bits: usize) -> Self {
        const { assert!(N > 0) }
        if bits <= Self::INLINE_CAPACITY {
            return Self::new();
        }
        let words_needed = Self::words_needed(bits);
        if N > 1 {
            let mut v = alloc::vec![0usize; words_needed];
            // Ensure all capacity is initialized so Drop can reconstruct
            // Vec with len == capacity soundly.
            v.resize(v.capacity(), 0);
            let cap = v.capacity();
            let ptr = v.as_ptr() as *mut usize;
            core::mem::forget(v);
            Self::from_heap(ptr, cap)
        } else {
            // N=1: capacity prefix at ptr[0]
            let mut v = Vec::with_capacity(words_needed + 1);
            v.push(0usize); // placeholder for capacity prefix
            v.resize(words_needed + 1, 0);
            // Ensure all capacity is initialized
            v.resize(v.capacity(), 0);
            let data_words = v.capacity() - 1;
            // SAFETY: v is non-empty; ptr[0] is the capacity prefix.
            unsafe { *(v.as_mut_ptr()) = data_words };
            let ptr = v.as_ptr() as *mut usize;
            core::mem::forget(v);
            Self::from_heap(ptr, data_words)
        }
    }

    #[inline(always)]
    fn is_inline(&self) -> bool {
        self.data[0] & INLINE_TAG != 0
    }

    #[inline(always)]
    fn spilled(&self) -> bool {
        !self.is_inline()
    }

    #[inline(always)]
    fn pointer(&self) -> *mut usize {
        debug_assert!(self.spilled());
        self.data[0] as *mut usize
    }

    #[inline]
    fn data_words(&self) -> usize {
        if self.spilled() {
            if N > 1 {
                self.data[1]
            } else {
                // SAFETY: when spilled, pointer() is a valid heap allocation
                // and ptr[0] holds the capacity prefix (written during allocation).
                unsafe { *self.pointer() }
            }
        } else {
            N
        }
    }

    #[inline]
    fn heap_data_offset(&self) -> usize {
        if N > 1 { 0 } else { 1 }
    }

    /// Returns a raw pointer to the heap data slice.
    ///
    /// # Safety (caller obligation)
    /// Must only be called when `self.spilled()` is true.
    #[inline]
    fn buffer_raw(&self) -> *mut [usize] {
        let offset = self.heap_data_offset();
        let words = self.data_words();
        // SAFETY: when spilled, pointer() returns a valid heap allocation of at
        // least `offset + words` usizes. Caller guarantees we are spilled.
        unsafe {
            core::ptr::slice_from_raw_parts_mut(self.pointer().add(offset), words)
        }
    }

    /// # Safety (caller obligation)
    /// Must only be called when `self.spilled()` is true.
    #[inline]
    fn buffer_mut(&mut self) -> &mut [usize] {
        // SAFETY: spilled precondition forwarded from caller; &mut self
        // guarantees exclusive access.
        unsafe { &mut *self.buffer_raw() }
    }

    /// # Safety (caller obligation)
    /// Must only be called when `self.spilled()` is true.
    #[inline]
    fn buffer(&self) -> &[usize] {
        // SAFETY: spilled precondition forwarded from caller.
        unsafe { &*self.buffer_raw() }
    }

    /// Get the current capacity of the `IttyBitty`
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data_words() * BITS_PER_WORD - if self.is_inline() { 1 } else { 0 }
    }

    /// # Safety
    ///
    /// `word` must be less than `self.data_words()`.
    unsafe fn get_word_unchecked(&self, word: usize) -> &usize {
        if self.spilled() {
            // SAFETY: caller ensures word < data_words(); buffer has data_words() elements.
            unsafe { &*self.buffer_raw().cast::<usize>().add(word) }
        } else {
            // SAFETY: caller ensures word < data_words() = N, so word < N.
            unsafe { self.data.get_unchecked(word) }
        }
    }

    /// # Safety
    ///
    /// `word` must be less than `self.data_words()`.
    unsafe fn get_word_unchecked_mut(&mut self, word: usize) -> &mut usize {
        if self.spilled() {
            // SAFETY: caller ensures word < data_words(); buffer has data_words() elements.
            unsafe { &mut *self.buffer_raw().cast::<usize>().add(word) }
        } else {
            // SAFETY: caller ensures word < data_words() = N, so word < N.
            unsafe { self.data.get_unchecked_mut(word) }
        }
    }

    /// Inline bit index `b` maps to raw bit `b + 1` in the data array
    /// (because bit 0 is the tag). For spilled, bit `b` maps directly.
    #[inline(always)]
    fn bit_coords(&self, bit: usize) -> (usize, usize) {
        if self.is_inline() {
            let raw = bit + 1;
            (raw >> BITS_PER_WORD_POT, 1 << (raw & BITS_PER_WORD_MASK))
        } else {
            (bit >> BITS_PER_WORD_POT, 1 << (bit & BITS_PER_WORD_MASK))
        }
    }

    /// Get the bit at `bit` without bounds checks.
    ///
    /// # Safety
    /// `bit` must be less than `self::capacity()`.
    #[inline]
    pub unsafe fn get_unchecked(&self, bit: usize) -> bool {
        let (w, mask) = self.bit_coords(bit);
        // SAFETY: bit < capacity() implies w < data_words().
        unsafe { self.get_word_unchecked(w) & mask != 0 }
    }

    /// Set the bit at `bit` without bounds checks.
    ///
    /// # Safety
    /// `bit` must be less than `self::capacity()`.
    #[inline]
    pub unsafe fn set_unchecked(&mut self, bit: usize, val: bool) {
        let (w, mask) = self.bit_coords(bit);
        // SAFETY: bit < capacity() implies w < data_words().
        let word = unsafe { self.get_word_unchecked_mut(w) };
        if val {
            *word |= mask;
        } else {
            *word &= !mask;
        }
    }

    /// Get the bit at `bit`. Returns false if beyond capacity.
    #[inline]
    pub fn get(&self, bit: usize) -> bool {
        if bit < self.capacity() {
            // SAFETY: just checked bit < capacity().
            unsafe { self.get_unchecked(bit) }
        } else {
            false
        }
    }

    /// Set the bit at `bit` to `value`.
    /// Extends capacity as needed if `value` is `true`, no-op if `false` and beyond bounds.
    #[inline]
    pub fn set(&mut self, bit: usize, value: bool) {
        if bit >= self.capacity() {
            if !value {
                return;
            }
            self.reallocate(bit + 1);
        }
        // SAFETY: bit < capacity() (either it was already, or reallocate expanded).
        unsafe {
            self.set_unchecked(bit, value);
        }
    }

    /// Returns a new `IttyBitty` with `bit` set to true.
    #[inline]
    #[must_use]
    pub fn with(mut self, bit: usize) -> Self {
        self.set(bit, true);
        self
    }

    /// Returns a new `IttyBitty` with `bit` set to false.
    #[inline]
    #[must_use]
    pub fn without(mut self, bit: usize) -> Self {
        self.set(bit, false);
        self
    }

    /// Set all bits to false.
    #[inline]
    pub fn clear(&mut self) {
        if self.spilled() {
            let buf = self.buffer_mut();
            for w in buf.iter_mut() {
                *w = 0;
            }
        } else {
            // Preserve the tag bit
            self.data[0] = INLINE_TAG;
            for i in 1..N {
                self.data[i] = 0;
            }
        }
    }

    /// Set `bit` and all bits beyond it to false.
    pub fn truncate(&mut self, bit: usize) {
        if bit >= self.capacity() {
            return;
        }
        let (w, _) = self.bit_coords(bit);
        if self.is_inline() {
            let raw = bit + 1;
            let b = raw & BITS_PER_WORD_MASK;
            // Clear bits at and above `raw` in word `w`
            if b > 0 {
                self.data[w] &= !(!0usize << b);
            } else {
                self.data[w] = 0;
            }
            // Preserve tag if w == 0
            if w == 0 {
                self.data[0] |= INLINE_TAG;
            }
            for i in (w + 1)..N {
                self.data[i] = 0;
            }
        } else {
            // SAFETY: bit < capacity() so w = bit/BITS_PER_WORD < data_words(),
            // and loop indices w+1..words are also < data_words().
            unsafe {
                let b = bit & BITS_PER_WORD_MASK;
                let words = self.data_words();
                let buf_w = self.get_word_unchecked_mut(w);
                if b > 0 {
                    *buf_w &= !(!0usize << b);
                } else {
                    *buf_w = 0;
                }
                for i in (w + 1)..words {
                    *self.get_word_unchecked_mut(i) = 0;
                }
            }
        }
    }

    fn reallocate(&mut self, bits: usize) {
        if bits <= self.capacity() {
            return;
        }

        let new_words = Self::words_needed(bits);

        if self.spilled() {
            let offset = self.heap_data_offset();
            let old_words = self.data_words();
            let old_alloc = old_words + offset;
            let new_alloc = new_words.max(old_alloc + 1) + offset;
            // SAFETY: pointer was originally leaked from a Vec with this
            // layout. old_alloc = old_words + offset = total element count.
            let mut v = unsafe {
                Vec::from_raw_parts(self.pointer(), old_alloc, old_alloc)
            };
            v.resize(new_alloc, 0);
            v.extend(core::iter::repeat_n(0, v.capacity() - v.len()));
            if N > 1 {
                self.data[1] = v.capacity() - offset;
            } else {
                // SAFETY: v is non-empty (was resized above); ptr[0] is the capacity prefix.
                unsafe { *(v.as_mut_ptr()) = v.capacity() - offset };
            }
            self.data[0] = v.as_ptr() as usize;
            core::mem::forget(v);
        } else {
            // Transitioning from inline to heap.
            // Inline bits are at raw position b+1 (tag at bit 0).
            // Spilled bits are at position b. Shift right by 1.
            let target_words = new_words.max(N + 1);
            if N > 1 {
                let mut v = Vec::<usize>::with_capacity(target_words);
                // Shift all inline words right by 1 bit
                for i in 0..N {
                    let curr = self.data[i] >> 1;
                    let carry = if i + 1 < N {
                        self.data[i + 1] << (BITS_PER_WORD - 1)
                    } else {
                        0
                    };
                    v.push(curr | carry);
                }
                v.resize(v.capacity(), 0);
                let cap = v.capacity();
                let ptr = v.as_ptr() as *mut usize;
                core::mem::forget(v);
                self.data[0] = ptr as usize;
                self.data[1] = cap;
            } else {
                // N=1: capacity prefix at [0], data at [1..]
                let mut v = Vec::with_capacity(target_words + 1);
                v.push(0usize); // placeholder for capacity
                v.push(self.data[0] >> 1); // shift right by 1 to remove tag
                v.resize(v.capacity(), 0);
                let data_words = v.capacity() - 1;
                // SAFETY: v has at least 2 elements; ptr[0] is the capacity prefix.
                unsafe { *(v.as_mut_ptr()) = data_words };
                let ptr = v.as_ptr() as *mut usize;
                core::mem::forget(v);
                self.data[0] = ptr as usize;
            }
        }
    }

    /// Iterate over true bits.
    #[inline]
    pub fn iter(&self) -> Iter<'_, N> {
        Iter { v: self, i: 0 }
    }

    /// Iterate over true bits backwards.
    #[inline]
    pub fn iter_rev(&self) -> IterRev<'_, N> {
        IterRev {
            v: self,
            i: self.capacity(),
        }
    }

    /// Gets the first true bit at or after `bit`.
    pub fn next_set_bit(&self, bit: usize) -> usize {
        if bit >= self.capacity() {
            return usize::MAX;
        }

        if self.is_inline() {
            let raw = bit + 1;
            let w = raw >> BITS_PER_WORD_POT;
            let b = raw & BITS_PER_WORD_MASK;
            // Mask off bits below `raw` in this word
            let masked = self.data[w] & (!0usize << b);
            let tz = masked.trailing_zeros() as usize;
            if tz < BITS_PER_WORD {
                return (w << BITS_PER_WORD_POT) + tz - 1; // -1 to undo the +1 offset
            }
            for w in (w + 1)..N {
                let tz = self.data[w].trailing_zeros() as usize;
                if tz < BITS_PER_WORD {
                    return (w << BITS_PER_WORD_POT) + tz - 1;
                }
            }
            usize::MAX
        } else {
            let w = bit >> BITS_PER_WORD_POT;
            let b = bit & BITS_PER_WORD_MASK;
            let words = self.data_words();

            // SAFETY: bit < capacity() so w = bit/BITS_PER_WORD < data_words().
            let masked = (unsafe { self.get_word_unchecked(w) } & (!0usize << b)).trailing_zeros() as usize;
            if masked < BITS_PER_WORD {
                return masked + (w << BITS_PER_WORD_POT);
            }
            for w in (w + 1)..words {
                // SAFETY: w < words = data_words().
                let tz = unsafe { self.get_word_unchecked(w) }.trailing_zeros() as usize;
                if tz < BITS_PER_WORD {
                    return tz + (w << BITS_PER_WORD_POT);
                }
            }
            usize::MAX
        }
    }

    /// Returns the logical word at index `w`, adjusted for the tag offset.
    /// Returns 0 if `w` is beyond the data.
    #[inline]
    fn logical_word(&self, w: usize) -> usize {
        if w >= self.data_words() {
            return 0;
        }
        if self.is_inline() {
            // Inline: w < data_words() = N, so data[w] is in bounds.
            // w + 1 < N is checked before accessing data[w + 1].
            let curr = self.data[w] >> 1;
            let carry = if w + 1 < N {
                self.data[w + 1] << (BITS_PER_WORD - 1)
            } else {
                0
            };
            curr | carry
        } else {
            // SAFETY: w < data_words() checked above.
            unsafe { *self.get_word_unchecked(w) }
        }
    }

    /// Construct from a slice of logical (tag-free) words.
    fn from_logical_words(words: &[usize]) -> Self {
        let len = words
            .iter()
            .rposition(|&w| w != 0)
            .map_or(0, |i| i + 1);
        if len == 0 {
            return Self::new();
        }
        if len <= N {
            let top = words[len - 1];
            let highest =
                (len - 1) * BITS_PER_WORD + (BITS_PER_WORD - 1 - top.leading_zeros() as usize);
            if highest < Self::INLINE_CAPACITY {
                let mut data = [0usize; N];
                data[0] = (words[0] << 1) | INLINE_TAG;
                for i in 1..N {
                    let prev_carry = words[i - 1] >> (BITS_PER_WORD - 1);
                    let curr = if i < len { words[i] << 1 } else { 0 };
                    data[i] = curr | prev_carry;
                }
                return Self { data };
            }
        }
        let mut result = Self::with_capacity(len * BITS_PER_WORD);
        let buf = result.buffer_mut();
        buf[..len].copy_from_slice(&words[..len]);
        result
    }

    /// Returns `true` if no bits are set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        if self.is_inline() {
            if self.data[0] != INLINE_TAG {
                return false;
            }
            for i in 1..N {
                if self.data[i] != 0 {
                    return false;
                }
            }
            true
        } else {
            let words = self.data_words();
            for w in 0..words {
                // SAFETY: w < words = data_words().
                if unsafe { *self.get_word_unchecked(w) } != 0 {
                    return false;
                }
            }
            true
        }
    }

    /// Returns `true` if `self` and `other` have any bits in common.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        if self.is_inline() && other.is_inline() {
            if self.data[0] & other.data[0] & !INLINE_TAG != 0 {
                return true;
            }
            for i in 1..N {
                if self.data[i] & other.data[i] != 0 {
                    return true;
                }
            }
            return false;
        }
        let words = self.data_words().min(other.data_words());
        for w in 0..words {
            if self.logical_word(w) & other.logical_word(w) != 0 {
                return true;
            }
        }
        false
    }

    /// Returns a new bitset containing only the bits set in both `self` and `other`.
    pub fn intersection(&self, other: &Self) -> Self {
        if self.is_inline() && other.is_inline() {
            let mut data = [0usize; N];
            data[0] = (self.data[0] & other.data[0]) | INLINE_TAG;
            for i in 1..N {
                data[i] = self.data[i] & other.data[i];
            }
            return Self { data };
        }
        let words = self.data_words().min(other.data_words());
        let mut v = Vec::with_capacity(words);
        for w in 0..words {
            v.push(self.logical_word(w) & other.logical_word(w));
        }
        Self::from_logical_words(&v)
    }

    /// Returns a new bitset containing the bits set in either `self` or `other`.
    pub fn union(&self, other: &Self) -> Self {
        if self.is_inline() && other.is_inline() {
            let mut data = [0usize; N];
            data[0] = self.data[0] | other.data[0];
            for i in 1..N {
                data[i] = self.data[i] | other.data[i];
            }
            return Self { data };
        }
        let words = self.data_words().max(other.data_words());
        let mut v = Vec::with_capacity(words);
        for w in 0..words {
            v.push(self.logical_word(w) | other.logical_word(w));
        }
        Self::from_logical_words(&v)
    }

    /// Returns a new bitset containing the bits set in exactly one of `self` or `other`.
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        if self.is_inline() && other.is_inline() {
            let mut data = [0usize; N];
            data[0] = (self.data[0] ^ other.data[0]) | INLINE_TAG;
            for i in 1..N {
                data[i] = self.data[i] ^ other.data[i];
            }
            return Self { data };
        }
        let words = self.data_words().max(other.data_words());
        let mut v = Vec::with_capacity(words);
        for w in 0..words {
            v.push(self.logical_word(w) ^ other.logical_word(w));
        }
        Self::from_logical_words(&v)
    }

    /// Gets the first true bit before `bit`.
    pub fn prev_set_bit(&self, bit: usize) -> usize {
        if bit == 0 {
            return usize::MAX;
        }
        let bit = bit.min(self.capacity());

        if self.is_inline() {
            let raw = bit + 1;
            let w = raw >> BITS_PER_WORD_POT;
            let b = raw & BITS_PER_WORD_MASK;
            if b > 0 {
                let mut masked = self.data[w] & !(!0usize << b);
                if w == 0 {
                    masked &= !INLINE_TAG;
                }
                let lz = masked.leading_zeros() as usize;
                if lz < BITS_PER_WORD {
                    return (w << BITS_PER_WORD_POT) + BITS_PER_WORD - 1 - lz - 1;
                }
            }
            // Scan words above 0 without tag masking
            for w in (1..w).rev() {
                let lz = self.data[w].leading_zeros() as usize;
                if lz < BITS_PER_WORD {
                    return (w << BITS_PER_WORD_POT) + BITS_PER_WORD - 1 - lz - 1;
                }
            }
            // Handle word 0 separately: mask tag bit
            if w > 0 {
                let lz = (self.data[0] & !INLINE_TAG).leading_zeros() as usize;
                if lz < BITS_PER_WORD {
                    return BITS_PER_WORD - 1 - lz - 1;
                }
            }
            usize::MAX
        } else {
            let w = bit >> BITS_PER_WORD_POT;
            let b = bit & BITS_PER_WORD_MASK;
            if b > 0 {
                // SAFETY: bit <= capacity(), so w = bit/BITS_PER_WORD <= data_words().
                // When b > 0, w < data_words() (since w*BITS + b <= capacity() and b > 0).
                let prev = (unsafe { self.get_word_unchecked(w) } & !(!0usize << b)).leading_zeros() as usize;
                if prev < BITS_PER_WORD {
                    return (w << BITS_PER_WORD_POT) + BITS_PER_WORD - 1 - prev;
                }
            }
            for w in (0..w).rev() {
                // SAFETY: w < data_words() (loop bound is at most data_words()).
                let prev = unsafe { self.get_word_unchecked(w) }.leading_zeros() as usize;
                if prev < BITS_PER_WORD {
                    return (w << BITS_PER_WORD_POT) + BITS_PER_WORD - 1 - prev;
                }
            }
            usize::MAX
        }
    }
}

impl<const N: usize> Clone for IttyBitty<N> {
    fn clone(&self) -> Self {
        if self.spilled() {
            let buffer = self.buffer().to_vec();
            if N > 1 {
                let mut buffer = buffer;
                // Ensure all capacity is initialized
                buffer.resize(buffer.capacity(), 0);
                let cap = buffer.capacity();
                let ptr = buffer.as_ptr() as *mut usize;
                core::mem::forget(buffer);
                Self::from_heap(ptr, cap)
            } else {
                // N=1: need capacity prefix
                let words = buffer.len();
                let mut v = Vec::with_capacity(words + 1);
                v.push(0usize); // placeholder
                v.extend_from_slice(&buffer);
                // Ensure all capacity is initialized
                v.resize(v.capacity(), 0);
                let data_words = v.capacity() - 1;
                // SAFETY: v is non-empty; ptr[0] is the capacity prefix.
                unsafe { *(v.as_mut_ptr()) = data_words };
                let ptr = v.as_ptr() as *mut usize;
                core::mem::forget(v);
                Self::from_heap(ptr, data_words)
            }
        } else {
            Self { data: self.data }
        }
    }
}

impl<const N: usize> Drop for IttyBitty<N> {
    fn drop(&mut self) {
        if self.spilled() {
            let offset = self.heap_data_offset();
            let words = self.data_words();
            let total = words + offset;
            // SAFETY: pointer was originally leaked from a Vec with this layout.
            // total = data words + offset (capacity prefix for N=1).
            unsafe { Vec::from_raw_parts(self.pointer(), total, total) };
        }
    }
}

impl<const N: usize> core::ops::Index<usize> for IttyBitty<N> {
    type Output = bool;

    #[inline(always)]
    fn index(&self, bit: usize) -> &Self::Output {
        if self.get(bit) { &true } else { &false }
    }
}

impl<const N: usize> Default for IttyBitty<N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Debug for IttyBitty<N> {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_list().entries(self.iter()).finish()
    }
}

impl<const N: usize> PartialEq for IttyBitty<N> {
    fn eq(&self, other: &Self) -> bool {
        let words_a = self.data_words();
        let words_b = other.data_words();
        let min_words = words_a.min(words_b);

        for w in min_words..words_a {
            if self.logical_word(w) != 0 {
                return false;
            }
        }
        for w in min_words..words_b {
            if other.logical_word(w) != 0 {
                return false;
            }
        }
        for w in 0..min_words {
            if self.logical_word(w) != other.logical_word(w) {
                return false;
            }
        }
        true
    }
}

impl<const N: usize> Eq for IttyBitty<N> {}

impl<const N: usize> core::ops::BitAnd for &IttyBitty<N> {
    type Output = IttyBitty<N>;

    #[inline]
    fn bitand(self, rhs: Self) -> IttyBitty<N> {
        self.intersection(rhs)
    }
}

impl<const N: usize> core::ops::BitOr for &IttyBitty<N> {
    type Output = IttyBitty<N>;

    #[inline]
    fn bitor(self, rhs: Self) -> IttyBitty<N> {
        self.union(rhs)
    }
}

impl<const N: usize> core::ops::BitXor for &IttyBitty<N> {
    type Output = IttyBitty<N>;

    #[inline]
    fn bitxor(self, rhs: Self) -> IttyBitty<N> {
        self.symmetric_difference(rhs)
    }
}

impl<const N: usize> FromIterator<usize> for IttyBitty<N> {
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        let mut b = Self::new();
        for bit in iter {
            b.set(bit, true);
        }
        b
    }
}

impl<const N: usize> IntoIterator for IttyBitty<N> {
    type Item = usize;
    type IntoIter = IntoIter<N>;

    #[inline]
    fn into_iter(self) -> IntoIter<N> {
        IntoIter { i: 0, v: self }
    }
}

impl<'a, const N: usize> IntoIterator for &'a IttyBitty<N> {
    type Item = usize;
    type IntoIter = Iter<'a, N>;

    #[inline]
    fn into_iter(self) -> Iter<'a, N> {
        self.iter()
    }
}

/// IttyBitty owned iterator
#[derive(Debug)]
pub struct IntoIter<const N: usize> {
    v: IttyBitty<N>,
    i: usize,
}

impl<const N: usize> Iterator for IntoIter<N> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        let i = self.v.next_set_bit(self.i);
        self.i = i;
        if i == usize::MAX {
            return None;
        }
        self.i = i + 1;
        Some(i)
    }
}

/// IttyBitty reference iterator
#[derive(Debug)]
pub struct Iter<'a, const N: usize> {
    v: &'a IttyBitty<N>,
    i: usize,
}

impl<'a, const N: usize> Iterator for Iter<'a, N> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        self.i = self.v.next_set_bit(self.i);
        if self.i == usize::MAX {
            return None;
        }
        let i = self.i;
        self.i += 1;
        Some(i)
    }
}

/// IttyBitty reverse iterator
#[derive(Debug)]
pub struct IterRev<'a, const N: usize> {
    v: &'a IttyBitty<N>,
    i: usize,
}

impl<'a, const N: usize> Iterator for IterRev<'a, N> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        if self.i == usize::MAX {
            return None;
        }
        self.i = self.v.prev_set_bit(self.i);
        if self.i == usize::MAX {
            return None;
        }
        Some(self.i)
    }
}
