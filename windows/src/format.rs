// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod bitmap;
pub mod files;
pub mod string;

use windows::Win32::System::SystemServices::{
	CF_BITMAP, CF_DIB, CF_DIBV5, CF_HDROP, CF_TEXT, CF_UNICODETEXT, CLIPBOARD_FORMATS,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClipboardFormat {
	Text,
	Bitmap,
	BitmapInfo,
	BitmapV5,
	DropHandle,
	UnicodeText,
}

impl ClipboardFormat {
	pub fn try_from_u32(format: u32) -> Option<Self> {
		match CLIPBOARD_FORMATS(format) {
			CF_TEXT => Some(Self::Text),
			CF_BITMAP => Some(Self::Bitmap),
			CF_DIB => Some(Self::BitmapInfo),
			CF_DIBV5 => Some(Self::BitmapV5),
			CF_HDROP => Some(Self::DropHandle),
			CF_UNICODETEXT => Some(Self::UnicodeText),
			_ => None,
		}
	}
}

impl From<ClipboardFormat> for CLIPBOARD_FORMATS {
	fn from(format: ClipboardFormat) -> Self {
		match format {
			ClipboardFormat::Text => CF_TEXT,
			ClipboardFormat::Bitmap => CF_BITMAP,
			ClipboardFormat::BitmapInfo => CF_DIB,
			ClipboardFormat::BitmapV5 => CF_DIBV5,
			ClipboardFormat::DropHandle => CF_HDROP,
			ClipboardFormat::UnicodeText => CF_UNICODETEXT,
		}
	}
}

impl From<ClipboardFormat> for u32 {
	fn from(format: ClipboardFormat) -> Self {
		CLIPBOARD_FORMATS::from(format).0
	}
}
