// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
	error::{Error, Result},
	lock::LockedPtr,
};
use std::{ffi::CStr, os::raw::c_char};
use windows::Win32::Foundation::HANDLE;
use wtf8::Wtf8Buf;

pub fn get(handle: HANDLE) -> Result<String> {
	let locked_str = unsafe { LockedPtr::<c_char>::new(handle) }?;
	let c_str = unsafe { CStr::from_ptr(locked_str.as_mut_ptr()) };
	c_str
		.to_str()
		.map(ToOwned::to_owned)
		.map_err(Error::InvalidString)
}

pub fn get_unicode(handle: HANDLE) -> Result<String> {
	let locked_str = unsafe { LockedPtr::<u16>::new(handle) }?;
	let len = locked_str.size()? / std::mem::size_of::<u16>() - 1;
	let u16_str = unsafe { std::slice::from_raw_parts(locked_str.as_mut_ptr(), len) };
	Ok(Wtf8Buf::from_ill_formed_utf16(u16_str).into_string_lossy())
}
