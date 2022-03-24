// SPDX-License-Identifier: MIT OR Apache-2.0

use clipee_windows::ClipboardHandle;

static UTF8_TEST: &str = include_str!("utf8.txt");

#[test]
pub fn round_trip_string() {
	let handle = ClipboardHandle::new().expect("failed to open clipboard");
	let result = handle.set_string(UTF8_TEST);
	assert!(
		result.is_ok(),
		"Failed to set string to clipboard: {}",
		result.unwrap_err()
	);
	let result = handle.string_unicode();
	assert!(
		result.is_ok(),
		"Failed to get string from clipboard: {}",
		result.unwrap_err()
	);
	let result = result
		.expect("string wasn't set in clipboard?")
		.expect("failed to get string from clipboard");
	assert_eq!(UTF8_TEST, result, "String didn't survive round-trip!");
}
