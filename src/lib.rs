#![feature(core_intrinsics)]
#![feature(const_fn)]
#![feature(const_generics)]
#![feature(maybe_uninit_ref)]
#![feature(maybe_uninit_extra)]

use crate::utils::*;
use std::cmp::{Ord, PartialEq};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Bound::Excluded, Bound::Included, Bound::Unbounded, Index, IndexMut, RangeBounds};
use std::ptr;
mod utils;

///A [Vec](std::vec::Vec)-like struct (directly API-compatible where it can be
///at least as far as function signatures go) implemented with
///const generics around a static array of fixed `N` capacity.
pub struct StaticVec<T, const N: usize> {
  data: [MaybeUninit<T>; N],
  length: usize,
}

///Similar to std's [Iter](std::slice::IterMut), but specifically implemented with StaticVecs in mind.
pub struct StaticVecIterConst<'a, T: 'a> {
  start: *const T,
  end: *const T,
  marker: PhantomData<&'a T>,
}

///Similar to std's [IterMut](std::slice::IterMut), but specifically implemented with StaticVecs in mind.
pub struct StaticVecIterMut<'a, T: 'a> {
  start: *mut T,
  end: *mut T,
  marker: PhantomData<&'a mut T>,
}

impl<T, const N: usize> StaticVec<T, { N }> {
  ///Returns a new StaticVec instance.
  #[inline(always)]
  pub fn new() -> Self {
    unsafe {
      Self {
        //Sound because data is an array of MaybeUninit<T>, not an array of T.
        data: MaybeUninit::uninit().assume_init(),
        length: 0,
      }
    }
  }

  ///Returns a new StaticVec instance filled with the contents, if any, of a slice.
  ///If the slice has a length greater than the StaticVec's capacity,
  ///any contents after that point are ignored.
  ///Locally requires that `T` implements [Copy](std::marker::Copy) to avoid soundness issues.
  #[inline]
  pub fn new_from_slice(values: &[T]) -> Self
  where T: Copy {
    unsafe {
      let mut _data: [MaybeUninit<T>; N] = MaybeUninit::uninit().assume_init();
      let fill_length = std::cmp::min(values.len(), N);
      values
        .as_ptr()
        .copy_to_nonoverlapping(_data.as_mut_ptr() as *mut T, fill_length);
      Self {
        data: _data,
        length: fill_length,
      }
    }
  }

  ///Returns a new StaticVec instance filled with the return value of
  ///an initializer function.
  ///`len()` will return the same as `capacity()` for the newly created StaticVec.
  #[inline]
  pub fn filled_with(initializer: fn() -> T) -> Self {
    unsafe {
      let mut _data: [MaybeUninit<T>; N] = MaybeUninit::uninit().assume_init();
      for val in _data.iter_mut() {
        val.write(initializer());
      }
      Self {
        data: _data,
        length: N,
      }
    }
  }

  ///Returns the current length of the StaticVec.
  ///Just as for a normal [Vec](std::vec::Vec), this means the number of elements that
  ///have been added to it with `push`, `insert`, e.t.c. except in the case
  ///that it has been set directly with the unsafe `set_len` function.
  #[inline(always)]
  pub fn len(&self) -> usize {
    self.length
  }

  ///Returns the total capacity of the StaticVec.
  ///This is always equivalent to the generic `N` parameter it was declared with,
  ///which determines the fixed size of the static backing array.
  #[inline(always)]
  pub const fn capacity(&self) -> usize {
    N
  }

  ///Directly sets the `length` field of the StaticVec to `new_len`. Useful if you intend
  ///to write to it solely element-wise, but marked unsafe due to how it creates the potential for reading
  ///from unitialized memory later on.
  #[inline(always)]
  pub unsafe fn set_len(&mut self, new_len: usize) {
    self.length = new_len;
  }

