#![allow(clippy::all, dead_code, incomplete_features, unused_imports)]
#![feature(
  adt_const_params,
  box_syntax,
  const_fn_floating_point_arithmetic,
  const_trait_impl,
  exact_size_is_empty,
  generic_const_exprs,
  read_buf,
  trusted_random_access
)]

// In case you're wondering why there's extensive use of "StaticVecs that contain boxed items"
// (something that would probably not be that common in the sense of normal use of this crate) in
// this file: it's done for the sake of wanting to be as "Miri detectable" as possible, by which I
// mean, "weird stuff done with heap memory" is significantly more likely to set Miri off than
// "weird stuff done with stack memory".

use staticvec::{sortedstaticvec, staticvec, CapacityError, StaticVec};

use core::cell;

#[cfg(feature = "std")]
use std::panic::{self, AssertUnwindSafe};

#[cfg(feature = "std")]
use cool_asserts::assert_panics;

#[derive(Debug, Eq, PartialEq, Default)]
struct Counter(cell::Cell<u32>);

impl Counter {
  fn increment(&self) {
    self.0.set(self.0.get() + 1);
  }

  fn get(&self) -> u32 {
    self.0.get()
  }
}

// Helper struct for ensuring things are correctly dropped. Use the `instance`
// method to create a LifespanCountingInstance, then use the init_count
// method to see how many such instances were created (either by clone or by
// `instance`), and the drop_count method to see how many were dropped.
// TODO: create a more advanced version of this pattern that checks WHICH
// elements have been dropped; ie, to ensure that the elements at the end of
// an array are correctly dropped after a truncate
#[derive(Debug, Default)]
struct LifespanCounter {
  // The number of times an instance was created
  init_count: Counter,

  // The number of times an instance was dropped
  drop_count: Counter,
}

impl LifespanCounter {
  fn instance(&self) -> LifespanCountingInstance {
    self.init_count.increment();
    LifespanCountingInstance { counter: self }
  }

  fn init_count(&self) -> u32 {
    self.init_count.get()
  }

  fn drop_count(&self) -> u32 {
    self.drop_count.get()
  }
}

#[derive(Debug)]
struct LifespanCountingInstance<'a> {
  counter: &'a LifespanCounter,
}

impl<'a> Clone for LifespanCountingInstance<'a> {
  fn clone(&self) -> Self {
    self.counter.instance()
  }

  // We deliberately do not provide a clone_from; we'd rather the default
  // behavior (drop and replace with a fresh instance) is used, so we can
  // accurately track clones.
}

impl<'a> Drop for LifespanCountingInstance<'a> {
  fn drop(&mut self) {
    self.counter.drop_count.increment()
  }
}

#[derive(Debug, Eq, PartialEq)]
struct Struct {
  s: &'static str,
}

impl Drop for Struct {
  fn drop(&mut self) {
    // This won't do anything observable in the test context, but it
    // works as a placeholder.
    println!("Dropping Struct with value: {}", self.s)
  }
}

#[derive(Debug, Eq, PartialEq)]
struct ZST {}

