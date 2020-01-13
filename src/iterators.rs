use crate::utils::{slice_from_raw_parts, slice_from_raw_parts_mut};
use crate::StaticVec;
use core::fmt::{self, Debug, Formatter};
use core::iter::{FusedIterator, TrustedLen};
use core::marker::{Send, Sync};
use core::mem::MaybeUninit;
use core::ptr;
use core::slice;

#[cfg(feature = "std")]
use alloc::string::String;

#[cfg(feature = "std")]
use alloc::format;

/// Similar to [`Iter`](core::slice::Iter), but specifically implemented with StaticVecs in mind.
#[derive(Clone)]
pub struct StaticVecIterConst<'a, T: 'a, const N: usize> {
  pub(crate) iter: slice::Iter<'a, T>,
}

/// Similar to [`IterMut`](core::slice::IterMut), but specifically implemented with StaticVecs in
/// mind.
pub struct StaticVecIterMut<'a, T: 'a, const N: usize> {
  pub(crate) iter: slice::IterMut<'a, T>,
}

/// A "consuming" iterator that reads each element out of
/// a source StaticVec by value.
pub struct StaticVecIntoIter<T, const N: usize> {
  pub(crate) start: usize,
  pub(crate) end: usize,
  pub(crate) data: MaybeUninit<[T; N]>,
}

/// A "draining" iterator, analogous to [`vec::Drain`](alloc::vec::Drain).
/// Instances of [`StaticVecDrain`](crate::iterators::StaticVecDrain) are created
/// by the [`drain_iter`](crate::StaticVec::drain_iter) method on [`StaticVec`](crate::StaticVec),
/// as while the [`drain`](crate::StaticVec::drain) method does have a similar purpose, it works by
/// immediately returning a new [`StaticVec`](crate::StaticVec) as opposed to an iterator.
pub struct StaticVecDrain<'a, T: 'a, const N: usize> {
  pub(crate) start: usize,
  pub(crate) length: usize,
  pub(crate) iter: StaticVecIterConst<'a, T, N>,
  pub(crate) vec: *mut StaticVec<T, N>,
}

impl<'a, T: 'a, const N: usize> StaticVecIterConst<'a, T, N> {
  /// Returns a string displaying the current values of the
  /// iterator's `start` and `end` elements on two separate lines.
  /// Locally requires that `T` implements [Debug](core::fmt::Debug)
  /// to make it possible to pretty-print the elements.
  #[cfg(feature = "std")]
  #[doc(cfg(feature = "std"))]
  #[inline(always)]
  pub fn bounds_to_string(&self) -> String
  where T: Debug {
    let slice = self.as_slice();
    match (slice.first(), slice.last()) {
      (Some(first), Some(last)) => format!(
        "Current value of element at `start`: {:?}\nCurrent value of element at `end`: {:?}",
        first, last
      ),
      _ => String::from("Empty iterator!"),
    }
  }

  /// Returns an immutable slice consisting of the elements in the range between the iterator's
  /// `start` and `end` pointers.
  #[inline(always)]
  pub fn as_slice(&self) -> &'a [T] {
    self.iter.as_slice()
  }
}

impl<'a, T: 'a, const N: usize> Iterator for StaticVecIterConst<'a, T, N> {
  type Item = &'a T;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next()
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
  }
}

impl<'a, T: 'a, const N: usize> DoubleEndedIterator for StaticVecIterConst<'a, T, N> {
  #[inline(always)]
  fn next_back(&mut self) -> Option<Self::Item> {
    self.iter.next_back()
  }
}

impl<'a, T: 'a, const N: usize> ExactSizeIterator for StaticVecIterConst<'a, T, N> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.iter.len()
  }

  #[inline(always)]
  fn is_empty(&self) -> bool {
    self.iter.is_empty()
  }
}

impl<'a, T: 'a, const N: usize> FusedIterator for StaticVecIterConst<'a, T, N> {}
unsafe impl<'a, T: 'a, const N: usize> TrustedLen for StaticVecIterConst<'a, T, N> {}

impl<'a, T: 'a + Debug, const N: usize> Debug for StaticVecIterConst<'a, T, N> {
  #[inline(always)]
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_tuple("StaticVecIterConst")
      .field(&self.as_slice())
      .finish()
  }
}