  ///Returns true if the current length of the StaticVec is 0.
  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    self.length == 0
  }

  ///Returns true if the current length of the StaticVec is greater than 0.
  #[inline(always)]
  pub fn is_not_empty(&self) -> bool {
    self.length > 0
  }

  ///Returns true if the current length of the StaticVec is equal to its capacity.
  #[inline(always)]
  pub fn is_full(&self) -> bool {
    self.length == N
  }

  ///Returns true if the current length of the StaticVec is less than its capacity.
  #[inline(always)]
  pub fn is_not_full(&self) -> bool {
    self.length < N
  }

  ///Returns a constant pointer to the first element of the StaticVec's internal array.
  #[inline(always)]
  pub fn as_ptr(&self) -> *const T {
    self.data.as_ptr() as *const T
  }

  ///Returns a mutable pointer to the first element of the StaticVec's internal array.
  #[inline(always)]
  pub fn as_mut_ptr(&mut self) -> *mut T {
    self.data.as_mut_ptr() as *mut T
  }

  ///Returns a constant reference to a slice of the StaticVec's inhabited area.
  #[inline(always)]
  pub fn as_slice(&self) -> &[T] {
    unsafe { &*(self.data.get_unchecked(0..self.length) as *const [MaybeUninit<T>] as *const [T]) }
  }

  ///Returns a mutable reference to a slice of the StaticVec's inhabited area.
  #[inline(always)]
  pub fn as_mut_slice(&mut self) -> &mut [T] {
    unsafe {
      &mut *(self.data.get_unchecked_mut(0..self.length) as *mut [MaybeUninit<T>] as *mut [T])
    }
  }

  ///Asserts that the current length of the StaticVec is less than `N`,
  ///and if so appends a value to the end of it.
  #[inline(always)]
  pub fn push(&mut self, value: T) {
    assert!(self.length < N, "No space left!");
    unsafe { self.data.get_unchecked_mut(self.length).write(value) };
    self.length += 1;
  }

  ///Removes the value at the last position of the StaticVec and returns it in `Some` if
  ///the StaticVec has a current length greater than 0, and returns `None` otherwise.
  #[inline(always)]
  pub fn pop(&mut self) -> Option<T> {
    if self.length == 0 {
      None
    } else {
      self.length -= 1;
      unsafe { Some(self.data.get_unchecked(self.length).read()) }
    }
  }

  ///Appends a value to the end of the StaticVec without asserting that
  ///its current length is less than `N`.
  #[inline(always)]
  pub unsafe fn push_unchecked(&mut self, value: T) {
    self.data.get_unchecked_mut(self.length).write(value);
    self.length += 1;
  }

  ///Pops a value from the end of the StaticVec and returns it directly without asserting that
  ///the StaticVec's current length is greater than 0.
  #[inline(always)]
  pub unsafe fn pop_unchecked(&mut self) -> T {
    self.length -= 1;
    self.data.get_unchecked(self.length).read()
  }

  ///Asserts that `index` is less than the current length of the StaticVec,
  ///and if so removes the value at that position and returns it. Any values
  ///that exist in later positions are shifted to the left.
  #[inline]
  pub fn remove(&mut self, index: usize) -> T {
    assert!(index < self.length, "Out of range!");
    unsafe {
      let p = self.as_mut_ptr().add(index);
      let res = p.read();
      p.offset(1).copy_to(p, self.length - index - 1);
      self.length -= 1;
      res
    }
  }

  ///Removes the first instance of `item` from the StaticVec if the item exists.
  #[inline(always)]
  pub fn remove_item(&mut self, item: &T) -> Option<T>
  where T: PartialEq {
    //Adapted this from normal Vec's implementation.
    if let Some(pos) = self.iter().position(|x| *x == *item) {
      Some(self.remove(pos))
    } else {
      None
    }
  }

  ///Asserts that `T`he current length of the StaticVec is less than `N` and that
  ///`index` is less than the length, and if so inserts `value` at that position.
  ///Any values that exist in later positions are shifted to the right.
  #[inline]
  pub fn insert(&mut self, index: usize, value: T) {
    assert!(
      self.length < N && index <= self.length,
      "Either you're out of range or there's no space left!"
    );
    unsafe {
      let p = self.as_mut_ptr().add(index);
      p.copy_to(p.offset(1), self.length - index);
      p.write(value);
      self.length += 1;
    }
  }

  ///Removes all contents from the StaticVec and sets its length back to 0.
  #[inline(always)]
  pub fn clear(&mut self) {
    unsafe {
      ptr::drop_in_place(self.as_mut_slice());
    }
    self.length = 0;
  }

  ///Performs an stable in-place sort of the StaticVec's inhabited area.
  ///Locally requires that `T` implements [Ord](std::cmp::Ord) to make the sorting possible.
  #[inline(always)]
  pub fn sort(&mut self)
  where T: Ord {
    self.as_mut_slice().sort();
  }

  ///Performs an unstable in-place sort of the StaticVec's inhabited area.
  ///Locally requires that `T` implements [Ord](std::cmp::Ord) to make the sorting possible.
  #[inline(always)]
  pub fn sort_unstable(&mut self)
  where T: Ord {
    self.as_mut_slice().sort_unstable();
  }

  ///Reverses the contents of the StaticVec's inhabited area in-place.
  #[inline(always)]
  pub fn reverse(&mut self) {
    self.as_mut_slice().reverse();
  }

  ///Returns a separate, stable-sorted StaticVec of the contents of the
  ///StaticVec's inhabited area without modifying the original data.
  ///Locally requires that `T` implements [Copy](std::marker::Copy) to avoid soundness issues,
  ///and [Ord](std::cmp::Ord) to make the sorting possible.
  #[inline]
  pub fn sorted(&mut self) -> Self
  where T: Copy + Ord {
    unsafe {
      let mut res = Self::new();
      res.length = self.length;
      self
        .as_ptr()
        .copy_to_nonoverlapping(res.as_mut_ptr(), self.length);
      res.sort();
      res
    }
  }

  ///Returns a separate, unstable-sorted StaticVec of the contents of the
  ///StaticVec's inhabited area without modifying the original data.
  ///Locally requires that `T` implements [Copy](std::marker::Copy) to avoid soundness issues,
  ///and [Ord](std::cmp::Ord) to make the sorting possible.
  #[inline]
  pub fn sorted_unstable(&mut self) -> Self
  where T: Copy + Ord {
    unsafe {
      let mut res = Self::new();
      res.length = self.length;
      self
        .as_ptr()
        .copy_to_nonoverlapping(res.as_mut_ptr(), self.length);
      res.sort_unstable();
      res
    }
  }

  ///Returns a separate, reversed StaticVec of the contents of the StaticVec's
  ///inhabited area without modifying the original data.
  ///Locally requires that `T` implements [Copy](std::marker::Copy) to avoid soundness issues.
  #[inline(always)]
  pub fn reversed(&mut self) -> Self
  where T: Copy {
    let mut res = Self::new();
    res.length = self.length;
    unsafe {
      reverse_copy(
        self.as_ptr(),
        self.as_ptr().add(self.length),
        res.as_mut_ptr(),
      );
    }
    res
  }

  ///Copies and appends all elements in a slice to the StaticVec.
  ///Unlike the implementation of this function for [Vec](std::vec::Vec), no iterator is used,
  ///just a single pointer-copy call.
  ///Locally requires that `T` implements [Copy](std::marker::Copy) to avoid soundness issues.
  #[inline]
  pub fn extend_from_slice(&mut self, other: &[T])
  where T: Copy {
    let mut added_length = other.len();
    while self.length + added_length > N {
      added_length -= 1;
    }
    unsafe {
      other
        .as_ptr()
        .copy_to_nonoverlapping(self.as_mut_ptr().add(self.length), added_length);
    }
    self.length += added_length;
  }

  ///Removes the specified range of elements from the StaticVec and returns them in a new one.
  #[inline]
  pub fn drain<R>(&mut self, range: R) -> Self
  //No Copy bounds here because the original StaticVec gives up all access to the values in question.
  where R: RangeBounds<usize> {
    //Borrowed this part from normal Vec's implementation.
    let start = match range.start_bound() {
      Included(&idx) => idx,
      Excluded(&idx) => idx + 1,
      Unbounded => 0,
    };
    let end = match range.end_bound() {
      Included(&idx) => idx + 1,
      Excluded(&idx) => idx,
      Unbounded => self.length,
    };
    assert!(start <= end && end <= self.length, "Out of range!");
    let mut res = Self::new();
    res.length = end - start;
    unsafe {
      self
        .as_ptr()
        .add(start)
        .copy_to_nonoverlapping(res.as_mut_ptr(), res.length);
      self
        .as_ptr()
        .add(end)
        .copy_to(self.as_mut_ptr().add(start), self.length - end);
    }
    self.length -= res.length;
    res
  }

  ///Returns a `StaticVecIterConst` over the StaticVec's inhabited area.
  #[inline]
  pub fn iter<'a>(&'a self) -> StaticVecIterConst<'a, T> {
    unsafe {
      if self.length > 0 {
        StaticVecIterConst::<'a, T> {
          start: self.as_ptr(),
          end: self.as_ptr().add(self.length),
          marker: PhantomData,
        }
      } else {
        StaticVecIterConst::<'a, T> {
          start: self.as_ptr(),
          end: self.as_ptr(),
          marker: PhantomData,
        }
      }
    }
  }

  ///Returns a `StaticVecIterMut` over the StaticVec's inhabited area.
  #[inline]
  pub fn iter_mut<'a>(&'a mut self) -> StaticVecIterMut<'a, T> {
    unsafe {
      if self.length > 0 {
        StaticVecIterMut::<'a, T> {
          start: self.as_mut_ptr(),
          end: self.as_mut_ptr().add(self.length),
          marker: PhantomData,
        }
      } else {
        StaticVecIterMut::<'a, T> {
          start: self.as_mut_ptr(),
          end: self.as_mut_ptr(),
          marker: PhantomData,
        }
      }
    }
  }
}

