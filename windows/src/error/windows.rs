// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
	fmt::{self, Display},
	mem::MaybeUninit,
};
use windows::{
	core::PWSTR,
	Win32::{
		Foundation::{GetLastError, WIN32_ERROR},
		System::{
			Diagnostics::Debug::{
				FormatMessageW, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
				FORMAT_MESSAGE_IGNORE_INSERTS,
			},
			Memory::LocalFree,
			SystemServices::{LANG_NEUTRAL, SUBLANG_DEFAULT},
		},
	},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WindowsError(WIN32_ERROR);

impl WindowsError {
	/// Returns the last error code.
	pub fn from_last_error() -> Self {
		Self(unsafe { GetLastError() })
	}

	/// Returns the last error code, or `None` if the last error code is `0`.
	pub fn try_from_last_error() -> Option<Self> {
		let last_error = unsafe { GetLastError() };
		if last_error.is_err() {
			Some(Self(last_error))
		} else {
			None
		}
	}
}

const fn make_lang_id(lang: u32, sublang: u32) -> u32 {
	lang | (sublang << 10)
}

impl std::error::Error for WindowsError {}

impl Display for WindowsError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.0.is_ok() {
			return write!(f, "OK (no error)");
		}
		// Create an uninitialized pointer.
		// Windows will later replace this with a pointer to our error message.
		let mut err_ptr = MaybeUninit::<*mut u16>::uninit();
		// Format our error message, using FormatMessageA.
		if unsafe {
			FormatMessageW(
				FORMAT_MESSAGE_ALLOCATE_BUFFER
					| FORMAT_MESSAGE_FROM_SYSTEM
					| FORMAT_MESSAGE_IGNORE_INSERTS,
				std::ptr::null(),
				self.0 .0,
				make_lang_id(LANG_NEUTRAL, SUBLANG_DEFAULT),
				PWSTR(err_ptr.as_mut_ptr() as _),
				0,
				std::ptr::null_mut(),
			)
		} == 0
		{
			return Err(fmt::Error);
		}
		let err_ptr = unsafe { err_ptr.assume_init() };
		// Ensure that the error message buffer is always freed, no matter what happens.
		// A defer will run when the function exits, or even if it panics!
		scopeguard::defer! { unsafe { LocalFree(err_ptr as isize); } };
		// Get the length of the u16 buffer.
		let mut len = 0;
		while unsafe { *err_ptr.add(len + 1) } != 0 {
			len += 1;
		}
		let u16_slice = unsafe { std::slice::from_raw_parts(err_ptr, len) };
		// Convert the u16 buffer to a String.
		let string = String::from_utf16(u16_slice)
			.expect("WINDOWS GAVE US AN INVALID ERROR MESSAGE, OH NO!");
		write!(f, "{}", string)
	}
}