impl Drop for ZST {
  fn drop(&mut self) {
    // This won't do anything observable in the test context, but it
    // works as a placeholder.
    println!("Dropping a ZST!")
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct CloneableZST;

#[test]
fn append() {
  let mut a = staticvec![
    box Struct { s: "A" },
    box Struct { s: "B" },
    box Struct { s: "C" }
  ];
  let mut b = staticvec![
    box Struct { s: "D" },
    box Struct { s: "E" },
    box Struct { s: "F" },
    box Struct { s: "G" }
  ];
  let mut c = StaticVec::<Box<Struct>, 6>::new();
  c.append(&mut a);
  c.append(&mut b);
  assert_eq!(format!("{:?}", a), "[]");
  assert_eq!(format!("{:?}", b), "[Struct { s: \"G\" }]");
  assert_eq!(
    c,
    staticvec![
      box Struct { s: "A" },
      box Struct { s: "B" },
      box Struct { s: "C" },
      box Struct { s: "D" },
      box Struct { s: "E" },
      box Struct { s: "F" }
    ]
  );
  let mut d = staticvec![box 12, box 24];
  let mut e = staticvec![box 1, box 2, box 3];
  d.pop().unwrap();
  d.append(&mut e);
  assert_eq!(e, [box 2, box 3]);
  assert_eq!(d, [box 12, box 1]);
  let mut f = StaticVec::<Box<Struct>, 0>::new();
  let mut g = staticvec![box Struct { s: "A" }, box Struct { s: "B" }];
  f.append(&mut g);
  assert_eq!(f, []);
  assert_eq!(g, [box Struct { s: "A" }, box Struct { s: "B" }]);
  let mut h = StaticVec::<Box<Struct>, 1>::new();
  let mut i = staticvec![box Struct { s: "A" }, box Struct { s: "B" }];
  h.append(&mut i);
  assert_eq!(h, [box Struct { s: "A" }]);
  assert_eq!(i, [box Struct { s: "B" }]);
}

#[test]
fn as_mut_ptr() {
  let mut v = staticvec![1, 2, 3];
  unsafe { assert_eq!(*v.as_mut_ptr(), 1) };
}

#[test]
fn as_mut_slice() {
  let mut buffer = staticvec![1, 2, 3, 5, 8];
  assert_eq!(buffer.as_mut_slice(), &mut [1, 2, 3, 5, 8]);
}

#[test]
fn as_ptr() {
  let v = staticvec![1, 2, 3];
  unsafe { assert_eq!(*v.as_ptr(), 1) };
}

#[test]
fn as_slice() {
  let buffer = staticvec![1, 2, 3, 5, 8];
  assert_eq!(buffer.as_slice(), &[1, 2, 3, 5, 8]);
}

#[cfg(feature = "std")]
#[test]
fn bounds_to_string() {
  let mut v = staticvec![1, 2, 3, 4];
  let it = v.iter();
  assert_eq!(
    "Current value of element at `start`: 1\nCurrent value of element at `end`: 4",
    it.bounds_to_string()
  );
  let itm = v.iter_mut();
  assert_eq!(
    "Current value of element at `start`: 1\nCurrent value of element at `end`: 4",
    itm.bounds_to_string()
  );
  let itv = v.into_iter();
  assert_eq!(
    "Current value of element at `start`: 1\nCurrent value of element at `end`: 4",
    itv.bounds_to_string()
  );
  let mut v2 = StaticVec::<Box<i32>, 0>::new();
  let it2 = v2.iter();
  assert_eq!("Empty iterator!", it2.bounds_to_string());
  let itm2 = v2.iter_mut();
  assert_eq!("Empty iterator!", itm2.bounds_to_string());
  let itv2 = v2.into_iter();
  assert_eq!("Empty iterator!", itv2.bounds_to_string());
  let v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut it3 = v3.iter();
  it3.next();
  it3.next_back();
  assert_eq!(
    "Current value of element at `start`: ZST\nCurrent value of element at `end`: ZST",
    it3.bounds_to_string()
  );
  it3.next();
  it3.next_back();
  assert_eq!("Empty iterator!", it3.bounds_to_string());
}

#[test]
fn capacity() {
  let vec = StaticVec::<i32, 10>::new();
  assert_eq!(vec.capacity(), 10);
}

#[test]
fn clear() {
  let mut v = staticvec![1, 2, 3];
  v.clear();
  assert!(v.is_empty());
}

#[test]
fn clone() {
  let v = staticvec![1, 2, 3, 4, 5, 6, 7, 8];
  let vv = v.clone();
  assert_eq!(v, vv);
}

#[test]
fn clone_from_shorter() {
  let src: StaticVec<u32, 20> = (1..10).collect();
  let mut dst: StaticVec<u32, 20> = (0..15).collect();
  dst.clone_from(&src);
  assert_eq!(dst, src);
}

#[test]
fn clone_from_longer() {
  let src: StaticVec<u32, 20> = (0..15).collect();
  let mut dst: StaticVec<u32, 20> = (1..10).collect();
  dst.clone_from(&src);
  assert_eq!(dst, src);
}

#[cfg(feature = "std")]
#[test]
fn panicking_clone() {
  // An earlier implementation of clone incorrectly leaked values in the event
  // of a panicking clone. This test ensures that that does not happen.
  // This struct will, if so configured, panic on a clone. Uses
  // LifespanCountingInstance to track instantiations and deletions, so that
  // we can ensure the correct number of drops are happening
  #[derive(Debug)]
  struct MaybePanicOnClone<'a> {
    tracker: LifespanCountingInstance<'a>,
    should_panic: bool,
  }

  impl<'a> MaybePanicOnClone<'a> {
    fn new(counter: &'a LifespanCounter, should_panic: bool) -> Self {
      Self {
        tracker: counter.instance(),
        should_panic,
      }
    }
  }

  impl<'a> Clone for MaybePanicOnClone<'a> {
    fn clone(&self) -> Self {
      if self.should_panic {
        panic!("Clone correctly panicked during a test")
      } else {
        Self {
          tracker: self.tracker.clone(),
          should_panic: self.should_panic,
        }
      }
    }
  }

  let lifespan_tracker = LifespanCounter::default();
  let mut vec1: StaticVec<MaybePanicOnClone, 20> = StaticVec::new();

  for _ in 0..5 {
    vec1.push(MaybePanicOnClone::new(&lifespan_tracker, false));
  }
  vec1.push(MaybePanicOnClone::new(&lifespan_tracker, true));

  // Sanity check: we've created 6 instances and dropped none of them
  assert_eq!(lifespan_tracker.init_count(), 6);
  assert_eq!(lifespan_tracker.drop_count(), 0);

  // Attempt to clone the staticvec; this will panic. This should result in
  // 5 successful clones, followed by a panic, followed by 5 drops during
  // unwinding.
  let result = panic::catch_unwind(AssertUnwindSafe(|| {
    let vec2 = vec1.clone();
    vec2
  }));

  // Ensure that a panic did occur
  assert!(result.is_err());

  // At this point, 5 instances should have been created and dropped in the
  // aborted clone
  assert_eq!(lifespan_tracker.init_count(), 11);
  assert_eq!(lifespan_tracker.drop_count(), 5);

  drop(vec1);

  assert_eq!(lifespan_tracker.init_count(), 11);
  assert_eq!(lifespan_tracker.drop_count(), 11);
}

#[test]
fn concat() {
  assert!(
    staticvec!["A, B"].concat(&staticvec!["C", "D", "E", "F"]) == ["A, B", "C", "D", "E", "F"]
  );
  let v = StaticVec::<i32, 0>::from([]).concat(&StaticVec::<i32, 0>::from([]));
  assert_eq!(v, []);
  let v2 = staticvec![4, 5, 6].concat(&staticvec![1, 2, 3]);
  assert_eq!(v2, [4, 5, 6, 1, 2, 3]);
}

#[test]
fn concat_clone() {
  assert!(
    staticvec![box "A, B"].concat_clone(&staticvec![box "C", box "D", box "E", box "F"])
      == [box "A, B", box "C", box "D", box "E", box "F"]
  );
  let v = StaticVec::<Box<i32>, 0>::from([]).concat_clone(&StaticVec::<Box<i32>, 0>::from([]));
  assert_eq!(v, []);
  let v2 = staticvec![box 4, box 5, box 6].concat_clone(&staticvec![box 1, box 2, box 3]);
  assert_eq!(v2, [box 4, box 5, box 6, box 1, box 2, box 3]);
}

#[test]
fn contains() {
  assert_eq!(staticvec![1, 2, 3].contains(&2), true);
  assert_eq!(staticvec![1, 2, 3].contains(&4), false);
  assert_eq!(staticvec![].contains(&2), false);
}

#[test]
fn dedup() {
  let mut vec = staticvec![1, 2, 2, 3, 2];
  vec.dedup();
  assert_eq!(vec, [1, 2, 3, 2]);
}

#[test]
fn dedup_by() {
  let mut vec = staticvec!["foo", "bar", "Bar", "baz", "bar"];
  vec.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
  assert_eq!(vec, ["foo", "bar", "baz", "bar"]);
}

#[test]
fn dedup_by_key() {
  let mut vec = staticvec![10, 20, 21, 30, 20];
  vec.dedup_by_key(|i| *i / 10);
  assert_eq!(vec, [10, 20, 30, 20]);
}

#[test]
fn difference() {
  assert_eq!(
    staticvec![4, 5, 6, 7].difference(&staticvec![1, 2, 3, 7]),
    [4, 5, 6]
  );
  assert_eq!(staticvec![1, 2, 3].difference(&staticvec![3, 4, 5]), [1, 2]);
}

#[test]
fn drain() {
  let mut v = staticvec![1, 2, 3];
  let u = v.drain(1..);
  assert_eq!(v, &[1]);
  assert_eq!(u, &[2, 3]);
  v.drain(..);
  assert_eq!(v, &[]);
  let mut v = StaticVec::from([0; 8]);
  v.pop();
  v.drain(0..7);
  assert_eq!(&v[..], &[]);
  v.extend(0..);
  v.drain(1..4);
  assert_eq!(&v[..], &[0, 4, 5, 6, 7]);
  let u: StaticVec<u8, 3> = v.drain(1..4).iter().rev().collect();
  assert_eq!(&u[..], &[6, 5, 4]);
  assert_eq!(&v[..], &[0, 7]);
  v.drain(..);
  assert_eq!(&v[..], &[]);
  let mut v2 = StaticVec::from([0; 8]);
  v2.drain(0..=7);
  assert_eq!(&v2[..], &[]);
  v2.extend(0..);
  v2.drain(1..=4);
  assert_eq!(&v2[..], &[0, 5, 6, 7]);
  let u: StaticVec<u8, 3> = v2.drain(1..=2).iter().rev().collect();
  assert_eq!(&u[..], &[6, 5]);
  assert_eq!(&v2[..], &[0, 7]);
  v2.drain(..);
  assert_eq!(&v2[..], &[]);
  let mut v3 = staticvec![box 12, box 12, box 12, box 12, box 12, box 12, box 12, box 12];
  v3.pop();
  v3.drain(0..7);
  assert_eq!(&v3[..], &[]);
  let mut v4 = staticvec![box 12, box 12, box 12, box 12, box 12, box 12, box 12, box 12];
  v4.drain(0..4);
  assert_eq!(&v4[..], &[box 12, box 12, box 12, box 12]);
}

#[test]
#[should_panic]
fn drain_panic() {
  let mut v3 = StaticVec::from([0; 0]);
  v3.drain(0..=0);
}

#[test]
fn drain_iter() {
  let mut v = staticvec![1, 2, 3];
  let u: StaticVec<i32, 6> = v.drain_iter(1..).collect();
  assert_eq!(v, &[1]);
  assert_eq!(u, &[2, 3]);
  v.drain_iter(..);
  assert_eq!(v, &[]);
  let mut v = StaticVec::from([0; 8]);
  v.pop();
  v.drain_iter(0..7);
  assert_eq!(&v[..], &[]);
  v.extend(0..);
  v.drain_iter(1..4);
  assert_eq!(&v[..], &[0, 4, 5, 6, 7]);
  let u: StaticVec<_, 3> = v.drain_iter(1..4).rev().collect();
  assert_eq!(&u[..], &[6, 5, 4]);
  assert_eq!(&v[..], &[0, 7]);
  v.drain_iter(..);
  assert_eq!(&v[..], &[]);
  let mut v2 = StaticVec::from([0; 8]);
  v2.drain_iter(0..=7);
  assert_eq!(&v2[..], &[]);
  v2.extend(0..);
  v2.drain_iter(1..=4);
  assert_eq!(&v2[..], &[0, 5, 6, 7]);
  let u: StaticVec<_, 3> = v2.drain_iter(1..=2).rev().collect();
  assert_eq!(&u[..], &[6, 5]);
  assert_eq!(&v2[..], &[0, 7]);
  v2.drain_iter(..);
  assert_eq!(&v2[..], &[]);
  let mut v3 = staticvec![box 12, box 12, box 12, box 12, box 12, box 12, box 12, box 12];
  v3.pop();
  v3.drain_iter(0..7);
  assert_eq!(&v3[..], &[]);
  let mut v4 = staticvec![box 12, box 12, box 12, box 12, box 12, box 12, box 12, box 12];
  v4.drain_iter(0..4);
  assert_eq!(&v4[..], &[box 12, box 12, box 12, box 12]);
  let mut v5 = staticvec![
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
  ];
  assert_eq!(v5.drain_iter(6..12).len(), 6);
  assert_eq!(v5.len(), 10);
  let mut v6 = staticvec![
    box 1, box 2, box 3, box 4, box 5, box 6, box 7, box 8, box 9, box 10, box 11, box 12, box 13,
    box 14, box 15, box 16,
  ];
  assert_eq!(v6.drain_iter(6..12).len(), 6);
  assert_eq!(v6.len(), 10);
  assert_eq!(
    staticvec![1, 2, 3, 4]
      .drain_iter(1..3)
      .zip(staticvec![5, 6, 7, 8].drain_iter(1..3))
      .collect::<StaticVec<(i32, i32), 2>>(),
    staticvec![(2, 6), (3, 7)]
  );
}

#[test]
#[should_panic]
fn drain_iter_panic() {
  let mut v3 = StaticVec::from([0; 0]);
  v3.drain_iter(0..=0);
}

#[test]
fn drain_filter() {
  let mut numbers = staticvec![1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15];
  let evens = numbers.drain_filter(|x| *x % 2 == 0);
  let odds = numbers;
  assert_eq!(evens, [2, 4, 6, 8, 14]);
  assert_eq!(odds, [1, 3, 5, 9, 11, 13, 15]);
  let mut zsts1 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let full = zsts1.drain_filter(|x| *x == ZST {});
  let empty = zsts1;
  assert_eq!(full, [ZST {}, ZST {}, ZST {}, ZST {}]);
  assert_eq!(empty, []);
  let mut empty: StaticVec<i32, 12> = StaticVec::from([]);
  assert_eq!(empty.drain_filter(|x| *x == 0), []);
  let mut structs: StaticVec<Box<Struct>, 4> = staticvec![
    box Struct { s: "A" },
    box Struct { s: "B" },
    box Struct { s: "C" },
    box Struct { s: "D" }
  ];
  assert_eq!(
    structs.drain_filter(|s| s.s > "B"),
    [box Struct { s: "C" }, box Struct { s: "D" }]
  );
}

#[test]
fn empty_slice() {
  let mut vec = staticvec![1, 2, 3, 4, 5];
  let s = &vec[0..0];
  assert_eq!(0, s.len());
  let s = &mut vec[0..0];
  assert_eq!(0, s.len());
}

#[test]
fn extend() {
  let mut c = StaticVec::<i32, 6>::new();
  c.push(5);
  c.push(6);
  c.push(7);
  c.extend(staticvec![1, 2, 3].iter());
  assert_eq!("[5, 6, 7, 1, 2, 3]", format!("{:?}", c));
  c.clear();
  assert_eq!(c.len(), 0);
  c.extend([1].iter());
  assert_eq!(c.len(), 1);
  c.extend(staticvec![1, 2, 3, 4, 5, 6, 7].into_iter());
  assert_eq!(c.len(), 6);
  c.clear();
  c.extend(staticvec![1, 2, 3, 4, 5, 6, 7]);
  assert_eq!(c.len(), 6);
  let c2 = staticvec![vec![1, 1], vec![1, 2], vec![1, 3], vec![1, 4]];
  let mut c3 = StaticVec::<Vec<u8>, 2>::new();
  c3.extend(c2);
  assert_eq!(c3, [vec![1, 1], vec![1, 2]]);
  let c4 = staticvec![vec![1, 1], vec![1, 2], vec![1, 3], vec![1, 4]];
  let mut c5 = StaticVec::<Vec<u8>, 4>::new();
  c5.extend(c4);
  assert_eq!(c5, [vec![1, 1], vec![1, 2], vec![1, 3], vec![1, 4]]);
  let c6 = staticvec![
    vec![vec![1, 1]],
    vec![vec![1, 2]],
    vec![vec![1, 3]],
    vec![vec![1, 4]],
    vec![vec![1, 5]],
    vec![vec![1, 6]]
  ];
  let mut c7 = StaticVec::<Vec<Vec<u8>>, 3>::new();
  c7.extend(c6);
  assert_eq!(c7, [vec![vec![1, 1]], vec![vec![1, 2]], vec![vec![1, 3]]]);
}

#[test]
fn extend_from_slice() {
  let mut vec = StaticVec::<i32, 4>::new_from_slice(&[1]);
  vec.extend_from_slice(&[2, 3, 4]);
  assert_eq!(vec, [1, 2, 3, 4]);
  let mut vec2 = StaticVec::<i32, 0>::new();
  vec2.extend_from_slice(&[2, 3, 4]);
  assert_eq!(vec2, []);
}

#[test]
fn filled_with() {
  let mut i = 0;
  let v = StaticVec::<i32, 64>::filled_with(|| {
    i += 1;
    i
  });
  assert_eq!(v.len(), 64);
  assert_eq!(v[0], 1);
  assert_eq!(v[1], 2);
  assert_eq!(v[2], 3);
  assert_eq!(v[3], 4);
  let v2 = StaticVec::<i32, 0>::filled_with(|| 0);
  assert_eq!(v2.len(), 0);
  assert_eq!(v2.capacity(), 0);
  assert_eq!(v2.remaining_capacity(), 0);
}

#[test]
fn filled_with_by_index() {
  let v = StaticVec::<usize, 64>::filled_with_by_index(|i| i + 1);
  assert_eq!(v.len(), 64);
  assert_eq!(v[0], 1);
  assert_eq!(v[1], 2);
  assert_eq!(v[2], 3);
  assert_eq!(v[3], 4);
  let v2 = StaticVec::<usize, 0>::filled_with_by_index(|i| i + 1);
  assert_eq!(v2.len(), 0);
  assert_eq!(v2.capacity(), 0);
  assert_eq!(v2.remaining_capacity(), 0);
}

#[test]
fn first() {
  let v = staticvec![1, 2, 3];
  assert_eq!(*v.first().unwrap(), 1);
}

#[test]
fn first_mut() {
  let mut v = staticvec![1, 2, 3];
  assert_eq!(*v.first_mut().unwrap(), 1);
}

#[test]
fn from() {
  assert_eq!(
    "[5, 6, 7, 1, 2, 3]",
    format!("{:?}", StaticVec::<i32, 6>::from(&[5, 6, 7, 1, 2, 3]))
  );
  assert_eq!(
    "[5, 6, 7]",
    format!("{:?}", StaticVec::<i32, 3>::from(&mut [5, 6, 7, 1, 2, 3]))
  );
  assert_eq!(
    "[1, 1, 1, 1, 1, 1]",
    format!("{:?}", StaticVec::<i32, 6>::from([1; 6]))
  );
  assert_eq!(
    "[1, 1, 1]",
    format!("{:?}", StaticVec::<i32, 3>::from([1; 6]))
  );
  assert_eq!(
    "[]",
    format!("{:?}", StaticVec::<Box<i32>, 0>::from([box 9i32]))
  );
  assert_eq!(
    "[9, 18, 27]",
    format!(
      "{:?}",
      StaticVec::<Box<i32>, 19>::from([box 9i32, box 18i32, box 27i32])
    )
  );
  const C: StaticVec<i32, 4> = StaticVec::from([1, 2, 3, 4]);
  assert_eq!("[1, 2, 3, 4]", format!("{:?}", C));
  let mut v = staticvec![1];
  v.clear();
  assert_eq!(StaticVec::<i32, 6>::from(v.as_slice()).len(), 0);
  assert_eq!(StaticVec::from(["A"]), ["A"]);
  assert_eq!(
    StaticVec::from([box Struct { s: "A" }, box Struct { s: "B" }]),
    [box Struct { s: "A" }, box Struct { s: "B" }]
  );
}

#[test]
fn from_iter() {
  assert_eq!(
    StaticVec::<u8, 12>::from_iter(&[1, 2, 3, 4, 5, 6]),
    [1, 2, 3, 4, 5, 6]
  );
  assert_eq!(
    StaticVec::<u8, 12>::from_iter([1, 2, 3, 4, 5, 6].iter()),
    [1, 2, 3, 4, 5, 6]
  );
  assert_eq!(
    StaticVec::<u8, 12>::from_iter(staticvec![1, 2, 3, 4, 5, 6].iter()),
    [1, 2, 3, 4, 5, 6]
  );
  assert_eq!(
    StaticVec::<u8, 12>::from_iter(IntoIterator::into_iter([1, 2, 3, 4, 5, 6])),
    [1, 2, 3, 4, 5, 6]
  );
  assert_eq!(StaticVec::<u8, 0>::from_iter(&[1, 2, 3, 4, 5, 6]), []);
  assert_eq!(
    StaticVec::<Box<Struct>, 2>::from_iter(
      staticvec![box Struct { s: "A" }, box Struct { s: "B" }].into_iter()
    ),
    [box Struct { s: "A" }, box Struct { s: "B" }]
  );
  assert_eq!(
    StaticVec::<Box<Struct>, 2>::from_iter(staticvec![
      box Struct { s: "A" },
      box Struct { s: "B" },
      box Struct { s: "C" },
      box Struct { s: "C" }
    ]),
    [box Struct { s: "A" }, box Struct { s: "B" }]
  );
  assert_eq!(
    StaticVec::<Box<Struct>, 4>::from_iter(staticvec![
      box Struct { s: "A" },
      box Struct { s: "B" },
      box Struct { s: "C" },
      box Struct { s: "C" }
    ]),
    [
      box Struct { s: "A" },
      box Struct { s: "B" },
      box Struct { s: "C" },
      box Struct { s: "C" }
    ]
  );
}

#[cfg(feature = "std")]
#[test]
fn from_vec() {
  let v = vec![
    box Struct { s: "AAA" },
    box Struct { s: "BBB" },
    box Struct { s: "CCC" },
  ];
  let vv = StaticVec::<Box<Struct>, 2>::from_vec(v);
  assert_eq!(vv.capacity(), 2);
  assert_eq!(vv.len(), 2);
  assert_eq!(vv, [box Struct { s: "AAA" }, box Struct { s: "BBB" }]);
  let x = Vec::<Box<Struct>>::new();
  let y = StaticVec::<Box<Struct>, 1>::from_vec(x);
  assert_eq!(y, []);
}

#[test]
fn get_unchecked() {
  let v = staticvec!["a", "b", "c"];
  assert_eq!(unsafe { *v.get_unchecked(1) }, "b");
}

#[test]
fn get_unchecked_mut() {
  let mut v = staticvec!["a", "b", "c"];
  assert_eq!(unsafe { *v.get_unchecked_mut(1) }, "b");
}

#[test]
fn index() {
  let vec = staticvec![0, 1, 2, 3, 4];
  assert_eq!(vec[3], 3);
  assert_eq!(vec[1..4], [1, 2, 3]);
  assert_eq!(vec[1..=1], [1]);
  assert_eq!(vec[1..3], [1, 2]);
  assert_eq!(vec[..3], [0, 1, 2]);
  assert_eq!(vec[..=3], [0, 1, 2, 3]);
  assert_eq!(vec[1..], [1, 2, 3, 4]);
  assert_eq!(vec[1..=3], [1, 2, 3]);
  assert_eq!(vec[..], [0, 1, 2, 3, 4]);
}

#[test]
#[cfg(feature = "std")]
fn index_panics() {
  let vec = staticvec![0, 1, 2, 3, 4];
  assert_panics!(vec[10]);
  assert_panics!(&vec[..10]);
  assert_panics!(&vec[10..]);
  assert_panics!(&vec[10..15]);
  assert_panics!(&vec[1..0]);
}

#[test]
fn insert() {
  let mut vec = StaticVec::<i32, 5>::new_from_slice(&[1, 2, 3]);
  vec.insert(1, 4);
  assert_eq!(vec, [1, 4, 2, 3]);
  vec.insert(4, 5);
  assert_eq!(vec, [1, 4, 2, 3, 5]);
}

#[test]
#[should_panic]
fn insert_already_full() {
  let mut vec = StaticVec::<i32, 5>::from([1, 2, 3, 4, 5]);
  vec.insert(1, 4);
}

#[test]
#[should_panic]
fn insert_index_too_high() {
  let mut vec = StaticVec::<i32, 8>::from([1, 2, 3, 4, 5]);
  vec.insert(19, 4);
}

#[test]
#[should_panic]
fn insert_no_capacity() {
  let mut vec = StaticVec::<i32, 0>::new();
  vec.insert(0, 4);
}

// The next couple of tests for `insert_from_slice` and `insert_many` and are adapted from the
// SmallVec testsuite.

#[test]
fn insert_from_slice() {
  let mut v: StaticVec<u8, 8> = StaticVec::new();
  for x in 0..4 {
    v.push(x);
  }
  assert_eq!(v.len(), 4);
  v.insert_from_slice(1, &[5, 6]);
  assert_eq!(
    &v.iter().map(|v| *v).collect::<StaticVec<_, 8>>(),
    &[0, 5, 6, 1, 2, 3]
  );
  v.clear();
  for x in 0..4 {
    v.push(x);
  }
  assert_eq!(v.len(), 4);
  v.insert_from_slice(1, &[5, 6]);
  assert_eq!(
    &v.iter().map(|v| *v).collect::<StaticVec<_, 8>>(),
    &[0, 5, 6, 1, 2, 3]
  );
  v.clear();
  for i in 0..6 {
    v.push(i + 1);
  }
  v.insert_from_slice(6, &[1]);
  assert_eq!(
    &v.iter().map(|v| *v).collect::<StaticVec<_, 8>>(),
    &[1, 2, 3, 4, 5, 6, 1]
  );
  let mut v2: StaticVec<u8, 1> = StaticVec::new();
  v2.insert_from_slice(0, &[12]);
  assert_eq!(v2, [12]);
}

#[test]
#[should_panic(
  expected = "Insufficient remaining capacity or bounds check failure in `StaticVec::insert_from_slice`!"
)]
fn insert_from_slice_panic_a() {
  let mut v: StaticVec<u8, 4> = StaticVec::from([1, 2, 3, 4]);
  v.insert_from_slice(0, &[4]);
}

