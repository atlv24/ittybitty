#[cfg(test)]
#[generic_tests::define]
mod tests {
    use core::usize;

    use crate::IttyBitty;
    use alloc::vec::Vec;

    #[test]
    fn test_inline_set_get<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        for x in 0..127 {
            for y in 0..127 {
                assert_eq!(b.get(y), false);
            }
            b.set(x, true);
            for y in 0..127 {
                assert_eq!(b.get(y), x == y);
            }
            b.set(x, false);
        }
    }

    #[test]
    fn test_spilling<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        for x in 0..258 {
            for y in 0..258 {
                assert_eq!(b.get(y), false);
            }
            b.set(x, true);
            for y in 0..258 {
                assert_eq!(b.get(y), x == y);
            }
            b.set(x, false);
        }
    }

    #[test]
    fn test_spilling_reverse<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        for x in (0..258).rev() {
            for y in 0..258 {
                assert_eq!(b.get(y), false);
            }
            b.set(x, true);
            for y in 0..258 {
                assert_eq!(b.get(y), x == y);
            }
            b.set(x, false);
        }
    }

    #[test]
    fn test_truncate<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        b.set(39, true);
        b.set(40, true);
        b.set(41, true);
        b.truncate(40);
        assert_eq!(b.get(39), true);
        assert_eq!(b.get(40), false);
        assert_eq!(b.get(41), false);
    }

    #[test]
    fn test_truncate_spilled<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        b.set(239, true);
        b.set(240, true);
        b.set(241, true);
        b.truncate(240);
        assert_eq!(b.get(239), true);
        assert_eq!(b.get(240), false);
        assert_eq!(b.get(241), false);
    }

    #[test]
    fn test_iter<const N: usize>() {
        let numbers: [usize; 8] = [3, 17, 127, 128, 340, 600, 942, 1732];
        for i in 0..numbers.len() {
            let mut b = IttyBitty::<N>::new();
            for &n in numbers.iter().take(i) {
                b.set(n, true);
            }
            assert_eq!(
                b.iter().collect::<Vec<_>>(),
                numbers.iter().take(i).map(|&u| u).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_iter_rev<const N: usize>() {
        let numbers: [usize; 8] = [3, 17, 127, 128, 340, 600, 942, 1732];
        for i in 0..numbers.len() {
            let mut b = IttyBitty::<N>::new();
            for &n in numbers.iter().take(i) {
                b.set(n, true);
            }
            assert_eq!(
                b.iter_rev().collect::<Vec<_>>(),
                numbers.iter().take(i).rev().map(|&u| u).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_into_iter<const N: usize>() {
        let numbers: [usize; 8] = [3, 17, 127, 128, 340, 600, 942, 1732];
        for i in 0..numbers.len() {
            let mut b = IttyBitty::<N>::new();
            for &n in numbers.iter().take(i) {
                b.set(n, true);
            }
            assert_eq!(
                b.into_iter().collect::<Vec<_>>(),
                numbers.iter().take(i).map(|&u| u).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_with_capacity<const N: usize>() {
        let numbers: [usize; 8] = [3, 17, 127, 128, 340, 600, 942, 1732];
        for i in 0..numbers.len() {
            let mut b = IttyBitty::<N>::with_capacity(numbers[i]);
            for &n in numbers.iter() {
                b.set(n, true);
            }
            assert_eq!(
                b.iter().collect::<Vec<_>>(),
                numbers.iter().map(|&u| u).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_next_set_bit<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        assert_eq!(b.next_set_bit(usize::MAX), usize::MAX);
        assert_eq!(b.next_set_bit(2), usize::MAX);
        assert_eq!(b.next_set_bit(0), usize::MAX);
        b.set(1, true);
        assert_eq!(b.next_set_bit(0), 1);
        assert_eq!(b.next_set_bit(1), 1);
        assert_eq!(b.next_set_bit(2), usize::MAX);
        b.set(2, true);
        assert_eq!(b.next_set_bit(0), 1);
        assert_eq!(b.next_set_bit(1), 1);
        assert_eq!(b.next_set_bit(2), 2);
        b.set(1, false);
        assert_eq!(b.next_set_bit(0), 2);
        assert_eq!(b.next_set_bit(1), 2);
        b.set(0, true);
        assert_eq!(b.next_set_bit(0), 0);
        assert_eq!(b.next_set_bit(1), 2);
        b.set(10, true);
        assert_eq!(b.next_set_bit(0), 0);
        assert_eq!(b.next_set_bit(1), 2);
        assert_eq!(b.next_set_bit(2), 2);
        assert_eq!(b.next_set_bit(9), 10);
        assert_eq!(b.next_set_bit(10), 10);
        b.set(1000, true);
        assert_eq!(b.next_set_bit(9), 10);
        assert_eq!(b.next_set_bit(10), 10);
        assert_eq!(b.next_set_bit(999), 1000);
        assert_eq!(b.next_set_bit(1000), 1000);
        assert_eq!(b.next_set_bit(1001), usize::MAX);
    }

    #[test]
    fn test_prev_set_bit<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        assert_eq!(b.prev_set_bit(usize::MAX), usize::MAX);
        assert_eq!(b.prev_set_bit(2), usize::MAX);
        assert_eq!(b.prev_set_bit(0), usize::MAX);
        b.set(1, true);
        assert_eq!(b.prev_set_bit(0), usize::MAX);
        assert_eq!(b.prev_set_bit(1), usize::MAX);
        assert_eq!(b.prev_set_bit(2), 1);
        assert_eq!(b.prev_set_bit(usize::MAX), 1);
        b.set(2, true);
        assert_eq!(b.prev_set_bit(1), usize::MAX);
        assert_eq!(b.prev_set_bit(2), 1);
        assert_eq!(b.prev_set_bit(3), 2);
        b.set(1, false);
        assert_eq!(b.prev_set_bit(1), usize::MAX);
        assert_eq!(b.prev_set_bit(3), 2);
        assert_eq!(b.prev_set_bit(2), usize::MAX);
        b.set(0, true);
        assert_eq!(b.prev_set_bit(0), usize::MAX);
        assert_eq!(b.prev_set_bit(1), 0);
        b.set(10, true);
        assert_eq!(b.prev_set_bit(2), 0);
        assert_eq!(b.prev_set_bit(9), 2);
        assert_eq!(b.prev_set_bit(10), 2);
        assert_eq!(b.prev_set_bit(11), 10);
        b.set(1000, true);
        assert_eq!(b.prev_set_bit(10), 2);
        assert_eq!(b.prev_set_bit(11), 10);
        assert_eq!(b.prev_set_bit(999), 10);
        assert_eq!(b.prev_set_bit(1000), 10);
        assert_eq!(b.prev_set_bit(1001), 1000);
    }

    #[instantiate_tests(<2>)]
    mod n2 {}
    #[instantiate_tests(<3>)]
    mod n3 {}
    #[instantiate_tests(<4>)]
    mod n4 {}
    #[instantiate_tests(<6>)]
    mod n6 {}
}
