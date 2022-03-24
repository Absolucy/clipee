// SPDX-License-Identifier: MIT OR Apache-2.0

use clipee_windows::ClipboardHandle;

#[test]
pub fn round_trip_files() {
	use std::path::PathBuf;

	let handle = ClipboardHandle::new().expect("failed to open clipboard");
	let files_list = vec![
		PathBuf::from("C:\\Users\\"),
		PathBuf::from("C:\\Users\\Clipboard\\"),
		PathBuf::from("C:\\Users\\Clipboard\\Desktop\\"),
		PathBuf::from("C:\\Users\\Clipboard\\Desktop\\test.txt"),
	];
	let result = handle.set_files(&files_list);
	assert!(
		result.is_ok(),
		"Failed to set files to clipboard: {}",
		result.unwrap_err()
	);
	let files = handle.files();
	assert!(
		files.is_ok(),
		"Failed to get files from clipboard: {}",
		files.unwrap_err()
	);
	let files = files
		.expect("files weren't set in clipboard")
		.expect("failed to get files from clipboard");
	assert_eq!(files_list, files, "File list didn't survive the round-trip");
}