impl<T, const N: usize> Drop for StaticVec<T, { N }> {
  ///Calls `clear` through the StaticVec before dropping it.
  #[inline(always)]
  fn drop(&mut self) {
    self.clear();
  }
}

impl<T, const N: usize> Index<usize> for StaticVec<T, { N }> {
  type Output = T;
  ///Asserts that `index` is less than the current length of the StaticVec,
  ///as if so returns the value at that position as a constant reference.
  #[inline(always)]
  fn index(&self, index: usize) -> &Self::Output {
    assert!(index < self.length, "Out of range!");
    unsafe { self.data.get_unchecked(index).get_ref() }
  }
}

impl<T, const N: usize> IndexMut<usize> for StaticVec<T, { N }> {
  ///Asserts that `index` is less than the current length of the StaticVec,
  ///as if so returns the value at that position as a mutable reference.
  #[inline(always)]
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    assert!(index < self.length, "Out of range!");
    unsafe { self.data.get_unchecked_mut(index).get_mut() }
  }
}

impl<'a, T: 'a, const N: usize> IntoIterator for &'a StaticVec<T, { N }> {
  type IntoIter = StaticVecIterConst<'a, T>;
  type Item = <Self::IntoIter as Iterator>::Item;
  ///Returns a `StaticVecIterConst` over the StaticVec's inhabited area.
  #[inline(always)]
  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