impl<'a, T: 'a, const N: usize> StaticVecIterMut<'a, T, N> {
  /// Returns a string displaying the current values of the
  /// iterator's `start` and `end` elements on two separate lines.
  /// Locally requires that `T` implements [Debug](core::fmt::Debug)
  /// to make it possible to pretty-print the elements.
  #[cfg(feature = "std")]
  #[doc(cfg(feature = "std"))]
  #[inline(always)]
  pub fn bounds_to_string(&self) -> String
  where T: Debug {
    let slice = self.as_slice();
    match (slice.first(), slice.last()) {
      (Some(first), Some(last)) => format!(
        "Current value of element at `start`: {:?}\nCurrent value of element at `end`: {:?}",
        first, last
      ),
      _ => String::from("Empty iterator!"),
    }
  }

  /// Returns an immutable slice consisting of the elements in the range between the iterator's
  /// `start` and `end` pointers. Though this is a mutable iterator, the slice cannot be mutable
  /// as it would lead to aliasing issues.
  #[inline(always)]
  pub fn as_slice(&self) -> &[T] {
    self.iter.as_slice()
  }
}

impl<'a, T: 'a, const N: usize> Iterator for StaticVecIterMut<'a, T, N> {
  type Item = &'a mut T;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    self.iter.next()
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
  }
}

impl<'a, T: 'a, const N: usize> DoubleEndedIterator for StaticVecIterMut<'a, T, N> {
  #[inline(always)]
  fn next_back(&mut self) -> Option<Self::Item> {
    self.iter.next_back()
  }
}

impl<'a, T: 'a, const N: usize> ExactSizeIterator for StaticVecIterMut<'a, T, N> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.iter.len()
  }

  #[inline(always)]
  fn is_empty(&self) -> bool {
    self.iter.is_empty()
  }
}

impl<'a, T: 'a, const N: usize> FusedIterator for StaticVecIterMut<'a, T, N> {}
unsafe impl<'a, T: 'a, const N: usize> TrustedLen for StaticVecIterMut<'a, T, N> {}

impl<'a, T: 'a + Debug, const N: usize> Debug for StaticVecIterMut<'a, T, N> {
  #[inline(always)]
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_tuple("StaticVecIterMut")
      .field(&self.as_slice())
      .finish()
  }
}

impl<T, const N: usize> StaticVecIntoIter<T, N> {
  /// Returns a string displaying the current values of the
  /// iterator's `start` and `end` elements on two separate lines.
  /// Locally requires that `T` implements [Debug](core::fmt::Debug)
  /// to make it possible to pretty-print the elements.
  #[cfg(feature = "std")]
  #[doc(cfg(feature = "std"))]
  #[inline(always)]
  pub fn bounds_to_string(&self) -> String
  where T: Debug {
    match self.len() {
      0 => String::from("Empty iterator!"),
      _ => unsafe {
        // Safety: `start` and `end` are never out of bounds.
        format!(
          "Current value of element at `start`: {:?}\nCurrent value of element at `end`: {:?}",
          &*StaticVec::first_ptr(&self.data).add(self.start),
          &*StaticVec::first_ptr(&self.data).add(self.end - 1)
        )
      },
    }
  }

  /// Returns an immutable slice consisting of the elements in the range between the iterator's
  /// `start` and `end` indices.
  #[inline(always)]
  pub fn as_slice(&self) -> &[T] {
    // Safety: `start` is never null. This function will "at worst" return an empty slice.
    slice_from_raw_parts(
      unsafe { StaticVec::first_ptr(&self.data).add(self.start) },
      self.len(),
    )
  }

  /// Returns a mutable slice consisting of the elements in the range between the iterator's
  /// `start` and `end` indices.
  #[inline(always)]
  pub fn as_mut_slice(&mut self) -> &mut [T] {
    // Safety: `start` is never null. This function will "at worst" return an empty slice.
    slice_from_raw_parts_mut(
      unsafe { StaticVec::first_ptr_mut(&mut self.data).add(self.start) },
      self.len(),
    )
  }
}

impl<T, const N: usize> Iterator for StaticVecIntoIter<T, N> {
  type Item = T;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    match self.end - self.start {
      0 => None,
      _ => {
        let res = Some(unsafe { StaticVec::first_ptr(&self.data).add(self.start).read() });
        self.start += 1;
        res
      }
    }
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.end - self.start;
    (len, Some(len))
  }
}

