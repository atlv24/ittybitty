//! [`IttyBitty<N>`] is a dynamically sized bit set that behaves akin to a `SmallVec<[usize; N]>`
//!
//! It holds `N * size_of::<usize>() - 1` bits inline. If a bit is set beyond that range, it will
//! allocate a buffer on the heap and stop using the inline bits.
//!
//! `N` must be 2 or greater.
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

const INLINE_BITS: usize = core::mem::size_of::<usize>() * 8;
const INLINE_BITS_POT: usize = INLINE_BITS.trailing_zeros() as usize;
const INLINE_BITS_MASK: usize = INLINE_BITS - 1;
const HEAP_FLAG: usize = 1 << (INLINE_BITS - 1);

/// A memory-access optimized dynamically sized bitset.
///
/// `IttyBitty<N>` is backed by a `[usize; N]`, but can spill to heap allocation.
pub struct IttyBitty<const N: usize> {
    data: [usize; N],
}

impl<const N: usize> IttyBitty<N> {
    const CAPACITY_WORD: usize = N - 1;
    const POINTER_WORD: usize = N - 2;
    const INLINE_CAPACITY: usize = INLINE_BITS * N - 1;

    #[inline(always)]
    const fn words_needed(bits: usize) -> usize {
        (bits + INLINE_BITS_MASK) >> INLINE_BITS_POT
    }

    /// Create an empty inline `IttyBitty`
    #[inline]
    pub const fn new() -> Self {
        const { assert!(N > 1) }
        Self { data: [0; N] }
    }

    #[inline]
    fn from_pointer(ptr: usize, cap: usize) -> Self {
        let mut data = [0; N];
        data[Self::POINTER_WORD] = ptr;
        data[Self::CAPACITY_WORD] = cap | HEAP_FLAG;
        Self { data }
    }

    #[inline]
    fn from_vec(v: Vec<usize>) -> Self {
        let ptr = v.as_ptr() as usize;
        let cap = v.capacity();
        core::mem::forget(v);
        Self::from_pointer(ptr, cap)
    }

    /// Create an empty inline `IttyBitty` with enough capacity to hold `bits`
    #[inline]
    pub fn with_capacity(bits: usize) -> Self {
        const { assert!(N > 1) }
        if bits <= Self::INLINE_CAPACITY {
            return Self::new();
        }
        Self::from_vec([0usize].repeat(Self::words_needed(bits)))
    }

    #[inline]
    fn spilled(&self) -> bool {
        self.data[Self::CAPACITY_WORD] & HEAP_FLAG != 0
    }

    #[inline]
    fn pointer(&self) -> *mut usize {
        debug_assert!(self.spilled());
        self.data[Self::POINTER_WORD] as *mut usize
    }

    #[inline]
    fn words(&self) -> usize {
        if self.spilled() {
            self.data[Self::CAPACITY_WORD] & !HEAP_FLAG
        } else {
            N
        }
    }

    #[inline]
    fn buffer_raw(&self) -> *mut [usize] {
        unsafe { core::slice::from_raw_parts_mut(self.pointer(), self.words()) }
    }

    #[inline]
    fn buffer_mut(&mut self) -> &mut [usize] {
        unsafe { &mut *self.buffer_raw() }
    }

    #[inline]
    fn buffer(&self) -> &[usize] {
        unsafe { &*self.buffer_raw() }
    }

    /// Get the current capacity of the `IttyBitty`
    #[inline]
    pub fn capacity(&self) -> usize {
        if self.spilled() {
            (self.data[Self::CAPACITY_WORD] & !HEAP_FLAG) * INLINE_BITS
        } else {
            Self::INLINE_CAPACITY
        }
    }

    unsafe fn get_word_unchecked(&self, word: usize) -> &usize {
        let slice = if self.spilled() {
            self.buffer()
        } else {
            self.data.as_slice()
        };
        unsafe { slice.get_unchecked(word) }
    }

    unsafe fn get_word_unchecked_mut(&mut self, word: usize) -> &mut usize {
        let slice = if self.spilled() {
            self.buffer_mut()
        } else {
            self.data.as_mut_slice()
        };
        unsafe { slice.get_unchecked_mut(word) }
    }

    /// Get the bit at `bit` without bounds checks.
    #[inline]
    pub unsafe fn get_unchecked(&self, bit: usize) -> bool {
        let w = bit >> INLINE_BITS_POT;
        let b = 1 << (bit & INLINE_BITS_MASK);
        unsafe { self.get_word_unchecked(w) & b != 0 }
    }

