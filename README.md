# ittybitty

`IttyBitty<N>` is a dynamically sized bit set that behaves akin to a `SmallVec<[usize; N]>`
It holds `N * size_of::<usize>() - 1` bits inline. If a bit is set beyond that range, it will
allocate a buffer on the heap and stop using the inline bits. `N` must be 2 or greater.

Please consult [**the documentation**](https://docs.rs/ittybitty) for more information.

Add it to your Cargo.toml:

```toml
[dependencies]
ittybitty = "0.1"
```

# Example

```rs
use ittybitty::IttyBitty;

let mut v = IttyBitty::<2>::new();
v.set(4, true);

assert_eq!(v.get(0), false);
assert_eq!(v.get(4), true);
```

## Safety

This code is only mildly not garbage, good luck.

## License

`ittybitty` is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

### Your contributions

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license,
shall be dual licensed as above,
without any additional terms or conditions.