impl<T, const N: usize> DoubleEndedIterator for StaticVecIntoIter<T, N> {
  #[inline(always)]
  fn next_back(&mut self) -> Option<Self::Item> {
    match self.end - self.start {
      0 => None,
      _ => {
        self.end -= 1;
        Some(unsafe { StaticVec::first_ptr(&self.data).add(self.end).read() })
      }
    }
  }
}

impl<T, const N: usize> ExactSizeIterator for StaticVecIntoIter<T, N> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.end - self.start
  }

  #[inline(always)]
  fn is_empty(&self) -> bool {
    self.end - self.start == 0
  }
}

impl<T, const N: usize> FusedIterator for StaticVecIntoIter<T, N> {}
unsafe impl<T, const N: usize> TrustedLen for StaticVecIntoIter<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for StaticVecIntoIter<T, N> {}
unsafe impl<T: Sync, const N: usize> Send for StaticVecIntoIter<T, N> {}

impl<T: Debug, const N: usize> Debug for StaticVecIntoIter<T, N> {
  #[inline(always)]
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_tuple("StaticVecIntoIter")
      .field(&self.as_slice())
      .finish()
  }
}

impl<T, const N: usize> Drop for StaticVecIntoIter<T, N> {
  #[inline(always)]
  fn drop(&mut self) {
    let item_count = self.end - self.start;
    match item_count {
      0 => (),
      _ => unsafe {
        ptr::drop_in_place(slice_from_raw_parts_mut(
          StaticVec::first_ptr_mut(&mut self.data).add(self.start),
          item_count,
        ))
      },
    }
  }
}

impl<'a, T: 'a, const N: usize> StaticVecDrain<'a, T, N> {
  /// Returns an immutable slice consisting of the current range of elements the iterator has a view
  /// over.
  #[inline(always)]
  pub fn as_slice(&self) -> &[T] {
    self.iter.as_slice()
  }
}

impl<'a, T: 'a, const N: usize> Iterator for StaticVecDrain<'a, T, N> {
  type Item = T;

  #[inline(always)]
  fn next(&mut self) -> Option<T> {
    self
      .iter
      .next()
      .map(|val| unsafe { (val as *const T).read() })
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
  }
}

impl<'a, T: 'a, const N: usize> DoubleEndedIterator for StaticVecDrain<'a, T, N> {
  #[inline(always)]
  fn next_back(&mut self) -> Option<T> {
    self
      .iter
      .next_back()
      .map(|val| unsafe { (val as *const T).read() })
  }
}

impl<'a, T: 'a, const N: usize> ExactSizeIterator for StaticVecDrain<'a, T, N> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.iter.len()
  }

  #[inline(always)]
  fn is_empty(&self) -> bool {
    self.iter.is_empty()
  }
}

impl<'a, T: 'a, const N: usize> FusedIterator for StaticVecDrain<'a, T, N> {}
unsafe impl<'a, T: 'a, const N: usize> TrustedLen for StaticVecDrain<'a, T, N> {}
unsafe impl<'a, T: 'a + Sync, const N: usize> Sync for StaticVecDrain<'a, T, N> {}
unsafe impl<'a, T: 'a + Sync, const N: usize> Send for StaticVecDrain<'a, T, N> {}

impl<'a, T: 'a + Debug, const N: usize> Debug for StaticVecDrain<'a, T, N> {
  #[inline(always)]
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_tuple("StaticVecDrain")
      .field(&self.iter.as_slice())
      .finish()
  }
}

impl<'a, T: 'a, const N: usize> Drop for StaticVecDrain<'a, T, N> {
  #[inline]
  fn drop(&mut self) {
    // Read out any remaining contents first.
    while let Some(_) = self.next() {}
    // Adjust the StaticVec that this StaticVecDrain was created from, if necessary.
    let total_length = self.length;
    if total_length > 0 {
      unsafe {
        let vec_ref = &mut *self.vec;
        let start = vec_ref.length;
        let tail = self.start;
        vec_ref
          .ptr_at_unchecked(tail)
          .copy_to(vec_ref.mut_ptr_at_unchecked(start), total_length);
        vec_ref.set_len(start + total_length);
      }
    }
  }
}