#[test]
#[should_panic(
  expected = "Insufficient remaining capacity or bounds check failure in `StaticVec::insert_from_slice`!"
)]
fn insert_from_slice_panic_b() {
  let mut v: StaticVec<u8, 8> = StaticVec::new();
  v.insert_from_slice(19, &[4]);
}

#[test]
#[should_panic(
  expected = "Insufficient remaining capacity or bounds check failure in `StaticVec::insert_from_slice`!"
)]
fn insert_from_slice_panic_c() {
  let mut v: StaticVec<u8, 8> = StaticVec::<u8, 8>::from(&[1, 2, 3, 4, 5, 6]);
  v.insert_from_slice(0, &[7, 8, 9, 10, 11]);
}

#[test]
fn insert_many() {
  let mut v: StaticVec<u8, 8> = StaticVec::new();
  for x in 0..4 {
    v.push(x);
  }
  assert_eq!(v.len(), 4);
  v.insert_many(1, [5, 6].iter().cloned());
  assert_eq!(
    &v.iter().map(|v| *v).collect::<StaticVec<_, 8>>(),
    &[0, 5, 6, 1, 2, 3]
  );
  v.clear();
  for x in 0..4 {
    v.push(x);
  }
  assert_eq!(v.len(), 4);
  v.insert_many(1, [5, 6].iter().cloned());
  assert_eq!(
    &v.iter().map(|v| *v).collect::<StaticVec<_, 8>>(),
    &[0, 5, 6, 1, 2, 3]
  );
  v.clear();
  for i in 0..6 {
    v.push(i + 1);
  }
  v.insert_many(6, [1].iter().cloned());
  assert_eq!(
    &v.iter().map(|v| *v).collect::<StaticVec<_, 8>>(),
    &[1, 2, 3, 4, 5, 6, 1]
  );
}

#[test]
#[should_panic(
  expected = "Insufficient remaining capacity or bounds check failure in `StaticVec::insert_many`!"
)]
fn insert_many_panic1() {
  let mut v: StaticVec<Box<u8>, 8> = StaticVec::new();
  for i in 0..7 {
    v.push(box (i + 1));
  }
  v.insert_many(0, [box 1, box 2, box 3, box 4].iter().cloned());
}

#[test]
#[should_panic(
  expected = "Insufficient remaining capacity or bounds check failure in `StaticVec::insert_many`!"
)]
fn insert_many_panic2() {
  let mut v2: StaticVec<Box<u8>, 0> = StaticVec::new();
  v2.insert_many(27, [box 1, box 2, box 3, box 4].iter().cloned());
}

#[test]
fn intersection() {
  assert_eq!(
    staticvec![4, 5, 6, 7].intersection(&staticvec![1, 2, 3, 7, 4]),
    [4, 7],
  );
  assert_eq!(staticvec![1].intersection(&staticvec![2]), [],);
  assert_eq!(staticvec![1, 2, 3].intersection(&staticvec![]), []);
}

#[test]
fn intersperse() {
  assert_eq!(
    staticvec!["A", "B", "C", "D"].intersperse("Z"),
    ["A", "Z", "B", "Z", "C", "Z", "D"]
  );
  assert_eq!(staticvec![""].intersperse("B"), [""]);
  assert_eq!(staticvec!["A"].intersperse("B"), ["A"]);
  let mut x = staticvec!["A"];
  x.clear();
  assert_eq!(x.intersperse("B"), StaticVec::<&str, 0>::new());
}

#[test]
fn intersperse_clone() {
  assert_eq!(
    staticvec![box "A", box "B", box "C", box "D"].intersperse_clone(box "Z"),
    [box "A", box "Z", box "B", box "Z", box "C", box "Z", box "D"]
  );
  assert_eq!(staticvec![box ""].intersperse_clone(box "B"), [box ""]);
  assert_eq!(staticvec![box "A"].intersperse_clone(box "B"), [box "A"]);
  let mut x = staticvec![box "A"];
  x.clear();
  assert_eq!(
    x.intersperse_clone(box "B"),
    StaticVec::<Box<&str>, 0>::new()
  );
}

#[test]
fn is_empty() {
  let mut v = StaticVec::<i32, 1>::new();
  assert!(v.is_empty());
  v.push(1);
  assert!(!v.is_empty());
}

#[test]
fn is_not_empty() {
  let mut v = StaticVec::<i32, 1>::new();
  assert!(v.is_empty());
  v.push(1);
  assert!(v.is_not_empty());
}

#[test]
fn is_full() {
  let mut v = StaticVec::<i32, 1>::new();
  v.push(1);
  assert!(v.is_full());
}

#[test]
fn is_not_full() {
  let v = StaticVec::<i32, 1>::new();
  assert!(v.is_not_full());
}

#[test]
fn iter() {
  let v = staticvec![box 1, box 2, box 3, box 4, box 5];
  let mut i = v.iter();
  assert_eq!(*i.next().unwrap(), box 1);
  assert_eq!(*i.next_back().unwrap(), box 5);
  assert_eq!("StaticVecIterConst([2, 3, 4])", format!("{:?}", i));
  assert_eq!(*i.next().unwrap(), box 2);
  assert_eq!(*i.next_back().unwrap(), box 4);
  assert_eq!("StaticVecIterConst([3])", format!("{:?}", i));
  assert_eq!(*i.next().unwrap(), box 3);
  assert_eq!("StaticVecIterConst([])", format!("{:?}", i));
  let v2 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut it2 = v2.iter();
  assert_eq!(it2.as_slice(), &[ZST {}, ZST {}, ZST {}, ZST {}]);
  assert_eq!(it2.len(), 4);
  assert_eq!(it2.is_empty(), false);
  it2.next();
  assert_eq!(it2.len(), 3);
  assert_eq!(it2.is_empty(), false);
  it2.next_back();
  assert_eq!(it2.len(), 2);
  assert_eq!(it2.is_empty(), false);
  it2.next();
  assert_eq!(it2.len(), 1);
  assert_eq!(it2.is_empty(), false);
  it2.next_back();
  assert_eq!(it2.len(), 0);
  assert_eq!(it2.is_empty(), true);
  let a1 = staticvec![box 1, box 2, box 3];
  let a2 = staticvec![box 4, box 5, box 6];
  let mut iter = a1.iter().zip(a2.iter());
  assert_eq!(iter.next(), Some((&box 1, &box 4)));
  assert_eq!(iter.next(), Some((&box 2, &box 5)));
  assert_eq!(iter.next(), Some((&box 3, &box 6)));
  assert_eq!(iter.next(), None);
  let a3 = staticvec![ZST {}, ZST {}, ZST {}];
  let a4 = staticvec![ZST {}, ZST {}, ZST {}];
  let mut iter2 = a3.iter().zip(a4.iter());
  assert_eq!(iter2.next(), Some((&ZST {}, &ZST {})));
  assert_eq!(iter2.next(), Some((&ZST {}, &ZST {})));
  assert_eq!(iter2.next(), Some((&ZST {}, &ZST {})));
  assert_eq!(iter2.next(), None);
  let a5 = staticvec![
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {}
  ];
  let mut iter3 = a5.iter();
  unsafe {
    assert_eq!(iter3.__iterator_get_unchecked(13), &ZST {});
  }
}

