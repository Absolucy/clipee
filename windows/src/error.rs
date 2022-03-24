// SPDX-License-Identifier: MIT OR Apache-2.0

mod windows;

pub use self::windows::WindowsError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
	#[error("A handle to the clipboard is already opened by this program")]
	ClipboardAlreadyOpen,
	#[error("Failed to allocate global object: {0}")]
	Allocation(WindowsError),
	#[error("Failed to lock global object: {0}")]
	Locking(WindowsError),
	#[error("Global object is invalid or discarded: {0}")]
	InvalidObject(WindowsError),
	#[error("Failed to open clipboard: {0}")]
	OpenClipboard(WindowsError),
	#[error("Failed to get clipboard data: {0}")]
	GetClipboard(WindowsError),
	#[error("Failed to set clipboard data: {0}")]
	SetClipboard(WindowsError),
	#[error("Failed to get pixels from bitmap: {0}")]
	ImageBits(WindowsError),
	#[error("A valid image could not be constructed from the clipboard data")]
	InvalidImage,
	#[error("Failed to count file paths in clipboard: {0}")]
	PathCount(WindowsError),
	#[error("Failed to get length of the path #{idx} in the clipboard: {err}")]
	PathLength { idx: usize, err: WindowsError },
	#[error("Failed to get file path #{idx} in the clipboard: {err}")]
	FilePath { idx: usize, err: WindowsError },
	#[error("Failed to decode string as UTF-8: {0}")]
	InvalidString(std::str::Utf8Error),
	#[error("Failed to create dummy window: {0}")]
	CreateWindow(WindowsError),
	#[error("Failed to set up listener on dummy window: {0}")]
	ListenWindow(WindowsError),
}