    /// Set the bit at `bit` without bounds checks.
    #[inline]
    pub unsafe fn set_unchecked(&mut self, bit: usize, val: bool) {
        let w = bit >> INLINE_BITS_POT;
        let b = 1 << (bit & INLINE_BITS_MASK);
        let word = unsafe { self.get_word_unchecked_mut(w) };
        if val {
            *word |= b;
        } else {
            *word &= !b;
        }
    }

    /// Get the bit at `bit`. Returns false if beyond capacity.
    #[inline]
    pub fn get(&self, bit: usize) -> bool {
        if bit < self.capacity() {
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
        unsafe {
            self.set_unchecked(bit, value);
        }
    }

    /// Set all bits to false.
    #[inline]
    pub fn clear(&mut self) {
        unsafe {
            for w in 0..self.words() {
                *self.get_word_unchecked_mut(w) = 0;
            }
        }
    }

    /// Set `bit` and all bits beyond it to false.
    pub fn truncate(&mut self, bit: usize) {
        unsafe {
            if bit < self.capacity() {
                let w = bit >> INLINE_BITS_POT;
                let b = bit & INLINE_BITS_MASK;
                *self.get_word_unchecked_mut(w) &= !(!0 << b);
                for w in (w + 1)..self.words() {
                    *self.get_word_unchecked_mut(w) = 0;
                }
            }
        }
    }

    fn reallocate(&mut self, bits: usize) {
        if bits <= self.capacity() {
            return;
        }

        let mut v = if self.spilled() {
            let words = self.words();
            unsafe { Vec::from_raw_parts(self.pointer(), words, words) }
        } else {
            self.data.to_vec()
        };

        v.resize(Self::words_needed(bits).max(v.capacity() + 1), 0);
        for _ in v.len()..v.capacity() {
            v.push(0);
        }

        self.data[Self::POINTER_WORD] = v.as_ptr() as usize;
        self.data[Self::CAPACITY_WORD] = v.capacity() | HEAP_FLAG;
        core::mem::forget(v);
    }

    /// Iterate over true bits.
    #[inline]
    pub fn iter(&self) -> Iter<N> {
        Iter { v: self, i: 0 }
    }

    /// Iterate over true bits backwards.
    #[inline]
    pub fn iter_rev(&self) -> IterRev<N> {
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
        let w = bit >> INLINE_BITS_POT;
        let b = bit & INLINE_BITS_MASK;

        let next = (unsafe { self.get_word_unchecked(w) } & (!0 << b)).trailing_zeros() as usize;
        if next < INLINE_BITS {
            return next + (w << INLINE_BITS_POT);
        }
        for w in (w + 1)..self.words() {
            let next = unsafe { self.get_word_unchecked(w) }.trailing_zeros() as usize;
            if next < INLINE_BITS {
                return next + (w << INLINE_BITS_POT);
            }
        }
        usize::MAX
    }

    /// Gets the first true bit before `bit`.
    pub fn prev_set_bit(&self, bit: usize) -> usize {
        if bit == 0 {
            return usize::MAX;
        }
        let bit = bit.min(self.capacity() - 1);
        let w = bit >> INLINE_BITS_POT;
        let b = bit & INLINE_BITS_MASK;
        let prev = (unsafe { self.get_word_unchecked(w) } & !(!0 << b)).leading_zeros() as usize;
        if prev < INLINE_BITS {
            return (w << INLINE_BITS_POT) + INLINE_BITS - 1 - prev;
        }
        for w in (0..w).rev() {
            let prev = unsafe { self.get_word_unchecked(w) }.leading_zeros() as usize;
            if prev < INLINE_BITS {
                return (w << INLINE_BITS_POT) + INLINE_BITS - 1 - prev;
            }
        }
        usize::MAX
    }
}

impl<const N: usize> Drop for IttyBitty<N> {
    fn drop(&mut self) {
        if self.spilled() {
            let words = self.words();
            unsafe { Vec::from_raw_parts(self.pointer(), words, words) };
        }
    }
}

impl<const N: usize> core::ops::Index<usize> for IttyBitty<N> {
    type Output = bool;

    #[inline(always)]
    fn index(&self, bit: usize) -> &Self::Output {
        if self.get(bit) {
            &true
        } else {
            &false
        }
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
        let words_a = self.words();
        let words_b = other.words();
        if words_a > words_b {
            for w in words_a..words_b {
                if unsafe { *other.get_word_unchecked(w) } != 0 {
                    return false;
                }
            }
        }
        if words_b > words_a {
            for w in words_b..words_a {
                if unsafe { *self.get_word_unchecked(w) } != 0 {
                    return false;
                }
            }
        }
        for w in 0..words_a {
            if unsafe { *self.get_word_unchecked(w) != *other.get_word_unchecked(w) } {
                return false;
            }
        }
        return true;
    }
}

impl<const N: usize> Eq for IttyBitty<N> {}

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