#[test]
fn iter_nth() {
  let v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut i3 = v3.iter();
  assert_eq!(i3.nth(2).unwrap(), &ZST {});
  assert_eq!(i3.as_slice(), &[ZST {}]);
  assert_eq!(i3.nth(0).unwrap(), &ZST {});
  assert_eq!(i3.nth(0), None);
  assert_eq!(i3.nth(0), None);
  let v4 = staticvec![1, 2, 3, 4];
  let mut i4 = v4.iter();
  assert_eq!(i4.nth(2).unwrap(), &3);
  assert_eq!(i4.as_slice(), &[4]);
  assert_eq!(i4.nth(0).unwrap(), &4);
  assert_eq!(i4.nth(0), None);
  assert_eq!(i4.nth(0), None);
  let xs = staticvec![0, 1, 2, 3, 4, 5];
  for (i, &x) in xs.iter().enumerate() {
    assert_eq!(i, x);
  }
  let mut it = xs.iter().enumerate();
  while let Some((i, &x)) = it.nth(0) {
    assert_eq!(i, x);
  }
  let mut it = xs.iter().enumerate();
  while let Some((i, &x)) = it.nth(1) {
    assert_eq!(i, x);
  }
  let (i, &x) = xs.iter().enumerate().nth(3).unwrap();
  assert_eq!(i, x);
  assert_eq!(i, 3);
  let xs5 = staticvec![vec![1], vec![2], vec![3], vec![4], vec![5]];
  let mut it5 = xs5.iter();
  assert_eq!(it5.nth(2).unwrap(), &vec![3]);
  assert_eq!(it5.as_slice(), &[vec![4], vec![5]]);
  assert_eq!(it5.next().unwrap(), &vec![4]);
  assert_eq!(it5.next_back().unwrap(), &vec![5]);
  assert_eq!(it5.nth(0), None);
  let xs6 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it6 = xs6.iter();
  let o = it6.nth(2);
  assert_eq!(format!("{:?}", o), "Some([3, 3])");
  assert_eq!(
    format!("{:?}", it6),
    "StaticVecIterConst([[4, 4], [5, 5], [6, 6]])"
  );
  let xs7 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it7 = xs7.iter();
  let o = it7.nth(5);
  assert_eq!(format!("{:?}", o), "Some([6, 6])");
  assert_eq!(format!("{:?}", it7), "StaticVecIterConst([])");
  let vlast = staticvec![
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {}
  ];
  let mut itlast = vlast.iter();
  assert_eq!(itlast.nth(13).unwrap(), &ZST {});
}

#[test]
fn iter_nth_back() {
  let v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut i3 = v3.iter();
  assert_eq!(i3.nth_back(2).unwrap(), &ZST {});
  assert_eq!(i3.as_slice(), &[ZST {}]);
  assert_eq!(i3.nth_back(0).unwrap(), &ZST {});
  assert_eq!(i3.nth_back(0), None);
  assert_eq!(i3.nth_back(0), None);
  let v4 = staticvec![1, 2, 3, 4];
  let mut i4 = v4.iter();
  assert_eq!(i4.nth_back(2).unwrap(), &2);
  assert_eq!(i4.as_slice(), &[1]);
  assert_eq!(i4.nth_back(0).unwrap(), &1);
  assert_eq!(i4.nth_back(0), None);
  assert_eq!(i4.nth_back(0), None);
  let xs = staticvec![0, 1, 2, 3, 4, 5];
  let mut it = xs.iter().enumerate();
  while let Some((i, &x)) = it.nth_back(0) {
    assert_eq!(i, x);
  }
  let mut it = xs.iter().enumerate();
  while let Some((i, &x)) = it.nth_back(1) {
    assert_eq!(i, x);
  }
  let (i, &x) = xs.iter().enumerate().nth_back(3).unwrap();
  assert_eq!(i, x);
  assert_eq!(i, 2);
  let xs5 = staticvec![vec![1], vec![2], vec![3], vec![4], vec![5]];
  let mut it5 = xs5.iter();
  assert_eq!(it5.nth_back(1).unwrap(), &vec![4]);
  assert_eq!(it5.as_slice(), &[vec![1], vec![2], vec![3]]);
  assert_eq!(it5.next().unwrap(), &vec![1]);
  assert_eq!(it5.next_back().unwrap(), &vec![3]);
  assert_eq!(it5.nth_back(0).unwrap(), &vec![2]);
  let xs6 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it6 = xs6.iter();
  let o = it6.nth_back(2);
  assert_eq!(format!("{:?}", o), "Some([4, 4])");
  assert_eq!(
    format!("{:?}", it6),
    "StaticVecIterConst([[1, 1], [2, 2], [3, 3]])"
  );
  let xs7 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it7 = xs7.iter();
  let o = it7.nth_back(5);
  assert_eq!(format!("{:?}", o), "Some([1, 1])");
  assert_eq!(format!("{:?}", it7), "StaticVecIterConst([])");
}

#[test]
fn iter_nth2() {
  let v = staticvec![0, 1, 2, 3, 4];
  for i in 0..v.len() {
    assert_eq!(v.iter().nth(i).unwrap(), &v[i]);
  }
  assert_eq!(v.iter().nth(v.len()), None);
}

#[test]
fn iter_nth_back2() {
  let v = staticvec![0, 1, 2, 3, 4];
  for i in 0..v.len() {
    assert_eq!(v.iter().nth_back(i).unwrap(), &v[v.len() - 1 - i]);
  }
  assert_eq!(v.iter().nth_back(v.len()), None);
}

#[test]
fn iter_rev_nth() {
  let v = staticvec![0, 1, 2, 3, 4];
  for i in 0..v.len() {
    assert_eq!(v.iter().rev().nth(i).unwrap(), &v[v.len() - 1 - i]);
  }
  assert_eq!(v.iter().rev().nth(v.len()), None);
}

#[test]
fn iter_rev_nth_back() {
  let v = staticvec![0, 1, 2, 3, 4];
  for i in 0..v.len() {
    assert_eq!(v.iter().rev().nth_back(i).unwrap(), &v[i]);
  }
  assert_eq!(v.iter().rev().nth_back(v.len()), None);
}

#[test]
fn iter_mut() {
  let mut v = staticvec![1, 2, 3, 4, 5];
  let mut i = v.iter_mut();
  assert_eq!(*i.next().unwrap(), 1);
  assert_eq!(*i.next_back().unwrap(), 5);
  assert_eq!("StaticVecIterMut([2, 3, 4])", format!("{:?}", i));
  assert_eq!(*i.next().unwrap(), 2);
  assert_eq!(*i.next_back().unwrap(), 4);
  assert_eq!("StaticVecIterMut([3])", format!("{:?}", i));
  assert_eq!(*i.next().unwrap(), 3);
  assert_eq!("StaticVecIterMut([])", format!("{:?}", i));
  let mut v2 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut it2 = v2.iter_mut();
  assert_eq!(it2.as_slice(), &[ZST {}, ZST {}, ZST {}, ZST {}]);
  assert_eq!(it2.len(), 4);
  assert_eq!(it2.is_empty(), false);
  it2.next();
  assert_eq!(it2.len(), 3);
  assert_eq!(it2.is_empty(), false);
  it2.next_back();
  assert_eq!(it2.len(), 2);
  assert_eq!(it2.is_empty(), false);
  it2.next();
  assert_eq!(it2.len(), 1);
  assert_eq!(it2.is_empty(), false);
  it2.next_back();
  assert_eq!(it2.len(), 0);
  assert_eq!(it2.is_empty(), true);
  let mut a1 = staticvec![box 1, box 2, box 3];
  let mut a2 = staticvec![box 4, box 5, box 6];
  let iter = a1.iter_mut().zip(a2.iter_mut());
  for tup in iter {
    **tup.0 += 1;
    **tup.1 += 1;
  }
  assert_eq!(
    a1.concat_clone(&a2),
    staticvec![box 2, box 3, box 4, box 5, box 6, box 7]
  );
  let mut a3 = staticvec![
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {}
  ];
  let mut iter2 = a3.iter_mut();
  unsafe {
    assert_eq!(iter2.__iterator_get_unchecked(13), &ZST {});
  }
}

#[test]
fn iter_mut_nth() {
  let mut v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut i3 = v3.iter_mut();
  assert_eq!(i3.nth(2).unwrap(), &mut ZST {});
  assert_eq!(i3.as_slice(), &mut [ZST {}]);
  assert_eq!(i3.nth(0).unwrap(), &mut ZST {});
  assert_eq!(i3.nth(0), None);
  let mut v4 = staticvec![1, 2, 3, 4];
  let mut i4 = v4.iter_mut();
  assert_eq!(i4.nth(2).unwrap(), &mut 3);
  assert_eq!(i4.as_slice(), &mut [4]);
  assert_eq!(i4.nth(0).unwrap(), &mut 4);
  assert_eq!(i4.nth(0), None);
  let mut xs = staticvec![0, 1, 2, 3, 4, 5];
  for (i, &mut x) in xs.iter_mut().enumerate() {
    assert_eq!(i, x);
  }
  let mut it = xs.iter_mut().enumerate();
  while let Some((i, &mut x)) = it.nth(0) {
    assert_eq!(i, x);
  }
  let mut it = xs.iter_mut().enumerate();
  while let Some((i, &mut x)) = it.nth(1) {
    assert_eq!(i, x);
  }
  let (i, &mut x) = xs.iter_mut().enumerate().nth(3).unwrap();
  assert_eq!(i, x);
  assert_eq!(i, 3);
  let mut xs5 = staticvec![vec![1], vec![2], vec![3], vec![4], vec![5]];
  let mut it5 = xs5.iter_mut();
  assert_eq!(it5.nth(2).unwrap(), &mut vec![3]);
  assert_eq!(it5.as_slice(), &[vec![4], vec![5]]);
  assert_eq!(it5.next().unwrap(), &mut vec![4]);
  assert_eq!(it5.next_back().unwrap(), &mut vec![5]);
  assert_eq!(it5.nth(0), None);
  let mut xs6 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it6 = xs6.iter_mut();
  let o = it6.nth(2);
  assert_eq!(format!("{:?}", o), "Some([3, 3])");
  assert_eq!(
    format!("{:?}", it6),
    "StaticVecIterMut([[4, 4], [5, 5], [6, 6]])"
  );
  let mut xs7 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it7 = xs7.iter_mut();
  let o = it7.nth(5);
  assert_eq!(format!("{:?}", o), "Some([6, 6])");
  assert_eq!(format!("{:?}", it7), "StaticVecIterMut([])");
  let mut vlast = staticvec![
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {}
  ];
  let mut itlast = vlast.iter_mut();
  assert_eq!(itlast.nth(13).unwrap(), &mut ZST {});
}

#[test]
fn iter_mut_nth_back() {
  let mut v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut i3 = v3.iter_mut();
  assert_eq!(i3.nth_back(2).unwrap(), &mut ZST {});
  assert_eq!(i3.as_slice(), &mut [ZST {}]);
  assert_eq!(i3.nth_back(0).unwrap(), &mut ZST {});
  assert_eq!(i3.nth_back(0), None);
  let mut v4 = staticvec![1, 2, 3, 4];
  let mut i4 = v4.iter_mut();
  assert_eq!(i4.nth_back(2).unwrap(), &mut 2);
  assert_eq!(i4.as_slice(), &[1]);
  assert_eq!(i4.nth_back(0).unwrap(), &mut 1);
  assert_eq!(i4.nth_back(0), None);
  let mut xs = staticvec![0, 1, 2, 3, 4, 5];
  let mut it = xs.iter_mut().enumerate();
  while let Some((i, &mut x)) = it.nth_back(0) {
    assert_eq!(i, x);
  }
  let mut it = xs.iter_mut().enumerate();
  while let Some((i, &mut x)) = it.nth_back(1) {
    assert_eq!(i, x);
  }
  let (i, &mut x) = xs.iter_mut().enumerate().nth_back(3).unwrap();
  assert_eq!(i, x);
  assert_eq!(i, 2);
  let mut xs5 = staticvec![vec![1], vec![2], vec![3], vec![4], vec![5]];
  let mut it5 = xs5.iter_mut();
  assert_eq!(it5.nth_back(1).unwrap(), &mut vec![4]);
  assert_eq!(it5.as_slice(), &[vec![1], vec![2], vec![3]]);
  assert_eq!(it5.next().unwrap(), &mut vec![1]);
  assert_eq!(it5.next_back().unwrap(), &mut vec![3]);
  assert_eq!(it5.nth_back(0).unwrap(), &mut vec![2]);
  let mut xs6 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it6 = xs6.iter_mut();
  let o = it6.nth_back(2);
  assert_eq!(format!("{:?}", o), "Some([4, 4])");
  assert_eq!(
    format!("{:?}", it6),
    "StaticVecIterMut([[1, 1], [2, 2], [3, 3]])"
  );
  let mut xs7 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it7 = xs7.iter_mut();
  let o = it7.nth_back(5);
  assert_eq!(format!("{:?}", o), "Some([1, 1])");
  assert_eq!(format!("{:?}", it7), "StaticVecIterMut([])");
}

