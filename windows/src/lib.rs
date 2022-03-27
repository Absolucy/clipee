// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod error;
pub mod format;
pub(crate) mod lock;

use self::{
	error::{Error, Result, WindowsError},
	format::ClipboardFormat,
	lock::LockedPtr,
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{
	ops::Deref,
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use windows::{
	core::PCWSTR,
	Win32::{
		Foundation::{BOOL, HANDLE, HINSTANCE, HWND, POINT},
		Graphics::Gdi::{BITMAPINFO, HBITMAP},
		System::DataExchange::{
			CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
			IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
		},
		UI::{
			Shell::DROPFILES,
			WindowsAndMessaging::{
				CloseWindow, CreateWindowExW, HMENU, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
			},
		},
	},
};

static CLIPBOARD_HANDLE: OnceCell<Mutex<Weak<ClipboardHandleInner>>> = OnceCell::new();

#[derive(Clone)]
pub struct ClipboardHandle(Arc<ClipboardHandleInner>);

impl ClipboardHandle {
	pub fn new() -> Result<Self> {
		match CLIPBOARD_HANDLE.get() {
			Some(handle) => {
				let mut handle = handle.lock();
				match handle.upgrade() {
					Some(handle) => Ok(ClipboardHandle(handle)),
					None => {
						let new_handle = Arc::new(ClipboardHandleInner::new()?);
						*handle = Arc::downgrade(&new_handle);
						Ok(ClipboardHandle(new_handle))
					}
				}
			}
			None => {
				let handle = Arc::new(ClipboardHandleInner::new()?);
				CLIPBOARD_HANDLE
					.set(Mutex::new(Arc::downgrade(&handle)))
					.unwrap_or_else(|_| unreachable!());
				Ok(ClipboardHandle(handle))
			}
		}
	}
}

impl Deref for ClipboardHandle {
	type Target = ClipboardHandleInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsRef<ClipboardHandleInner> for ClipboardHandle {
	fn as_ref(&self) -> &ClipboardHandleInner {
		&self.0
	}
}

/// This is just a type that runs `OpenClipboard` on creation, and `CloseClipboard` on drop.
/// It's used to ensure that the clipboard is always closed when we're done with it.
#[derive(Debug)]
pub struct ClipboardHandleInner {
	window: HWND,
}

impl ClipboardHandleInner {
	fn new() -> Result<Self> {
		static STYLE: &[u16] = &[0x53, 0x74, 0x61, 0x74, 0x69, 0x63, 0x00]; // "Static" + \0
		let window = unsafe {
			CreateWindowExW(
				WINDOW_EX_STYLE::default(),
				PCWSTR(STYLE.as_ptr()),
				PCWSTR::default(),
				WINDOW_STYLE::default(),
				0,
				0,
				0,
				0,
				HWND_MESSAGE,
				HMENU::default(),
				HINSTANCE::default(),
				std::ptr::null(),
			)
		};
		if window.is_invalid() {
			return Err(Error::CreateWindow(WindowsError::from_last_error()));
		};
		if !unsafe { OpenClipboard(window) }.as_bool() {
			return Err(Error::OpenClipboard(WindowsError::from_last_error()));
		}
		Ok(Self { window })
	}

	pub fn set_string<StringType: ToString>(&self, string: StringType) -> Result<()> {
		self.set_string_impl(string.to_string())
	}

	fn set_string_impl(&self, string: String) -> Result<()> {
		// Encode string as UTF-16
		let utf16_bytes = string.encode_utf16().collect::<Vec<_>>();
		// Get number of 16-bit words in this string.
		let memory_len = utf16_bytes.len();
		// Allocate memory for this string (+ null terminator)
		let memory = LockedPtr::<u16>::alloc(memory_len + 1)?;
		// Convert that memory slice into a &mut [u16]
		let slice = unsafe { std::slice::from_raw_parts_mut(memory.as_mut_ptr(), memory_len + 1) };
		// Copy the UTF-16 bytes into the slice
		slice[..memory_len].copy_from_slice(&utf16_bytes[..]);
		// Set last byte to a null byte
		slice[memory_len] = 0;
		// Alright, time to set this string on the clipboard
		if unsafe { SetClipboardData(ClipboardFormat::UnicodeText.into(), memory.as_raw_handle()) }
			.is_invalid()
		{
			return Err(Error::SetClipboard(WindowsError::from_last_error()));
		}
		Ok(())
	}

	pub fn string(&self) -> Result<Option<String>> {
		if !Self::is_clipboard_format_available(ClipboardFormat::Text) {
			return Ok(None);
		}
		let handle = Self::get_clipboard_data(ClipboardFormat::Text)?;
		format::string::get(handle).map(Some)
	}

	pub fn string_unicode(&self) -> Result<Option<String>> {
		if !Self::is_clipboard_format_available(ClipboardFormat::UnicodeText) {
			return Ok(None);
		}
		let handle = Self::get_clipboard_data(ClipboardFormat::UnicodeText)?;
		format::string::get_unicode(handle).map(Some)
	}

	pub fn files(&self) -> Result<Option<Vec<PathBuf>>> {
		if !Self::is_clipboard_format_available(ClipboardFormat::DropHandle) {
			return Ok(None);
		}
		let handle = Self::get_clipboard_data(ClipboardFormat::DropHandle)?;
		format::files::get(handle).map(Some)
	}

	pub fn set_files<PathType: AsRef<Path>, PathList: AsRef<[PathType]>>(
		&self,
		paths: PathList,
	) -> Result<()> {
		self.set_files_impl(paths.as_ref())
	}

	fn set_files_impl<PathType: AsRef<Path>>(&self, paths: &[PathType]) -> Result<()> {
		let mut list = Vec::<u16>::new();
		for path in paths {
			let path = match path.as_ref().to_str() {
				Some(s) => s,
				None => continue,
			};
			// Reserve enough bytes for the UTF-16 version of this path, and the null byte.
			list.reserve((path.len() * 2) + std::mem::size_of::<u16>());
			path.encode_utf16().for_each(|byte| list.push(byte));
			// Null-terminate it.
			list.push(0);
		}
		// The list is double-null-terminated, so we need a SECOND null terminator here!
		list.push(0);
		self.set_files_impl_2(list)
	}

	fn set_files_impl_2(&self, paths_structure: Vec<u16>) -> Result<()> {
		let memory = LockedPtr::<u8>::alloc(
			std::mem::size_of::<DROPFILES>() + (paths_structure.len() * std::mem::size_of::<u16>()),
		)?;
		// microsoft never intended anyone to manually create this but fuck you I do what I want.
		let drop_files = DROPFILES {
			pFiles: std::mem::size_of::<DROPFILES>() as u32,
			pt: POINT::default(),
			fNC: BOOL(1),
			fWide: BOOL(1),
		};
		unsafe {
			// this is a fucking abomination
			*(memory.as_mut_ptr() as *mut DROPFILES) = drop_files;
			// so is this
			let u16_ptr = memory.as_mut_ptr().add(std::mem::size_of::<DROPFILES>()) as *mut u16;
			// ugh let's just copy the UTF-16 bytes over.
			std::ptr::copy_nonoverlapping(paths_structure.as_ptr(), u16_ptr, paths_structure.len());
			// actually set the clipboard data
			if SetClipboardData(ClipboardFormat::DropHandle.into(), memory.as_raw_handle())
				.is_invalid()
			{
				return Err(Error::SetClipboard(WindowsError::from_last_error()));
			}
		};
		Ok(())
	}

	pub fn image(&self) -> Result<Option<image::RgbImage>> {
		if !Self::is_clipboard_format_available(ClipboardFormat::Bitmap)
			|| !Self::is_clipboard_format_available(ClipboardFormat::BitmapInfo)
		{
			return Ok(None);
		}
		let hbitmap = Self::get_clipboard_data(ClipboardFormat::Bitmap).map(|h| HBITMAP(h.0))?;
		let bitmap_info = Self::get_clipboard_data(ClipboardFormat::BitmapInfo)
			.and_then(|handle| unsafe { LockedPtr::<BITMAPINFO>::new(handle) })?;
		format::bitmap::get(hbitmap, bitmap_info).map(Some)
	}

	pub fn empty(&self) -> Result<()> {
		if !unsafe { EmptyClipboard() }.as_bool() {
			return Err(Error::GetClipboard(WindowsError::from_last_error()));
		}
		Ok(())
	}

	pub fn available_formats(&self) -> Result<Vec<ClipboardFormat>> {
		let mut formats = Vec::<ClipboardFormat>::new();
		let mut last_format = 0;
		loop {
			last_format = unsafe { EnumClipboardFormats(last_format) };
			if last_format == 0 {
				match WindowsError::try_from_last_error() {
					Some(err) => return Err(Error::EnumClipboard(err)),
					None => break,
				}
			}
			if let Some(format) = ClipboardFormat::try_from_u32(last_format) {
				formats.push(format);
			}
		}
		Ok(formats)
	}

	fn is_clipboard_format_available(format: ClipboardFormat) -> bool {
		unsafe { IsClipboardFormatAvailable(format.into()) }.as_bool()
	}

	fn get_clipboard_data(format: ClipboardFormat) -> Result<HANDLE> {
		let handle = unsafe { GetClipboardData(format.into()) };
		if handle.is_invalid() {
			return Err(Error::GetClipboard(WindowsError::from_last_error()));
		}
		Ok(handle)
	}
}

impl Drop for ClipboardHandleInner {
	fn drop(&mut self) {
		unsafe {
			CloseClipboard();
			CloseWindow(self.window);
		}
	}
}
