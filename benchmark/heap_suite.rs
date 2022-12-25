#![allow(clippy::all, incomplete_features)]
#![feature(test)]

extern crate test;

use test::Bencher;

use std::time::SystemTime;

use oorandom::Rand64;

use staticvec::{StaticHeap, StaticVec};

#[bench]
fn staticheap_push_random_u64_512(b: &mut Bencher) {
  let mut rng = Rand64::new(
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_nanos(),
  );
  let vec = StaticVec::<u64, 512>::filled_with(|| rng.rand_range(1..641));
  let mut heap = StaticHeap::<u64, 512>::new();
  b.iter(|| {
    for item in &vec {
      heap.push(*item);
    }
    heap.clear();
  });
}

#[bench]
fn staticheap_push_random_u64_1024(b: &mut Bencher) {
  let mut rng = Rand64::new(
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_nanos(),
  );
  let vec = StaticVec::<u64, 1024>::filled_with(|| rng.rand_range(1..1281));
  let mut heap = StaticHeap::<u64, 1024>::new();
  b.iter(|| {
    for item in &vec {
      heap.push(*item);
    }
    heap.clear();
  });
}

#[bench]
fn staticheap_push_random_u64_2048(b: &mut Bencher) {
  let mut rng = Rand64::new(
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_nanos(),
  );
  let vec = StaticVec::<u64, 2048>::filled_with(|| rng.rand_range(1..2561));
  let mut heap = StaticHeap::<u64, 2048>::new();
  b.iter(|| {
    for item in &vec {
      heap.push(*item);
    }
    heap.clear();
  });
}

#[bench]
fn staticheap_push_random_u64_4096(b: &mut Bencher) {
  let mut rng = Rand64::new(
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_nanos(),
  );
  let vec = StaticVec::<u64, 4096>::filled_with(|| rng.rand_range(1..5121));
  let mut heap = StaticHeap::<u64, 4096>::new();
  b.iter(|| {
    for item in &vec {
      heap.push(*item);
    }
    heap.clear();
  });
}

#[bench]
fn staticheap_push_random_u64_8192(b: &mut Bencher) {
  let mut rng = Rand64::new(
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_nanos(),
  );
  let vec = StaticVec::<u64, 8192>::filled_with(|| rng.rand_range(1..10241));
  let mut heap = StaticHeap::<u64, 8192>::new();
  b.iter(|| {
    for item in &vec {
      heap.push(*item);
    }
    heap.clear();
  });
}