#[test]
fn into_inner() {
  // Someone ELI5 why "box syntax" isn't more widely used... If I'd have known about it sooner I'd
  // have never once used `Box::new()` in any of these tests (something I now feel like I'm
  // ultimately gonna want to go back and change to just `box` at some point for each of them.)
  let v: StaticVec<Box<i32>, 12> = staticvec![
    box 1, box 2, box 3, box 4, box 5, box 6, box 7, box 8, box 9, box 10, box 11, box 12
  ];
  let z = v.into_inner();
  assert!(z.is_ok());
  assert_eq!(
    z.unwrap(),
    [box 1, box 2, box 3, box 4, box 5, box 6, box 7, box 8, box 9, box 10, box 11, box 12]
  );
  let vv: StaticVec<Vec<Vec<u32>>, 4> =
    staticvec![vec![vec![1]], vec![vec![2]], vec![vec![3]], vec![vec![4]]];
  let zz = vv.into_inner();
  assert!(zz.is_ok());
  assert_eq!(
    zz.unwrap(),
    [vec![vec![1]], vec![vec![2]], vec![vec![3]], vec![vec![4]]]
  );
  let mut vvv = staticvec![box 9, box 1, box 1];
  vvv.pop();
  let zzz = vvv.into_inner();
  assert!(zzz.is_err());
}

#[test]
fn into_iter() {
  let v = staticvec![1, 2, 3, 4, 5];
  let mut i = v.into_iter();
  assert_eq!(i.next().unwrap(), 1);
  assert_eq!(i.next_back().unwrap(), 5);
  assert_eq!("StaticVecIntoIter([2, 3, 4])", format!("{:?}", i));
  assert_eq!(i.next().unwrap(), 2);
  assert_eq!(i.next_back().unwrap(), 4);
  assert_eq!("StaticVecIntoIter([3])", format!("{:?}", i));
  assert_eq!(i.next().unwrap(), 3);
  assert_eq!("StaticVecIntoIter([])", format!("{:?}", i));
  let v2 = staticvec![
    box Struct { s: "AAA" },
    box Struct { s: "BBB" },
    box Struct { s: "CCC" },
  ];
  let mut i2 = v2.into_iter();
  assert_eq!(i2.next().unwrap(), box Struct { s: "AAA" });
  assert_eq!(i2.next().unwrap(), box Struct { s: "BBB" });
  assert_eq!(i2.next().unwrap(), box Struct { s: "CCC" });
  assert_eq!("StaticVecIntoIter([])", format!("{:?}", i2));
  let v3 = staticvec![
    box Struct { s: "AAA" },
    box Struct { s: "BBB" },
    box Struct { s: "CCC" },
  ];
  let mut i3 = v3.into_iter();
  // We do this so Miri can make sure it drops the remaining values properly.
  i3.next();
  let v4 = staticvec![ZST {}, ZST {}, ZST {}];
  let mut i4 = v4.into_iter();
  // We do this so Miri can make sure it drops the remaining values properly.
  i4.next();
  let v5 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut it5 = v5.into_iter();
  assert_eq!(it5.as_slice(), &[ZST {}, ZST {}, ZST {}, ZST {}]);
  assert_eq!(it5.as_mut_slice(), &mut [ZST {}, ZST {}, ZST {}, ZST {}]);
  let a1 = staticvec![box 1, box 2, box 3];
  let a2 = staticvec![box 4, box 5, box 6];
  let iter = a1.into_iter().zip(a2.into_iter());
  let mut a3 = iter.collect::<StaticVec<(Box<i32>, Box<i32>), 6>>();
  for tup in a3.iter_mut() {
    *tup.0 += 1;
    *tup.1 += 1;
  }
  assert_eq!(
    a3,
    staticvec![(box 2, box 5), (box 3, box 6), (box 4, box 7)]
  );
  // Make sure cloning doesn't cause any issues.
  let a4 = staticvec![box 1, box 2, box 3, box 4, box 5, box 6];
  let mut iter2 = a4.into_iter();
  iter2.next();
  iter2.next_back();
  iter2.next();
  let iter3 = iter2.clone();
  assert_eq!(iter2.as_slice(), iter3.as_slice());
  // Needs to work properly with ZSTs too of course.
  let a5 = staticvec![
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST
  ];
  let mut iter4 = a5.into_iter();
  iter4.nth(0);
  iter4.next_back();
  iter4.next();
  let mut iter5 = iter4.clone();
  assert_eq!(iter4.as_slice(), iter5.as_slice());
  for x in iter4.as_slice().iter() {
    assert_eq!(x, &CloneableZST);
  }
  for y in iter5.as_mut_slice().iter_mut() {
    assert_eq!(y, &mut CloneableZST);
  }
  let mut i = 0;
  for x in staticvec![
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST,
    CloneableZST
  ] {
    assert_eq!(x, CloneableZST);
    i += 1;
  }
  assert_eq!(i, 16);
}

#[test]
fn into_iter_nth() {
  let v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut i3 = v3.into_iter();
  assert_eq!(i3.nth(2).unwrap(), ZST {});
  assert_eq!(i3.as_slice(), [ZST {}]);
  assert_eq!(i3.nth(0).unwrap(), ZST {});
  assert_eq!(i3.nth(0), None);
  assert_eq!(i3.nth(0), None);
  let v4 = staticvec![1, 2, 3, 4];
  let mut i4 = v4.into_iter();
  assert_eq!(i4.nth(2).unwrap(), 3);
  assert_eq!(i4.as_slice(), [4]);
  assert_eq!(i4.nth(0).unwrap(), 4);
  assert_eq!(i4.nth(0), None);
  assert_eq!(i4.nth(0), None);
  let xs1 = staticvec![0, 1, 2, 3, 4, 5];
  for (i, x) in xs1.into_iter().enumerate() {
    assert_eq!(i, x);
  }
  let xs2 = staticvec![0, 1, 2, 3, 4, 5];
  let mut it2 = xs2.into_iter().enumerate();
  while let Some((i, x)) = it2.nth(0) {
    assert_eq!(i, x);
  }
  let xs3 = staticvec![0, 1, 2, 3, 4, 5];
  let mut it3 = xs3.into_iter().enumerate();
  while let Some((i, x)) = it3.nth(1) {
    assert_eq!(i, x);
  }
  let xs4 = staticvec![0, 1, 2, 3, 4, 5];
  let (i, x) = xs4.into_iter().enumerate().nth(3).unwrap();
  assert_eq!(i, x);
  assert_eq!(i, 3);
  // We use "StaticVecs of Vec" below to test the functionality for non-trivial "need Drop" types.
  let xs5 = staticvec![vec![1], vec![2], vec![3], vec![4], vec![5]];
  let mut it5 = xs5.into_iter();
  assert_eq!(it5.nth(2).unwrap(), vec![3]);
  assert_eq!(it5.as_slice(), &[vec![4], vec![5]]);
  assert_eq!(it5.next().unwrap(), vec![4]);
  assert_eq!(it5.next_back().unwrap(), vec![5]);
  assert_eq!(it5.nth(0), None);
  let xs6 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it6 = xs6.into_iter();
  let o = it6.nth(2);
  assert_eq!(format!("{:?}", o), "Some([3, 3])");
  assert_eq!(
    format!("{:?}", it6),
    "StaticVecIntoIter([[4, 4], [5, 5], [6, 6]])"
  );
  let xs7 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it7 = xs7.into_iter();
  let o = it7.nth(5);
  assert_eq!(format!("{:?}", o), "Some([6, 6])");
  assert_eq!(format!("{:?}", it7), "StaticVecIntoIter([])");
  let vlast = staticvec![
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {},
    ZST {}
  ];
  let mut itlast = vlast.into_iter();
  assert_eq!(itlast.nth(13).unwrap(), ZST {});
}

#[test]
fn into_iter_nth_back() {
  let v3 = staticvec![ZST {}, ZST {}, ZST {}, ZST {}];
  let mut i3 = v3.into_iter();
  assert_eq!(i3.nth_back(2).unwrap(), ZST {});
  assert_eq!(i3.as_slice(), [ZST {}]);
  assert_eq!(i3.nth_back(0).unwrap(), ZST {});
  assert_eq!(i3.nth_back(0), None);
  assert_eq!(i3.nth_back(0), None);
  let v4 = staticvec![1, 2, 3, 4];
  let mut i4 = v4.into_iter();
  assert_eq!(i4.nth_back(2).unwrap(), 2);
  assert_eq!(i4.as_slice(), [1]);
  assert_eq!(i4.nth_back(0).unwrap(), 1);
  assert_eq!(i4.nth_back(0), None);
  assert_eq!(i4.nth_back(0), None);
  let xs1 = staticvec![0, 1, 2, 3, 4, 5];
  let mut it1 = xs1.into_iter().enumerate();
  while let Some((i, x)) = it1.nth_back(0) {
    assert_eq!(i, x);
  }
  let xs2 = staticvec![0, 1, 2, 3, 4, 5];
  let mut it2 = xs2.into_iter().enumerate();
  while let Some((i, x)) = it2.nth_back(1) {
    assert_eq!(i, x);
  }
  let xs3 = staticvec![0, 1, 2, 3, 4, 5];
  let (i, x) = xs3.into_iter().enumerate().nth_back(3).unwrap();
  assert_eq!(i, x);
  assert_eq!(i, 2);
  // We use "StaticVecs of Vec" below to test the functionality for non-trivial "need Drop" types.
  let xs5 = staticvec![vec![1], vec![2], vec![3], vec![4], vec![5]];
  let mut it5 = xs5.into_iter();
  assert_eq!(it5.nth_back(1).unwrap(), vec![4]);
  assert_eq!(it5.as_slice(), &[vec![1], vec![2], vec![3]]);
  assert_eq!(it5.next().unwrap(), vec![1]);
  assert_eq!(it5.next_back().unwrap(), vec![3]);
  assert_eq!(it5.nth_back(0).unwrap(), vec![2]);
  let xs6 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it6 = xs6.into_iter();
  let o = it6.nth_back(2);
  assert_eq!(format!("{:?}", o), "Some([4, 4])");
  assert_eq!(
    format!("{:?}", it6),
    "StaticVecIntoIter([[1, 1], [2, 2], [3, 3]])"
  );
  let xs7 = staticvec![
    vec![1, 1],
    vec![2, 2],
    vec![3, 3],
    vec![4, 4],
    vec![5, 5],
    vec![6, 6]
  ];
  let mut it7 = xs7.into_iter();
  let o = it7.nth_back(5);
  assert_eq!(format!("{:?}", o), "Some([1, 1])");
  assert_eq!(format!("{:?}", it7), "StaticVecIntoIter([])");
}

#[cfg(feature = "std")]
#[test]
fn into_vec() {
  let v = staticvec![
    box Struct { s: "AAA" },
    box Struct { s: "BBB" },
    box Struct { s: "CCC" }
  ];
  let vv = v.into_vec();
  assert_eq!(vv.capacity(), 3);
  assert_eq!(vv.len(), 3);
}

#[test]
fn last() {
  let v = staticvec![1, 2, 3];
  assert_eq!(*v.last().unwrap(), 3);
}

#[test]
fn last_mut() {
  let mut v = staticvec![1, 2, 3];
  assert_eq!(*v.last_mut().unwrap(), 3);
}

#[test]
fn len() {
  let a = staticvec![1, 2, 3];
  assert_eq!(a.len(), 3);
}

#[test]
fn macros() {
  let v = staticvec![staticvec![staticvec![1, 2, 3, 4]]];
  assert_eq!(v[0][0], [1, 2, 3, 4]);
  let v2 = staticvec![12.0; 64];
  assert!(v2 == [12.0; 64]);
  const V3: StaticVec<i32, 4> = staticvec![1, 2, 3, 4];
  assert_eq!(V3, [1, 2, 3, 4]);
  const V4: StaticVec<i32, 128> = staticvec![27; 128];
  assert!(V4 == [27; 128]);
  static V: StaticVec<f64, 3> = sortedstaticvec!(f64, [16.0, 15.0, 14.0]);
  assert_eq!(V, [14.0, 15.0, 16.0]);
  assert_eq!(V.reversed().drain(0..1), [16.0]);
  static VV: StaticVec<f64, 0> = sortedstaticvec!(f64, []);
  assert_eq!(VV, []);
  // Test trailing commas
  assert_eq!(staticvec![1, 2, 3, 4,], staticvec![1, 2, 3, 4]);
}