impl<'a, T: 'a, const N: usize> IntoIterator for &'a mut StaticVec<T, { N }> {
  type IntoIter = StaticVecIterMut<'a, T>;
  type Item = <Self::IntoIter as Iterator>::Item;
  ///Returns a `StaticVecIterMut` over the StaticVec's inhabited area.
  #[inline(always)]
  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}

impl<T, const N: usize> FromIterator<T> for StaticVec<T, { N }> {
  ///Creates a new StaticVec instance from the elements, if any, of `iter`.
  ///If it has a size greater than the StaticVec's capacity, any items after
  ///that point are ignored.
  #[inline]
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let mut res = Self::new();
    for value in iter {
      if res.is_not_full() {
        unsafe {
          res.push_unchecked(value);
        }
      } else {
        break;
      }
    }
    res
  }
}

impl<'a, T: 'a> Iterator for StaticVecIterConst<'a, T> {
  type Item = &'a T;
  ///Returns `Some(&*self.start)` if `start` is less than `end`,
  ///and `None` if it's not.
  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.start < self.end {
      unsafe {
        let res = Some(&*self.start);
        self.start = self.start.add(1);
        res
      }
    } else {
      None
    }
  }
  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = distance_between(self.end, self.start);
    (len, Some(len))
  }
}

impl<'a, T: 'a> DoubleEndedIterator for StaticVecIterConst<'a, T> {
  ///Returns `Some(&*self.end)` if `end` is greater than `start`,
  ///and `None` if it's not.
  #[inline(always)]
  fn next_back(&mut self) -> Option<Self::Item> {
    if self.end > self.start {
      unsafe {
        let res = Some(&*self.end);
        self.end = self.end.sub(1);
        res
      }
    } else {
      None
    }
  }
}

impl<'a, T: 'a> ExactSizeIterator for StaticVecIterConst<'a, T> {}

impl<'a, T: 'a> Iterator for StaticVecIterMut<'a, T> {
  type Item = &'a mut T;
  ///Returns `Some(&mut *self.start)` if `start` is less than `end`,
  ///and `None` if it's not.
  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.start < self.end {
      unsafe {
        let res = Some(&mut *self.start);
        self.start = self.start.add(1);
        res
      }
    } else {
      None
    }
  }
  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = distance_between(self.end, self.start);
    (len, Some(len))
  }
}

impl<'a, T: 'a> DoubleEndedIterator for StaticVecIterMut<'a, T> {
  ///Returns `Some(&mut *self.end)` if `end` is greater than `start`,
  ///and `None` if it's not.
  #[inline(always)]
  fn next_back(&mut self) -> Option<Self::Item> {
    if self.end > self.start {
      unsafe {
        let res = Some(&mut *self.end);
        self.end = self.end.sub(1);
        res
      }
    } else {
      None
    }
  }
}

impl<'a, T: 'a> ExactSizeIterator for StaticVecIterMut<'a, T> {}
