use crate::{env, util};
use chrono::{NaiveDate, NaiveDateTime};
use regex::Regex;
use std::fs::{read_to_string, DirEntry};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct File {
	pub path: Box<Path>,
	pub url: String,
	pub output_file: PathBuf,
	pub output_dir: PathBuf,

	pub title: String,
	pub slug: String,
	pub raw_contents: String,
	pub contents: String,

	pub datetime: NaiveDateTime,
	pub dateiso: String,
	pub datehuman: String,
	pub year: String,
	pub year_month: String,
}

impl File {
	/// Convert an article title into an ASCII, URL-friendly string
	fn slugify(title: &str, dateiso: &str) -> String {
		if title == "" {
			return dateiso.to_owned();
		}

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

	/// Figure out the extension of a given file. If not supported,
	/// return None.
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

	/// Remove ISO date from filenames to generate article title
	fn determine_title(entry: &DirEntry, extension: &str) -> String {
		let mut name = String::from(entry.file_name().to_str().unwrap());

		// Remove extension
		name.truncate(name.len() - extension.len());

		// Remove date from start
		let re = Regex::new(r"^(\d{4}-\d{2}-\d{2})?(.*)").unwrap();
		let captures = re.captures(&name);
		if let Some(captures) = captures {
			if let Some(title) = captures.get(2) {
				return String::from(title.as_str());
			}
		}

		String::from("")
	}

	/// Simple conversion of text files into HTML (adds paragraphs and line breaks)
	fn determine_contents(raw_contents: &str, extension: &str) -> String {
		let mut contents = String::from(raw_contents);

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
		let re = Regex::new(r"^(\d{4}-\d{2}-\d{2}).*").unwrap();
		let captures = re.captures(name);
		if captures.is_none() {
			NaiveDateTime::new(modified.date(), modified.time())
		} else {
			let date = captures.unwrap().get(1).unwrap().as_str();
			let nd = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
			NaiveDateTime::new(nd, modified.time())
		}
	}

	fn determine_absolute_url(slug: &str, datetime: &NaiveDateTime) -> String {
		format!(
			"/{}/{}/{}.html",
			datetime.format("%Y"),
			datetime.format("%m"),
			slug
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
		let mut title = File::determine_title(&entry, &extension);
		title = title.trim().to_owned();
		let raw_contents = read_to_string(&path).unwrap();
		let contents = File::determine_contents(&raw_contents, &extension);

		// Format dates
		let datetime = File::determine_datetime(&entry);
		let dateiso = datetime.format("%Y-%m-%d").to_string();
		let datehuman = datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();
		let year = datetime.format("%Y").to_string();
		let month = datetime.format("%m").to_string();
		let year_month = datetime.format("%Y/%m").to_string();

		// Remaining fields
		let slug = File::slugify(&title, &dateiso);
		let url = File::determine_absolute_url(&slug, &datetime);
		let rel_filename = format!("{}.html", slug);
		let output_dir =
			util::str_to_path(&[&env_output_dir, &year, &month]).unwrap();
		let output_dir_str = output_dir.to_str().unwrap();
		let output_file =
			util::str_to_path(&[output_dir_str, &rel_filename]).unwrap();

		Some(File {
			path,
			url,
			output_file,
			output_dir,

			title,
			slug,
			contents,
			raw_contents,

			datetime,
			dateiso,
			datehuman,
			year,
			year_month,
		})
	}
}