#[test]
fn math_functions() {
  static A: StaticVec<f64, 4> = staticvec![4.0, 5.0, 6.0, 7.0];
  static B: StaticVec<f64, 4> = staticvec![2.0, 3.0, 4.0, 5.0];
  assert_eq!(A.added(&B), [6.0, 8.0, 10.0, 12.0]);
  assert_eq!(A.subtracted(&B), [2.0, 2.0, 2.0, 2.0]);
  assert_eq!(A.multiplied(&B), [8.0, 15.0, 24.0, 35.0]);
  assert_eq!(A.divided(&B), [2.0, 1.6666666666666667, 1.5, 1.4]);
}

#[test]
fn mut_ptr_at() {
  let mut v = staticvec![1, 2, 3];
  unsafe { assert_eq!(*v.mut_ptr_at(0), 1) };
  unsafe { assert_eq!(*v.mut_ptr_at(1), 2) };
  unsafe { assert_eq!(*v.mut_ptr_at(2), 3) };
}

#[test]
fn mut_ptr_at_unchecked() {
  let mut v = staticvec![1, 2, 3];
  unsafe { assert_eq!(*v.mut_ptr_at_unchecked(0), 1) };
  unsafe { assert_eq!(*v.mut_ptr_at_unchecked(1), 2) };
  unsafe { assert_eq!(*v.mut_ptr_at_unchecked(2), 3) };
}

#[test]
fn new() {
  let v = StaticVec::<i32, 1>::new();
  assert_eq!(v.capacity(), 1);
}

#[test]
fn new_from_array() {
  let vec = StaticVec::<i32, 3>::new_from_array([1; 3]);
  assert_eq!(vec, [1, 1, 1]);
  let vec2 = StaticVec::<i32, 3>::new_from_array([1; 6]);
  assert_eq!(vec2, [1, 1, 1]);
  let vec3 = StaticVec::<i32, 27>::new_from_array([0; 0]);
  assert_eq!(vec3, []);
  let vec4 = StaticVec::<f32, 1024>::new_from_array([24.0; 512]);
  assert_eq!(vec4, staticvec![24.0; 512]);
  let v = StaticVec::<i32, 3>::new_from_array([1, 2, 3]);
  assert_eq!(v, [1, 2, 3]);
  let v2 = StaticVec::<i32, 3>::new_from_array([1, 2, 3, 4, 5, 6]);
  assert_eq!(v2, [1, 2, 3]);
  let v5 = StaticVec::<Box<Struct>, 2>::new_from_array([
    box Struct { s: "AAA" },
    box Struct { s: "BBB" },
    box Struct { s: "CCC" },
    box Struct { s: "DDD" },
    box Struct { s: "EEE" },
  ]);
  assert_eq!(v5, [box Struct { s: "AAA" }, box Struct { s: "BBB" }]);
  let v6 = StaticVec::<Box<i32>, 0>::new_from_array([box 1, box 2, box 3]);
  assert_eq!(v6, []);
}

#[test]
fn new_from_const_array() {
  const VEC2: StaticVec<i32, 6> = StaticVec::new_from_const_array([1; 6]);
  assert_eq!(VEC2, [1, 1, 1, 1, 1, 1]);
  const VEC3: StaticVec<i32, 0> = StaticVec::new_from_const_array([0; 0]);
  assert_eq!(VEC3, []);
  const VEC4: StaticVec<f32, 512> = StaticVec::new_from_const_array([24.0; 512]);
  assert_eq!(VEC4, staticvec![24.0; 512]);
  const V: StaticVec<&'static str, 3> = StaticVec::new_from_const_array(["A", "B", "C"]);
  assert_eq!(V.reversed(), ["C", "B", "A"]);
  const V2: StaticVec<u8, 6> = StaticVec::new_from_const_array([1, 2, 3, 4, 5, 6]);
  assert_eq!(V2, [1, 2, 3, 4, 5, 6]);
  const V6: StaticVec<Struct, 3> = StaticVec::new_from_const_array([
    Struct { s: "AAA" },
    Struct { s: "BBB" },
    Struct { s: "CCC" },
  ]);
  assert_eq!(
    V6,
    [
      Struct { s: "AAA" },
      Struct { s: "BBB" },
      Struct { s: "CCC" },
    ]
  );
}

#[test]
fn new_from_slice() {
  let vec = StaticVec::<i32, 3>::new_from_slice(&[1, 2, 3]);
  assert_eq!(vec, [1, 2, 3]);
  let vec2 = StaticVec::<i32, 3>::new_from_slice(&[1, 2, 3, 4, 5, 6]);
  assert_eq!(vec2, [1, 2, 3]);
  let vec3 = StaticVec::<i32, 27>::new_from_slice(&[]);
  assert_eq!(vec3, []);
  let vec4 = StaticVec::<i32, 0>::new_from_slice(&[1, 2, 3]);
  assert_eq!(vec4, []);
}

#[test]
fn partial_eq() {
  assert_eq!(StaticVec::<i32, 0>::new(), [0; 0]);
  assert_eq!(StaticVec::<i32, 0>::new(), []);
  assert_eq!(StaticVec::<i32, 0>::new(), &[]);
  assert_eq!(StaticVec::<i32, 0>::new(), &mut []);
  assert_eq!(StaticVec::<i32, 0>::new(), StaticVec::<i32, 0>::new());
  assert_eq!(StaticVec::<i32, 0>::new(), &StaticVec::<i32, 0>::new());
  assert_eq!(StaticVec::<i32, 0>::new(), &mut StaticVec::<i32, 0>::new());
  // assert_eq! is written in a way that's limited by LengthAtMost32, so I can't
  // use it for the next part.
  if staticvec![1; 64] != [1; 64] {
    panic!();
  }
  if &staticvec![1; 64] != [1; 64] {
    panic!();
  }
  if &mut staticvec![1; 64] != [1; 64] {
    panic!();
  }
  if staticvec![1; 64] != &[1; 64] {
    panic!();
  }
  if staticvec![1; 64] != &mut [1; 64] {
    panic!();
  }
  if staticvec![1; 64] != staticvec![1; 64] {
    panic!();
  }
  if staticvec![1; 64] != &staticvec![1; 64] {
    panic!();
  }
  if staticvec![1; 64] != &mut staticvec![1; 64] {
    panic!();
  }
  if staticvec![1.0; 64] != [1.0; 64] {
    panic!();
  }
  if &staticvec![1.0; 64] != [1.0; 64] {
    panic!();
  }
  if &mut staticvec![1.0; 64] != [1.0; 64] {
    panic!();
  }
  if staticvec![1.0; 64] != &[1.0; 64] {
    panic!();
  }
  if staticvec![1.0; 64] != &mut [1.0; 64] {
    panic!();
  }
  if staticvec![1.0; 64] != staticvec![1.0; 64] {
    panic!();
  }
  if staticvec![1.0; 64] != &staticvec![1.0; 64] {
    panic!();
  }
  if staticvec![1.0; 64] != &mut staticvec![1.0; 64] {
    panic!();
  }
}

#[test]
fn partial_ord() {
  assert!(staticvec![1] < staticvec![2]);
  assert!(staticvec![1] > []);
  assert!(staticvec![1] <= &staticvec![2]);
  assert!(staticvec![1] >= &[]);
  assert!(staticvec![1] > &mut []);
  assert!(staticvec![vec![1], vec![2]] < staticvec![vec![1], vec![2], vec![3]]);
  assert!(staticvec![vec![1]] > []);
  assert!(staticvec![vec![1]] <= &staticvec![vec![2]]);
  assert!(staticvec![vec![1]] >= &[]);
  assert!(staticvec![vec![1]] > &mut []);
  assert!(staticvec![1.23] < staticvec![2.33]);
  assert!(staticvec![1.23] > []);
  assert!(staticvec![1.23] <= &staticvec![2.33]);
  assert!(staticvec![1.23] >= &[]);
  assert!(staticvec![1.23] > &mut []);
  assert!(staticvec![vec![1.23], vec![2.33]] < staticvec![vec![1.23], vec![2.33], vec![3.33]]);
  assert!(staticvec![vec![1.23]] > []);
  assert!(staticvec![vec![1.23]] <= &staticvec![vec![2.33]]);
  assert!(staticvec![vec![1.23]] >= &[]);
  assert!(staticvec![vec![1.23]] > &mut []);
}

#[test]
fn pop() {
  let mut vec = staticvec![box 1, box 2, box 3];
  assert_eq!(vec.pop(), Some(box 3));
  assert_eq!(vec, [box 1, box 2]);
  assert_eq!(vec.pop(), Some(box 2));
  assert_eq!(vec, [box 1]);
  assert_eq!(vec.pop(), Some(box 1));
  assert_eq!(vec, []);
  assert_eq!(vec.pop(), None);
}

#[test]
fn ptr_at() {
  let v = staticvec![1, 2, 3];
  unsafe { assert_eq!(*v.ptr_at(0), 1) };
  unsafe { assert_eq!(*v.ptr_at(1), 2) };
  unsafe { assert_eq!(*v.ptr_at(2), 3) };
}

#[test]
fn ptr_at_unchecked() {
  let v = staticvec![1, 2, 3];
  unsafe { assert_eq!(*v.ptr_at_unchecked(0), 1) };
  unsafe { assert_eq!(*v.ptr_at_unchecked(1), 2) };
  unsafe { assert_eq!(*v.ptr_at_unchecked(2), 3) };
}

#[test]
fn push() {
  let mut vec = StaticVec::<i32, 4>::from(&[1, 2, 3]);
  vec.push(3);
  assert_eq!(vec, [1, 2, 3, 3]);
}

#[test]
#[should_panic]
fn push_panic() {
  let mut v = staticvec![box 1, box 2, box 3];
  v.push(box 12);
}

#[test]
fn quicksorted_unstable() {
  let v: StaticVec<StaticVec<i32, 3>, 2> = staticvec![staticvec![1, 2, 3], staticvec![6, 5, 4]];
  assert_eq!(
    v.iter()
      .flatten()
      .collect::<StaticVec<i32, 6>>()
      .quicksorted_unstable(),
    [1, 2, 3, 4, 5, 6]
  );
  let v2 = StaticVec::<i32, 128>::new();
  assert_eq!(v2.quicksorted_unstable(), []);
  assert_eq!(staticvec![2, 1].quicksorted_unstable(), [1, 2]);
}

#[test]
fn quicksort_unstable() {
  let v1 = staticvec![staticvec![1, 2, 3], staticvec![6, 5, 4]];
  let mut v2 = v1.iter().flatten().collect::<StaticVec<i32, 6>>();
  v2.quicksort_unstable();
  assert_eq!(v2, [1, 2, 3, 4, 5, 6]);
  let mut v3 = StaticVec::<i32, 128>::new();
  v3.quicksort_unstable();
  assert_eq!(v3, []);
  let mut v4 = staticvec![2, 1];
  v4.quicksort_unstable();
  assert_eq!(v4, [1, 2]);
}

#[cfg(feature = "std")]
mod read_tests {
  use core::mem::MaybeUninit;
  use staticvec::*;
  use std::io::{self, BorrowedBuf, BufRead, Read};

  // We provide custom implementations of most `Read` methods; test those impls
  #[test]
  fn read() {
    let mut ints = staticvec![1, 2, 3, 4, 6, 7, 8, 9, 10];
    let mut buffer = [0, 0, 0, 0];
    assert_eq!(ints.read(&mut buffer).unwrap(), 4);
    assert_eq!(buffer, [1, 2, 3, 4]);
    let mut buffer2 = [];
    assert_eq!(ints.read(&mut buffer2).unwrap(), 0);
    assert_eq!(buffer2, []);
    let mut buffer3 = staticvec![0; 9];
    assert_eq!(ints.read(buffer3.as_mut_slice()).unwrap(), 5);
    assert_eq!(ints, []);
    assert_eq!(ints.read(buffer3.as_mut_slice()).unwrap(), 0);
    assert_eq!(ints, []);
    assert_eq!(ints.read(staticvec![].as_mut_slice()).unwrap(), 0);
  }

