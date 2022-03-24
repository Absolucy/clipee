// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
	error::{Error, Result, WindowsError},
	lock::LockedPtr,
};
use image::RgbImage;
use windows::Win32::{
	Foundation::{HANDLE, HWND},
	Graphics::Gdi::{GetDC, GetDIBits, BITMAPINFO, BITMAPV5HEADER, DIB_RGB_COLORS, HBITMAP},
};

pub fn get(
	hbitmap: HBITMAP,
	header_handle: HANDLE,
	bitmap_info: LockedPtr<BITMAPINFO>,
) -> Result<RgbImage> {
	let bitmap_v5_header = unsafe { LockedPtr::<BITMAPV5HEADER>::new(header_handle) }?;
	let bitmap_v5_header = unsafe { &*bitmap_v5_header.as_ptr() };
	// this is just weird.
	let should_flip = bitmap_v5_header.bV5Height.is_positive();
	// Get the size of the bitmap.
	let width = bitmap_v5_header.bV5Width;
	let height = bitmap_v5_header.bV5Height.abs() as u32;
	let bits_per_pixel = bitmap_v5_header.bV5BitCount;
	let bytes_per_pixel = bits_per_pixel / 8;
	let size = bitmap_v5_header.bV5SizeImage as usize;
	// Alright, create a Vec with all the pixels, and then copy them into the Vec.
	let mut raw = vec![0_u8; size];
	let gdc = unsafe { GetDC(HWND::default()) };
	if unsafe {
		GetDIBits(
			gdc,
			hbitmap,
			0,
			height as u32,
			raw.as_mut_ptr() as *mut _,
			bitmap_info.as_mut_ptr(),
			DIB_RGB_COLORS,
		)
	} == 0
	{
		return Err(Error::ImageBits(WindowsError::from_last_error()));
	}
	// Lop off padding.
	let row_byte_length =
		((i32::from(bitmap_v5_header.bV5BitCount) * width + 31) / 32 * 4) as usize;
	let mut pixels = vec![0; height as usize * row_byte_length];
	for (raw_chunk, dst_chunk) in raw
		.chunks_exact(row_byte_length)
		.zip(pixels.chunks_exact_mut(width as usize * bytes_per_pixel as usize))
	{
		dst_chunk.copy_from_slice(&raw_chunk[..width as usize * bytes_per_pixel as usize]);
	}
	// Trim off excess length
	pixels.truncate(width as usize * height as usize * bytes_per_pixel as usize);
	let mut image =
		RgbImage::from_raw(width as u32, height as u32, pixels).ok_or(Error::InvalidImage)?;
	if should_flip {
		image::imageops::flip_vertical_in_place(&mut image);
	}
	Ok(image)
}
