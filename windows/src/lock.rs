// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::{Error, Result, WindowsError};
use windows::Win32::{
	Foundation::HANDLE,
	System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
};

pub struct LockedPtr<T> {
	lock: isize,
	ptr: *mut T,
}

impl<T> LockedPtr<T> {
	pub unsafe fn new(handle: HANDLE) -> Result<Self> {
		let lock = handle.0;
		let ptr = GlobalLock(lock);
		if ptr.is_null() {
			return Err(Error::Locking(WindowsError::from_last_error()));
		}
		let alloc_size = GlobalSize(lock);
		if alloc_size == 0 {
			return Err(Error::InvalidObject(WindowsError::from_last_error()));
		}
		// Sanity check
		if std::mem::size_of::<T>() > alloc_size {
			Self::panic_if_invalid_size(alloc_size);
		}
		Ok(LockedPtr {
			lock,
			ptr: ptr as _,
		})
	}

	pub fn alloc(amt: usize) -> Result<Self> {
		let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of::<T>() * amt) };
		if handle == 0 {
			return Err(Error::Allocation(WindowsError::from_last_error()));
		}
		unsafe { Self::new(HANDLE(handle)) }
	}

	/// Returns the size of the allocation, in bytes.
	pub fn size(&self) -> Result<usize> {
		let alloc_size = unsafe { GlobalSize(self.lock) };
		if alloc_size == 0 {
			return Err(Error::InvalidObject(WindowsError::from_last_error()));
		}
		Ok(alloc_size)
	}

	// Seperate function so we can have the #[cold] attribute to tell LLVM "ay this will probably never run"
	#[cold]
	fn panic_if_invalid_size(alloc_size: usize) {
		panic!(
			"Attempted to create a LockedPtr<{type_name}>, however the allocation size is {alloc_size} bytes, which cannot fit the {type_size} bytes of {type_name}",
			type_name = std::any::type_name::<T>(),
			type_size = std::mem::size_of::<T>()
		);
	}

	pub fn as_ptr(&self) -> *const T {
		self.ptr as _
	}

	pub fn as_mut_ptr(&self) -> *mut T {
		self.ptr
	}

	pub fn as_raw_handle(&self) -> HANDLE {
		HANDLE(self.lock)
	}
}

impl<T> Drop for LockedPtr<T> {
	fn drop(&mut self) {
		unsafe { GlobalUnlock(self.lock) };
	}
}
