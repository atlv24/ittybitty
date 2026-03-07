#[cfg(test)]
#[generic_tests::define]
mod tests {
    use crate::IttyBitty;
    use alloc::vec::Vec;

    #[test]
    fn test_inline_set_get<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        for x in 0..127 {
            for y in 0..127 {
                assert!(!b.get(y));
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
                assert!(!b.get(y));
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
                assert!(!b.get(y));
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
        assert!(b.get(39));
        assert!(!b.get(40));
        assert!(!b.get(41));
    }

    #[test]
    fn test_truncate_spilled<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        b.set(239, true);
        b.set(240, true);
        b.set(241, true);
        b.truncate(240);
        assert!(b.get(239));
        assert!(!b.get(240));
        assert!(!b.get(241));
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
                numbers.iter().take(i).copied().collect::<Vec<_>>()
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
                numbers.iter().take(i).rev().copied().collect::<Vec<_>>()
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
                numbers.iter().take(i).copied().collect::<Vec<_>>()
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
                numbers.to_vec()
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

    #[test]
    fn test_prev_set_bit_highest_inline<const N: usize>() {
        let mut b = IttyBitty::<N>::new();
        let highest = b.capacity() - 1;
        b.set(highest, true);
        assert_eq!(b.prev_set_bit(highest + 1), highest);
        assert_eq!(b.prev_set_bit(usize::MAX), highest);
        assert_eq!(b.iter_rev().collect::<Vec<_>>(), alloc::vec![highest]);
    }

    #[test]
    fn test_is_empty<const N: usize>() {
        let b = IttyBitty::<N>::new();
        assert!(b.is_empty());
        let mut b = IttyBitty::<N>::new();
        b.set(0, true);
        assert!(!b.is_empty());
        b.set(0, false);
        assert!(b.is_empty());
        b.set(500, true);
        assert!(!b.is_empty());
        b.set(500, false);
        assert!(b.is_empty());
    }

    #[test]
    fn test_intersects_inline_inline<const N: usize>() {
        let mut a = IttyBitty::<N>::new();
        let mut b = IttyBitty::<N>::new();
        assert!(!a.intersects(&b));
        a.set(5, true);
        assert!(!a.intersects(&b));
        b.set(10, true);
        assert!(!a.intersects(&b));
        b.set(5, true);
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_intersects_inline_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(5);
        let b = IttyBitty::<N>::new().with(5).with(500);
        assert!(a.intersects(&b));
        let a2 = IttyBitty::<N>::new().with(10);
        assert!(!a2.intersects(&b));
    }

    #[test]
    fn test_intersects_spilled_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(5).with(500);
        let b = IttyBitty::<N>::new().with(5);
        assert!(a.intersects(&b));
        let b2 = IttyBitty::<N>::new().with(10);
        assert!(!a.intersects(&b2));
    }

    #[test]
    fn test_intersects_spilled_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(5).with(500);
        let b = IttyBitty::<N>::new().with(10).with(500);
        assert!(a.intersects(&b));
        let c = IttyBitty::<N>::new().with(10).with(600);
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_intersection_inline_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(10).with(42);
        let b = IttyBitty::<N>::new().with(10).with(42).with(99);
        let c = a.intersection(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![10, 42]);
        assert_eq!(&a & &b, c);
    }

    #[test]
    fn test_intersection_inline_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(10);
        let b = IttyBitty::<N>::new().with(3).with(500);
        let c = a.intersection(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3]);
    }

    #[test]
    fn test_intersection_spilled_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(500);
        let b = IttyBitty::<N>::new().with(3).with(10);
        let c = a.intersection(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3]);
    }

    #[test]
    fn test_intersection_spilled_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(500);
        let b = IttyBitty::<N>::new().with(3).with(600);
        let c = a.intersection(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3]);
    }

    #[test]
    fn test_union_inline_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(10);
        let b = IttyBitty::<N>::new().with(10).with(42);
        let c = a.union(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3, 10, 42]);
        assert_eq!(&a | &b, c);
    }

    #[test]
    fn test_union_inline_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3);
        let b = IttyBitty::<N>::new().with(500);
        let c = a.union(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3, 500]);
    }

    #[test]
    fn test_union_spilled_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(500);
        let b = IttyBitty::<N>::new().with(10);
        let c = a.union(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3, 10, 500]);
    }

    #[test]
    fn test_union_spilled_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(500);
        let b = IttyBitty::<N>::new().with(10).with(600);
        let c = a.union(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3, 10, 500, 600]);
    }

    #[test]
    fn test_symmetric_difference_inline_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(10);
        let b = IttyBitty::<N>::new().with(10).with(42);
        let c = a.symmetric_difference(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![3, 42]);
        assert_eq!(&a ^ &b, c);
    }

    #[test]
    fn test_symmetric_difference_inline_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3);
        let b = IttyBitty::<N>::new().with(3).with(500);
        let c = a.symmetric_difference(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![500]);
    }

    #[test]
    fn test_symmetric_difference_spilled_inline<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(500);
        let b = IttyBitty::<N>::new().with(3);
        let c = a.symmetric_difference(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![500]);
    }

    #[test]
    fn test_symmetric_difference_spilled_spilled<const N: usize>() {
        let a = IttyBitty::<N>::new().with(3).with(500);
        let b = IttyBitty::<N>::new().with(3).with(600);
        let c = a.symmetric_difference(&b);
        assert_eq!(c.iter().collect::<Vec<_>>(), alloc::vec![500, 600]);
    }

    #[test]
    fn test_from_iter<const N: usize>() {
        let bits = [3, 17, 42, 100];
        let b: IttyBitty<N> = bits.iter().copied().collect();
        assert_eq!(b.iter().collect::<Vec<_>>(), alloc::vec![3, 17, 42, 100]);
    }

    #[test]
    fn test_eq_different_spill<const N: usize>() {
        let mut a = IttyBitty::<N>::new();
        let mut b = IttyBitty::<N>::new();
        a.set(3, true);
        b.set(3, true);
        b.set(500, true);
        assert_ne!(a, b);
        b.set(500, false);
        assert_eq!(a, b);
    }

    #[instantiate_tests(<1>)]
    mod n1 {}
    #[instantiate_tests(<2>)]
    mod n2 {}
    #[instantiate_tests(<3>)]
    mod n3 {}
    #[instantiate_tests(<4>)]
    mod n4 {}
    #[instantiate_tests(<6>)]
    mod n6 {}
}
