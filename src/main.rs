use chrono::{NaiveDate, NaiveDateTime};
use dotenv::dotenv;
use fs::write;
use regex::Regex;
use std::fs::{self, read_to_string, DirEntry};
use std::path::Path;
use std::time::SystemTime;
use std::{collections::HashMap, env};

#[derive(Clone, Debug)]
struct File {
	path: Box<Path>,
	datetime: NaiveDateTime,
	title: String,
	contents: String,
	url: String,
}

fn get_all_dirs(path: &str) -> Vec<Box<Path>> {
	let default = String::from("public_archive");
	let dir_match = env::var("PUBLIC_DIR").unwrap_or(default);
	fs::read_dir(path).unwrap().fold(vec![], |mut acc, p| {
		let node = p.unwrap().path();
		if !node.is_dir() {
			return acc;
		}

		let name = node.to_str().unwrap();

		if name.ends_with(&dir_match) {
			acc.push(node.into_boxed_path());
			return acc;
		}

		let newfiles = get_all_dirs(name);
		vec![acc, newfiles].concat()
	})
}

fn get_file_title(entry: &DirEntry, extension: &str) -> String {
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

/// Convert SystemTime into NaiveDateTime
fn st_to_ndt(time: SystemTime) -> NaiveDateTime {
	let st_duration = time.duration_since(std::time::UNIX_EPOCH).unwrap();
	NaiveDateTime::from_timestamp(
		st_duration.as_secs() as i64,
		st_duration.subsec_nanos(),
	)
}

/// Determine a file's date.
/// Prefer an ISO8601 date in the filename, but fallback to the system modified date.
fn get_file_date(entry: &DirEntry) -> NaiveDateTime {
	// Short Filename
	let name = entry.file_name();
	let name = name.to_str().unwrap();

	// Get file metadata
	let meta = std::fs::metadata(entry.path()).unwrap();
	let dt = meta.modified().unwrap();
	let modified = st_to_ndt(dt);

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

fn format_contents(contents: &str, extension: &str) -> String {
	let mut contents = contents.to_owned();

	if extension == ".html" {
		return contents;
	}

	contents = contents.replace("\r", "");
	contents = contents.replace("\n\n", "</p><p>");
	contents = contents.replace("\n", "<br />");

	format!("<p>{}</p>", contents)
}

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

fn get_extensions() -> Vec<String> {
	let default = String::from("html,md,txt");
	let extensions = env::var("EXTENSIONS").unwrap_or(default);

	extensions.split(",").map(|s| String::from(s)).collect()
}

// Get a file
fn get_file_meta(entry: DirEntry) -> Option<File> {
	let extensions = get_extensions();
	let node = entry.path();
	let name = String::from(entry.file_name().to_str().unwrap());
	let path = node.into_boxed_path();

	if name.starts_with("DRAFT") {
		return None;
	}

	let extension = extensions.into_iter().fold(None, |acc, ext| {
		let new_ext = format!(".{}", ext);
		if name.ends_with(&new_ext) {
			Some(new_ext)
		} else {
			acc
		}
	});

	if extension.is_none() {
		return None;
	}

	let extension = extension.unwrap();
	let datetime = get_file_date(&entry);
	let title = get_file_title(&entry, &extension);
	let contents = format_contents(&read_to_string(&path).unwrap(), &extension);
	let url = format!(
		"/{}/{}/{}.html",
		datetime.format("%Y"),
		datetime.format("%m"),
		slugify(&title)
	);
	let file = File {
		path,
		datetime,
		title,
		contents,
		url,
	};
	Some(file)
}

/// Retrieve all files from one archive folder
fn get_files(path: Box<Path>) -> Vec<File> {
	fs::read_dir(path)
		.unwrap()
		.filter_map(|p| {
			if p.is_err() {
				return None;
			}

			get_file_meta(p.unwrap())
		})
		.collect()
}

fn group_by_year(files: &Vec<File>) -> HashMap<String, Vec<File>> {
	let mut map: HashMap<String, Vec<File>> = HashMap::new();

	files.iter().for_each(|file| {
		let year = file.datetime.format("%Y").to_string();
		if !map.contains_key(&year) {
			map.insert(year.clone(), vec![]);
		}
		let vec = map.get_mut(&year).unwrap();
		vec.push(file.clone());
	});

	map
}

/// Return a list of unique folders based on the posts to convert
fn get_unique_folders(files: &Vec<File>) -> Vec<String> {
	files
		.iter()
		.fold(Vec::with_capacity(files.len()), |mut acc, file| {
			let year = file.datetime.format("%Y").to_string();
			let month = file.datetime.format("%Y/%m").to_string();

			if !acc.contains(&year) {
				acc.push(year);
			}

			if !acc.contains(&month) {
				acc.push(month);
			}

			acc
		})
}

/// Create all year and month directories for
fn create_directories(files: &Vec<File>) -> std::io::Result<()> {
	let default = String::from(".\\output");
	let output_dir = env::var("OUTPUT_DIR").unwrap_or(default);
	let output_path = Path::new(&output_dir);

	fs::create_dir_all(output_path)?;

	for folder in get_unique_folders(files) {
		let path = output_path.join(folder);
		if !path.exists() {
			fs::create_dir(path)?;
		}
	}

	Ok(())
}

fn create_index(
	template_html: &str,
	archive_html: &str,
	title: &str,
	year: &str,
	files: &Vec<File>,
) -> String {
	let list = files
		.iter()
		.map(|file| {
			let dateiso = file.datetime.format("%Y-%m-%d").to_string();
			format!(
				"<li><span>{}</span><a href='{}'>{}</a></li>",
				&dateiso, &file.url, &file.title
			)
		})
		.collect::<Vec<String>>()
		.join("");

	let list = format!("<ul>{}</ul>", list);

	let mut archive = String::from(archive_html);
	archive = archive.replace(r"{{title}}", &title);
	archive = archive.replace(r"{{content}}", &list);

	let mut template = String::from(template_html);
	template = template.replace(r"{{title}}", &title);
	template = template.replace(r"{{content}}", &archive);
	template = template.replace(r"{{dateyear}}", &year);

	template
}

fn create_indexes(output: &str, files: &Vec<File>) -> std::io::Result<()> {
	let template = read_to_string("template\\template.html")?;
	let archive = read_to_string("template\\archive.html")?;

	let now = SystemTime::now();
	let year = st_to_ndt(now).format("%Y").to_string();
	let index = create_index(&template, &archive, "home", &year, files);
	write(format!("{}\\index.html", output), index)?;

	let archives = group_by_year(files);
	for (year, files) in archives {
		let title = format!("Posts from {}", &year);
		let contents = create_index(&template, &archive, &title, &year, &files);
		write(format!("{}\\{}\\index.html", output, &year), contents)?;
	}

	Ok(())
}

fn file_to_template(
	output: &str,
	folder: &str,
	file: &File,
) -> std::io::Result<()> {
	// Ignore files that have already been processed
	// TODO: Look at checksum and update the file if it's different
	let absolute_path = format!("{}{}", output, file.url);
	let path = Path::new(&absolute_path);
	if path.exists() {
		return Ok(());
	}

	let mut template = read_to_string("template\\template.html")?;
	let mut single = read_to_string("template\\single.html")?;

	let title = &file.title;
	let content = &file.contents;
	let dateiso = file.datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();
	let datehuman = file.datetime.format("%A, %d %B %Y").to_string();
	let dateyear = file.datetime.format("%Y").to_string();

	single = single.replace(r"{{title}}", title);
	single = single.replace(r"{{content}}", content);
	single = single.replace(r"{{dateiso}}", &dateiso);
	single = single.replace(r"{{datehuman}}", &datehuman);

	template = template.replace(r"{{content}}", &single);
	template = template.replace(r"{{title}}", title);
	template = template.replace(r"{{dateyear}}", &dateyear);

	write(
		format!("{}\\{}\\{}.html", output, folder, slugify(title)),
		template,
	)?;

	Ok(())
}

fn copy_assets(output: &str, files: &Vec<File>) -> std::io::Result<()> {
	let search = Regex::new(r#""\./(.*)""#).unwrap();
	for file in files {
		let assets = search.captures_iter(&file.contents);
		for asset in assets {
			let asset_path = file.path.with_file_name(&asset[1]);
			let year_month = file.datetime.format("%Y\\%m").to_string();
			let destination =
				format!("{}\\{}\\{}", output, year_month, &asset[1]);
			let to_path = Path::new(&destination);
			if to_path.exists() {
				continue;
			}
			fs::copy(asset_path, to_path)?;
		}
	}

	Ok(())
}

fn main() -> std::io::Result<()> {
	dotenv().ok();

	let path = env::var("DATA_DIR").unwrap();
	let dirs = get_all_dirs(&path);
	let mut files: Vec<File> =
		dirs.into_iter().flat_map(|d| get_files(d)).collect();
	files.sort_by(|a, b| b.datetime.partial_cmp(&a.datetime).unwrap());

	let default = String::from(".\\output");
	let output_dir = env::var("OUTPUT_DIR").unwrap_or(default);

	create_directories(&files)?;
	create_indexes(&output_dir, &files)?;
	copy_assets(&output_dir, &files)?;

	for file in &files {
		let folder = file.datetime.format("%Y/%m").to_string();
		file_to_template(&output_dir, &folder, file)?;
	}

	Ok(())
}
