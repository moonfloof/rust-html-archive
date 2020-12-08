use chrono::NaiveDateTime;
use std::time::SystemTime;

/// Convert SystemTime into NaiveDateTime
pub fn st_to_ndt(time: SystemTime) -> NaiveDateTime {
	let st_duration = time.duration_since(std::time::UNIX_EPOCH).unwrap();
	NaiveDateTime::from_timestamp(
		st_duration.as_secs() as i64,
		st_duration.subsec_nanos(),
	)
}
