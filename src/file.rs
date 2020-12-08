use crate::{env, util};
use chrono::{NaiveDate, NaiveDateTime};
use regex::Regex;
use std::fs::{read_to_string, DirEntry};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct File {
	pub path: Box<Path>,
	pub url: String,
	pub output_file: String,
	pub output_dir: String,

	pub title: String,
	pub slug: String,
	pub contents: String,

	pub datetime: NaiveDateTime,
	pub dateiso: String,
	pub datehuman: String,
	pub year: String,
	pub year_month: String,
}

impl File {
	fn slugify(title: &str) -> String {
		let mut slug = title
			.chars()
			.into_iter()
			.filter_map(|c| {
				if c.is_ascii_alphanumeric() || c == ' ' {
					Some(c)
				} else {
					None
				}
			})
			.collect::<String>();

		slug = slug.replace(" ", "-");
		slug = slug.trim_end_matches("-").to_string();
		slug = slug.trim_start_matches("-").to_string();
		slug = slug.to_lowercase();

		slug
	}

	fn determine_extension(name: &str) -> Option<String> {
		env::get_extensions().into_iter().fold(None, |acc, ext| {
			let new_ext = format!(".{}", ext);
			if name.ends_with(&new_ext) {
				Some(new_ext)
			} else {
				acc
			}
		})
	}

	fn determine_title(entry: &DirEntry, extension: &str) -> String {
		let mut name = String::from(entry.file_name().to_str().unwrap());

		// Remove extension
		name.truncate(name.len() - extension.len());

		// Remove date from start
		let re = Regex::new(r"^\d{4}-\d{2}-\d{2}\s(.*)").unwrap();
		let captures = re.captures(&name);
		if captures.is_some() {
			name = String::from(captures.unwrap().get(1).unwrap().as_str());
		}

		name
	}

	fn determine_contents(path: &Box<Path>, extension: &str) -> String {
		let mut contents = read_to_string(&path).unwrap();

		if extension == ".html" {
			return contents;
		}

		contents = contents.replace("\r", "");
		contents = contents.replace("\n\n", "</p><p>");
		contents = contents.replace("\n", "<br />");

		format!("<p>{}</p>", contents)
	}

	/// Determine a file's date.
	/// Prefer an ISO8601 date in the filename, but fallback to the system modified date.
	fn determine_datetime(entry: &DirEntry) -> NaiveDateTime {
		// Short Filename
		let name = entry.file_name();
		let name = name.to_str().unwrap();

		// Get file metadata
		let meta = std::fs::metadata(entry.path()).unwrap();
		let dt = meta.modified().unwrap();
		let modified = util::st_to_ndt(dt);

		// Regex
		let re = Regex::new(r"^(\d{4}-\d{2}-\d{2})\s.*").unwrap();
		let captures = re.captures(name);
		if captures.is_none() {
			NaiveDateTime::new(modified.date(), modified.time())
		} else {
			let date = captures.unwrap().get(1).unwrap().as_str();
			let nd = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
			NaiveDateTime::new(nd, modified.time())
		}
	}

	fn determine_absolute_url(title: &str, datetime: &NaiveDateTime) -> String {
		format!(
			"/{}/{}/{}.html",
			datetime.format("%Y"),
			datetime.format("%m"),
			File::slugify(title)
		)
	}

	pub fn new(entry: DirEntry) -> Option<File> {
		let path = entry.path().into_boxed_path();
		let env_output_dir = env::get_output_dir();

		// Filename
		let filename = String::from(entry.file_name().to_str().unwrap());
		if filename.starts_with("DRAFT") {
			return None;
		}

		// Extension
		let extension = File::determine_extension(&filename);
		if extension.is_none() {
			return None;
		}

		// Get all fields for File
		let extension = extension.unwrap();
		let title = File::determine_title(&entry, &extension);
		let contents = File::determine_contents(&path, &extension);

		// Format dates
		let datetime = File::determine_datetime(&entry);
		let dateiso = datetime.format("%Y-%m-%d").to_string();
		let datehuman = datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();
		let year = datetime.format("%Y").to_string();
		let month = datetime.format("%m").to_string();
		let year_month = datetime.format("%Y/%m").to_string();

		// Remaining fields
		let url = File::determine_absolute_url(&title, &datetime);
		let slug = File::slugify(&title);
		let output_dir = format!("{}\\{}\\{}", &env_output_dir, &year, &month);
		let output_file = format!("{}\\{}.html", &output_dir, &slug);

		Some(File {
			path,
			url,
			output_file,
			output_dir,

			title,
			slug,
			contents,
			datetime,
			dateiso,
			datehuman,
			year,
			year_month,
		})
	}
}
