use chrono::NaiveDateTime;
use std::path::PathBuf;
use std::time::SystemTime;

/// Convert SystemTime into NaiveDateTime
pub fn st_to_ndt(time: SystemTime) -> NaiveDateTime {
	let st_duration = time.duration_since(std::time::UNIX_EPOCH).unwrap();
	NaiveDateTime::from_timestamp_opt(
		st_duration.as_secs() as i64,
		st_duration.subsec_nanos(),
	)
	.expect("Invalid conversion from SystemTime to NaiveDateTime")
}

pub fn str_to_path(paths: &[&str]) -> Option<PathBuf> {
	if paths.len() == 0 {
		return None;
	}

	let mut path = PathBuf::new();

	for subpath in paths {
		path = path.join(subpath);
	}

	Some(path)
}