  #[test]
  fn read_to_end() {
    let mut ints = staticvec![1, 2, 3, 4, 5, 6, 7];
    let mut buffer = vec![2, 3];
    assert_eq!(ints.read_to_end(&mut buffer).unwrap(), 7);
    assert_eq!(ints, &[]);
    assert_eq!(buffer, &[2, 3, 1, 2, 3, 4, 5, 6, 7]);
  }

  #[test]
  fn read_to_string() {
    // Hello world in ascii
    let mut input = StaticVec::<u8, 30>::new_from_slice(b"World!");
    let mut dest = String::from("Hello, ");
    assert_eq!(input.read_to_string(&mut dest).unwrap(), 6);
    assert_eq!(dest, "Hello, World!");
    assert_eq!(input, &[]);
  }

  #[test]
  fn read_to_string_failure() {
    // Invalid UTF-8 bytes
    let mut input = staticvec![0b1101_1010, 0b1100_0000];
    let mut dest = String::new();
    let err = input.read_to_string(&mut dest).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
  }

  #[test]
  fn read_exact() {
    let mut ints = staticvec![1, 2, 3, 4, 6, 7, 8, 9, 10];
    let mut buffer = [0, 0, 0, 0];
    ints.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer, [1, 2, 3, 4]);
    assert_eq!(ints, &[6, 7, 8, 9, 10]);
    let mut buffer2 = [0, 0, 0, 0, 0, 0, 0, 0];
    let err = ints.read_exact(&mut buffer2).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
  }

  #[test]
  fn read_vectored() {
    let mut ints = staticvec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut buf1 = [0; 4];
    let mut buf2 = [0; 4];
    let mut buf3 = [0; 4];
    let mut bufs = [
      io::IoSliceMut::new(&mut buf1),
      io::IoSliceMut::new(&mut buf2),
      io::IoSliceMut::new(&mut buf3),
    ];
    assert_eq!(ints.read_vectored(&mut bufs).unwrap(), 12);
    assert_eq!(
      "[[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]",
      format!("{:?}", bufs)
    );
    assert_eq!(ints, []);
    let mut ints2 = staticvec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut buf4 = [0; 2];
    let mut buf5 = [0; 3];
    let mut buf6 = [0; 4];
    let mut bufs2 = [
      io::IoSliceMut::new(&mut buf4),
      io::IoSliceMut::new(&mut buf5),
      io::IoSliceMut::new(&mut buf6),
    ];
    assert_eq!(ints2.read_vectored(&mut bufs2).unwrap(), 9);
    assert_eq!("[[1, 2], [3, 4, 5], [6, 7, 8, 9]]", format!("{:?}", bufs2));
    assert_eq!(ints2, [10, 11, 12]);
  }

  #[test]
  fn read_buf() {
    let inner: &[u8] = &[5, 6, 7, 0, 1, 2, 3, 4];
    let mut reader = StaticVec::<u8, 8>::from(inner);
    let buf: &mut [_] = &mut [MaybeUninit::uninit(); 3];
    let mut buf: BorrowedBuf<'_> = buf.into();
    reader.read_buf(buf.unfilled()).unwrap();
    assert_eq!(buf.filled(), [5, 6, 7]);
    assert_eq!(reader, [0, 1, 2, 3, 4]);
    let buf: &mut [_] = &mut [MaybeUninit::uninit(); 2];
    let mut buf: BorrowedBuf<'_> = buf.into();
    reader.read_buf(buf.unfilled()).unwrap();
    assert_eq!(buf.filled(), [0, 1]);
    assert_eq!(reader, [2, 3, 4]);
    let buf: &mut [_] = &mut [MaybeUninit::uninit(); 1];
    let mut buf: BorrowedBuf<'_> = buf.into();
    reader.read_buf(buf.unfilled()).unwrap();
    assert_eq!(buf.filled(), [2]);
    assert_eq!(reader, [3, 4]);
    let buf: &mut [_] = &mut [MaybeUninit::uninit(); 3];
    let mut buf: BorrowedBuf<'_> = buf.into();
    reader.read_buf(buf.unfilled()).unwrap();
    assert_eq!(buf.filled(), [3, 4]);
    assert_eq!(reader, []);
    reader.read_buf(buf.unfilled()).unwrap();
    assert_eq!(buf.filled(), [3, 4]);
    assert_eq!(reader, []);
    buf.clear();
    reader.read_buf(buf.unfilled()).unwrap();
    assert!(buf.filled().is_empty());
  }

  #[test]
  fn bufread() {
    let mut cursor = StaticVec::<u8, 7>::from("foo\nbar".as_bytes());
    let mut buf = String::new();
    let num_bytes = cursor
      .read_line(&mut buf)
      .expect("reading from cursor won't fail");
    assert_eq!(num_bytes, 4);
    assert_eq!(buf, "foo\n");
    buf.clear();
    let num_bytes = cursor
      .read_line(&mut buf)
      .expect("reading from cursor won't fail");
    assert_eq!(num_bytes, 3);
    assert_eq!(buf, "bar");
    buf.clear();
    let num_bytes = cursor
      .read_line(&mut buf)
      .expect("reading from cursor won't fail");
    assert_eq!(num_bytes, 0);
    assert_eq!(buf, "");
    let cursor2 = StaticVec::<u8, 18>::from("lorem\nipsum\r\ndolor".as_bytes());
    let mut lines_iter = cursor2.lines().map(|l| l.unwrap());
    assert_eq!(lines_iter.next(), Some(String::from("lorem")));
    assert_eq!(lines_iter.next(), Some(String::from("ipsum")));
    assert_eq!(lines_iter.next(), Some(String::from("dolor")));
    assert_eq!(lines_iter.next(), None);
    let mut cursor3 = StaticVec::<u8, 11>::from("lorem-ipsum".as_bytes());
    let mut buf = vec![];
    let num_bytes = cursor3
      .read_until(b'-', &mut buf)
      .expect("reading from cursor won't fail");
    assert_eq!(num_bytes, 6);
    assert_eq!(buf, b"lorem-");
    buf.clear();
    let num_bytes = cursor3
      .read_until(b'-', &mut buf)
      .expect("reading from cursor won't fail");
    assert_eq!(num_bytes, 5);
    assert_eq!(buf, b"ipsum");
    buf.clear();
    let num_bytes = cursor3
      .read_until(b'-', &mut buf)
      .expect("reading from cursor won't fail");
    assert_eq!(num_bytes, 0);
    assert_eq!(buf, b"");
    let cursor4 = StaticVec::<u8, 17>::from("lorem-ipsum-dolor".as_bytes());
    let mut split_iter = cursor4.split(b'-').map(|l| l.unwrap());
    assert_eq!(split_iter.next(), Some(b"lorem".to_vec()));
    assert_eq!(split_iter.next(), Some(b"ipsum".to_vec()));
    assert_eq!(split_iter.next(), Some(b"dolor".to_vec()));
    assert_eq!(split_iter.next(), None);
  }
}

#[test]
fn remaining_capacity() {
  let mut v = StaticVec::<i32, 3>::new();
  v.push(12);
  assert_eq!(v.remaining_capacity(), 2);
}

#[test]
fn remove() {
  let mut v = staticvec![1, 2, 3];
  assert_eq!(v.remove(1), 2);
  assert_eq!(v, [1, 3]);
}

#[test]
#[should_panic]
fn remove_panic() {
  let mut v = staticvec![1, 2, 3];
  v.remove(128);
}

#[test]
fn remove_item() {
  let mut vec = staticvec![1, 2, 3, 1];
  vec.remove_item(&1);
  assert_eq!(vec, staticvec![2, 3, 1]);
}

#[test]
fn retain() {
  let mut vec = staticvec![1, 2, 3, 4, 5];
  let keep = [false, true, true, false, true];
  let mut i = 0;
  vec.retain(|_| (keep[i], i += 1).0);
  assert_eq!(vec, [2, 3, 5]);
}

#[test]
fn reversed() {
  let v = staticvec![1, 2, 3].reversed();
  assert!(v == [3, 2, 1]);
  let mut x = StaticVec::<f64, 24>::new();
  let mut y = StaticVec::<f64, 12>::new();
  for _ in 0..12 {
    y.push(12.0);
  }
  x.append(&mut y);
  assert_eq!(x.reversed().len(), 12);
  assert_eq!(
    x.reversed(),
    [12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0]
  );
}

#[test]
fn size_in_bytes() {
  let x = StaticVec::<u8, 8>::from([1, 2, 3, 4, 5, 6, 7, 8]);
  assert_eq!(x.size_in_bytes(), 8);
  let y = StaticVec::<u16, 8>::from([1, 2, 3, 4, 5, 6, 7, 8]);
  assert_eq!(y.size_in_bytes(), 16);
  let z = StaticVec::<u32, 8>::from([1, 2, 3, 4, 5, 6, 7, 8]);
  assert_eq!(z.size_in_bytes(), 32);
  let w = StaticVec::<u64, 8>::from([1, 2, 3, 4, 5, 6, 7, 8]);
  assert_eq!(w.size_in_bytes(), 64);
}

#[test]
fn set_len() {
  let mut v = staticvec![1, 2, 3];
  assert_eq!(v.len(), 3);
  unsafe { v.set_len(0) };
  assert_eq!(v.len(), 0);
}

#[cfg(feature = "std")]
#[test]
fn sorted() {
  const V: StaticVec<StaticVec<i32, 3>, 2> = staticvec![staticvec![1, 2, 3], staticvec![6, 5, 4]];
  assert_eq!(
    V.iter().flatten().collect::<StaticVec<i32, 6>>().sorted(),
    [1, 2, 3, 4, 5, 6]
  );
  let v2 = StaticVec::<i32, 128>::new();
  assert_eq!(v2.sorted(), []);
  assert_eq!(staticvec![2, 1].sorted(), [1, 2]);
}

#[test]
fn sorted_unstable() {
  const V: StaticVec<StaticVec<i32, 3>, 2> = staticvec![staticvec![1, 2, 3], staticvec![6, 5, 4]];
  assert_eq!(
    V.iter()
      .flatten()
      .collect::<StaticVec<i32, 6>>()
      .sorted_unstable(),
    [1, 2, 3, 4, 5, 6]
  );
  let v2 = StaticVec::<i32, 128>::new();
  assert_eq!(v2.sorted_unstable(), []);
  assert_eq!(staticvec![2, 1].sorted_unstable(), [1, 2]);
}

#[test]
fn splice() {
  let mut v = StaticVec::<i32, 6>::from([1, 2, 3, 4, 5]);
  let a = [10, 11, 12];
  v.splice(2..4, a.iter().cloned());
  assert_eq!(v, &[1, 2, 10, 11, 12, 5]);
  v.splice(1..3, Some(20));
  assert_eq!(v, &[1, 20, 11, 12, 5]);
}

#[test]
fn splice_boxed() {
  let mut v = StaticVec::<Box<i32>, 6>::from([box 1, box 2, box 3, box 4, box 5]);
  let a = staticvec![box 10, box 11, box 12];
  v.splice(2..4, a.into_iter());
  assert_eq!(v, &[box 1, box 2, box 10, box 11, box 12, box 5]);
  v.splice(1..3, Some(box 20));
  assert_eq!(v, &[box 1, box 20, box 11, box 12, box 5]);
}

#[test]
fn splice_debug_impl() {
  let mut v = StaticVec::<Box<i32>, 8>::from([box 1, box 2, box 3, box 4]);
  let v2 = staticvec![box 6, box 7, box 8, box 9, box 10];
  let mut s = v.splice(1..3, v2.iter().cloned());
  assert_eq!(format!("{:?}", s), "StaticVecSplice([2, 3])");
  s.next();
  assert_eq!(format!("{:?}", s), "StaticVecSplice([3])");
  s.next();
  assert_eq!(format!("{:?}", s), "StaticVecSplice([])");
  s.next();
  assert_eq!(format!("{:?}", s), "StaticVecSplice([])");
}

#[test]
#[should_panic]
fn splice_empty() {
  let mut v = StaticVec::<Box<i32>, 0>::new();
  v.splice(12..24, staticvec![box 1, box 2, box 3].into_iter());
  assert_eq!(v, []);
}

#[test]
fn splice_empty_iterator() {
  let mut v = StaticVec::<Box<i32>, 12>::from([box 1, box 2, box 3, box 4, box 5, box 6]);
  v.splice(1..4, StaticVec::<Box<i32>, 0>::new().into_iter());
  assert_eq!(v, [box 1, box 5, box 6]);
}

