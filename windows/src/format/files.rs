// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
	error::{Error, Result, WindowsError},
	lock::LockedPtr,
};
use std::path::PathBuf;
use windows::Win32::{
	Foundation::HANDLE,
	UI::Shell::{DragQueryFileW, HDROP},
};
use wtf8::Wtf8Buf;

pub fn get(handle: HANDLE) -> Result<Vec<PathBuf>> {
	let locked_hdrop = unsafe { LockedPtr::<()>::new(handle) }?;
	DropHandle::from(HDROP(locked_hdrop.as_ptr() as isize)).get_files()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct DropHandle(HDROP);

impl DropHandle {
	pub fn get_files(&self) -> Result<Vec<PathBuf>> {
		// Figure out how many files are in this HDROP
		let file_count = unsafe { DragQueryFileW(self.0, u32::MAX, &mut []) } as usize;
		if file_count == 0 {
			return Err(Error::PathCount(WindowsError::from_last_error()));
		}
		// Allocate a Vec of PathBufs big enough to handle all those files
		let mut out = Vec::with_capacity(file_count);
		// Iterate through all the files
		for idx in 0..file_count {
			// Figure out how big our buffer needs to be to fit this file path
			let needed_len = unsafe { DragQueryFileW(self.0, idx as u32, &mut []) } as usize;
			if needed_len == 0 {
				return Err(Error::PathLength {
					idx,
					err: WindowsError::from_last_error(),
				});
			}
			// Allocate the buffer where we'll store the file path
			let mut buf = vec![0_u16; needed_len + 1];
			// Get the file path, storing it in our buffer, and getting the total bytes written to our buffer.
			let written_len = unsafe { DragQueryFileW(self.0, idx as u32, &mut buf) } as usize;
			if written_len == 0 {
				return Err(Error::FilePath {
					idx,
					err: WindowsError::from_last_error(),
				});
			}
			// Truncate any unwritten bytes off our buffer.
			buf.truncate(written_len);
			// Convert our buffer to a CString, then to a String, and then into a PathBuf. Should take zero allocations.
			let path = PathBuf::from(Wtf8Buf::from_ill_formed_utf16(&buf).into_string_lossy());
			// Add our path to the output Vec.
			out.push(path);
		}
		Ok(out)
	}
}

impl From<HDROP> for DropHandle {
	fn from(handle: HDROP) -> Self {
		DropHandle(handle)
	}
}