#[test]
#[should_panic]
fn splice_inclusive_out_of_bounds() {
  let mut v = staticvec![1, 2, 3, 4, 5];
  let a = [10, 11, 12];
  v.splice(5..=5, a.iter().cloned());
}

#[test]
fn splice_inclusive_range() {
  let mut v = StaticVec::<i32, 6>::from([1, 2, 3, 4, 5]);
  let a = [10, 11, 12];
  let t1: StaticVec<_, 2> = v.splice(2..=3, a.iter().cloned()).collect();
  assert_eq!(v, &[1, 2, 10, 11, 12, 5]);
  assert_eq!(t1, &[3, 4]);
  let t2: StaticVec<_, 2> = v.splice(1..=2, Some(20)).collect();
  assert_eq!(v, &[1, 20, 11, 12, 5]);
  assert_eq!(t2, &[2, 10]);
}

#[test]
fn splice_items_zero_sized() {
  let mut vec = staticvec![(), (), ()];
  let vec2 = staticvec![];
  let t: StaticVec<_, 1> = vec.splice(1..2, vec2.iter().cloned()).collect();
  assert_eq!(vec, &[(), ()]);
  assert_eq!(t, &[()]);
}

#[test]
#[should_panic]
fn splice_out_of_bounds() {
  let mut v = staticvec![1, 2, 3, 4, 5];
  let a = [10, 11, 12];
  v.splice(5..6, a.iter().cloned());
}

#[test]
fn splice_single_item_iterator() {
  let mut v = StaticVec::<Box<i32>, 12>::from([box 1, box 2, box 3, box 4, box 5, box 6]);
  v.splice(1..4, StaticVec::<Box<i32>, 1>::from([box 1]).into_iter());
  assert_eq!(v, [box 1, box 1, box 5, box 6]);
}

#[test]
fn splice_unbounded() {
  let mut vec = staticvec![1, 2, 3, 4, 5];
  let t: StaticVec<_, 5> = vec.splice(.., None).collect();
  assert_eq!(vec, &[]);
  assert_eq!(t, &[1, 2, 3, 4, 5]);
}

#[test]
fn split_at() {
  let v1 = staticvec![box 1, box 2, box 3, box 4, box 5, box 6];
  let t1 = v1.split_at::<0>();
  assert_eq!(t1.0, []);
  assert_eq!(t1.1, [box 1, box 2, box 3, box 4, box 5, box 6]);
  let v2 = staticvec![box 1, box 2, box 3, box 4, box 5, box 6];
  let t2 = v2.split_at::<2>();
  assert_eq!(t2.0, [box 1, box 2]);
  assert_eq!(t2.1, [box 3, box 4, box 5, box 6]);
  let v3 = staticvec![box 1, box 2, box 3, box 4, box 5, box 6];
  let t3 = v3.split_at::<6>();
  assert_eq!(t3.0, [box 1, box 2, box 3, box 4, box 5, box 6]);
  assert_eq!(t3.1, []);
}

#[test]
#[should_panic]
fn split_at_assert() {
  let v = StaticVec::<Box<i32>, 12>::new();
  // `v` has a capacity of 12, but a current length of 0, so this should panic.
  let t = v.split_at::<12>();
  assert_eq!(t.0, []);
  assert_eq!(t.1, []);
}

#[test]
fn split_off() {
  let mut vec = staticvec![1, 2, 3];
  let vec2 = vec.split_off(1);
  assert_eq!(vec, [1]);
  assert_eq!(vec2, [2, 3]);
  let mut vec3 = staticvec![box 1, box 2, box 3, box 4];
  let vec4 = vec3.split_off(2);
  assert_eq!(vec3, [box 1, box 2]);
  assert_eq!(vec4, [box 3, box 4]);
}

#[test]
#[should_panic]
fn split_off_assert() {
  let mut vec3 = StaticVec::<i32, 0>::new();
  assert_eq!(vec3.split_off(9000), []);
}

#[test]
fn symmetric_difference() {
  assert_eq!(
    staticvec![1, 2, 3].symmetric_difference(&staticvec![3, 4, 5]),
    [1, 2, 4, 5]
  );
  assert_eq!(
    staticvec![501, 502, 503, 504].symmetric_difference(&staticvec![502, 503, 504, 505]),
    [501, 505]
  );
  assert_eq!(
    staticvec![box 1, box 1, box 1, box 1]
      .symmetric_difference(&staticvec![box 2, box 2, box 2, box 2]),
    [box 1, box 1, box 1, box 1, box 2, box 2, box 2, box 2]
  );
}

#[test]
fn swap_pop() {
  let mut v = staticvec!["foo", "bar", "baz", "qux"];
  assert_eq!(v.swap_pop(1).unwrap(), "bar");
  assert_eq!(v, ["foo", "qux", "baz"]);
  assert_eq!(v.swap_pop(0).unwrap(), "foo");
  assert_eq!(v, ["baz", "qux"]);
  assert_eq!(v.swap_pop(17), None);
}

#[test]
fn swap_remove() {
  let mut v = staticvec!["foo", "bar", "baz", "qux"];
  assert_eq!(v.swap_remove(1), "bar");
  assert_eq!(v, ["foo", "qux", "baz"]);
  assert_eq!(v.swap_remove(0), "foo");
  assert_eq!(v, ["baz", "qux"]);
}

#[test]
fn triple() {
  static V: StaticVec<usize, 4> = staticvec![4, 5, 6, 7];
  assert_eq!(V.triple(), (V.as_ptr(), 4, 4));
}

#[test]
fn triple_mut() {
  let mut v = staticvec![4, 5, 6, 7];
  let mp = v.as_mut_ptr();
  let t = v.triple_mut();
  assert_eq!(t, (mp, 4, 4));
  unsafe { *t.0 = 8 };
  assert_eq!(v, [8, 5, 6, 7]);
}

#[test]
fn truncate() {
  let mut vec = staticvec![1, 2, 3, 4, 5];
  vec.truncate(2);
  assert_eq!(vec, [1, 2]);
  let mut vec2 = staticvec![1, 2, 3, 4, 5];
  vec2.truncate(2);
  assert_eq!(vec2, [1, 2]);
  let mut vec3 = staticvec![1, 2, 3];
  vec3.truncate(0);
  assert_eq!(vec3, []);
  let mut vec4 = staticvec![1, 2, 3, 4];
  vec4.truncate(97);
  assert_eq!(vec4.len(), 4);
  let mut vec5 = staticvec![box 1, box 2, box 3, box 4, box 5];
  vec5.truncate(2);
  assert_eq!(vec5, [box 1, box 2]);
}

#[test]
fn try_extend_from_slice() {
  let mut v = StaticVec::<i32, 3>::from([1, 2, 3]);
  assert_eq!(v.try_extend_from_slice(&[2, 3]), Err(CapacityError::<3> {}));
  let mut w = StaticVec::<i32, 4>::from([1, 2, 3]);
  assert_eq!(w.try_extend_from_slice(&[2]), Ok(()));
}

#[allow(unused_must_use)]
#[test]
fn try_insert() {
  let mut vec = staticvec![1, 2, 3, 4, 5];
  assert_eq!(vec.try_insert(2, 0), Err(CapacityError::<5> {}));
  let mut vec2 = StaticVec::<i32, 4>::new_from_slice(&[1, 2, 3]);
  vec2.try_insert(2, 3);
  assert_eq!(vec2, [1, 2, 3, 3]);
}

#[allow(unused_must_use)]
#[test]
fn try_insert_from_slice() {
  let mut v1 = StaticVec::<usize, 8>::from([1, 2, 3, 4, 7, 8]);
  assert!(v1.try_insert_from_slice(4, &[5, 6]).is_ok());
  assert_eq!(v1, [1, 2, 3, 4, 5, 6, 7, 8]);
  let mut v2 = StaticVec::<usize, 8>::from([1, 2, 3, 4, 7, 8]);
  assert!(v2.try_insert_from_slice(207, &[5, 6]).is_err());
}

#[test]
fn try_push() {
  let mut vec = staticvec![1, 2, 3, 4, 5];
  let err = vec.try_push(2).unwrap_err();
  assert_eq!(err.into_value(), 2);
  let mut vec2 = StaticVec::<i32, 4>::new_from_slice(&[1, 2, 3]);
  assert_eq!(vec2.try_push(3), Ok(()));
  assert_eq!(vec2, [1, 2, 3, 3]);
}

#[test]
fn union() {
  assert_eq!(
    staticvec![1, 2, 3].union(&staticvec![4, 2, 3, 4]),
    [1, 2, 3, 4],
  );
  assert_eq!(
    staticvec![box 5, box 5].union(&staticvec![box 7, box 7]),
    [box 5, box 7]
  );
}

mod fmt_write_tests {
  use core::fmt::{self, Write};
  use core::str::from_utf8;
  use staticvec::*;

  #[test]
  fn write_str() {
    fn writer<W: Write>(f: &mut W, s: &str) -> fmt::Result {
      f.write_str(s)
    }
    // Arbitrarily bigger than it needs to be for the sake of the test.
    let mut buf = StaticVec::<u8, 12>::new();
    // If either unwrap fails here, we've got a problem.
    writer(&mut buf, "hola").unwrap();
    assert_eq!(from_utf8(buf.as_slice()).unwrap(), "hola");
    // Smaller than it needs to be, again for the sake of the test.
    let mut buf2 = StaticVec::<u8, 2>::new();
    // Make sure `Err` is returned when appropriate.
    assert!(writer(&mut buf2, "hola").is_err());
  }

  #[test]
  fn write_char() {
    fn writer<W: Write>(f: &mut W, c: char) -> fmt::Result {
      f.write_char(c)
    }
    // Arbitrarily bigger than it needs to be for the sake of the test.
    let mut buf = StaticVec::<u8, 12>::new();
    // If any of the unwraps fail here, we've got a problem.
    writer(&mut buf, 'a').unwrap();
    writer(&mut buf, 'b').unwrap();
    assert_eq!(from_utf8(buf.as_slice()).unwrap(), "ab");
    // Smaller than it needs to be, again for the sake of the test.
    let mut buf2 = StaticVec::<u8, 1>::new();
    // Make sure `Err` is returned when appropriate.
    assert!(writer(&mut buf2, '👻').is_err());
  }

  #[test]
  fn write_fmt() {
    fn writer<W: Write>(f: &mut W, s: &str) -> fmt::Result {
      f.write_fmt(format_args!("{}", s))
    }
    // Arbitrarily bigger than it needs to be for the sake of the test.
    let mut buf = StaticVec::<u8, 12>::new();
    // If either unwrap fails here, we've got a problem.
    writer(&mut buf, "world").unwrap();
    assert_eq!(from_utf8(buf.as_slice()).unwrap(), "world");
    // Smaller than it needs to be, again for the sake of the test.
    let mut buf2 = StaticVec::<u8, 4>::new();
    // Make sure `Err` is returned when appropriate.
    assert!(writer(&mut buf2, "universe").is_err());
  }
}

#[cfg(feature = "std")]
mod io_write_tests {
  use staticvec::*;
  use std::io::{IoSlice, Write};

  #[test]
  fn write() {
    // From arrayvec
    let mut v = StaticVec::<u8, 8>::new();
    write!(&mut v, "\x01\x02\x03").unwrap();
    assert_eq!(&v[..], &[1, 2, 3]);
    let r = v.write(&[9; 16]).unwrap();
    assert_eq!(r, 5);
    assert_eq!(&v[..], &[1, 2, 3, 9, 9, 9, 9, 9]);
  }

  #[test]
  fn write_all() {
    let mut v = StaticVec::<u8, 6>::new();
    assert!(v.write_all(&[1, 2, 3, 4, 5, 6, 7, 8]).is_err());
    v.clear();
    assert!(v.write_all(&[1, 2, 3, 4, 5, 6]).is_ok());
  }

  #[test]
  fn write_vectored() {
    let mut v = StaticVec::<u8, 8>::new();
    assert_eq!(
      v.write_vectored(&[IoSlice::new(&[1, 2, 3, 4]), IoSlice::new(&[5, 6, 7, 8])])
        .unwrap(),
      8
    );
    assert_eq!(v, [1, 2, 3, 4, 5, 6, 7, 8]);
    let mut v2 = StaticVec::<u8, 4>::new();
    assert_eq!(
      v2.write_vectored(&[IoSlice::new(&[1, 2, 3, 4]), IoSlice::new(&[5, 6, 7, 8])])
        .unwrap(),
      4
    );
    assert_eq!(v2, [1, 2, 3, 4]);
  }
}
