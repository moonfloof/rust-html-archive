use dotenv::dotenv;
use fs::write;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, read_to_string};
use std::path::Path;
use std::time::SystemTime;

mod env;
mod file;
mod util;

use file::File;

/// Get all public directories
fn get_all_dirs(path: &str) -> Vec<Box<Path>> {
	let dir_match = env::get_public_dir();
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

/// Retrieve all files from one archive folder
fn get_files(path: Box<Path>) -> Vec<File> {
	fs::read_dir(path)
		.unwrap()
		.filter_map(|p| {
			if p.is_err() {
				return None;
			}

			File::new(p.unwrap())
		})
		.collect()
}

/// Convert single Vec of Files into groups of years
/// (for making yearly archives)
fn group_by_year(files: &Vec<File>) -> HashMap<String, Vec<File>> {
	let mut map: HashMap<String, Vec<File>> = HashMap::new();

	files.iter().for_each(|file| {
		if !map.contains_key(&file.year) {
			map.insert(file.year.clone(), vec![]);
		}
		let vec = map.get_mut(&file.year).unwrap();
		vec.push(file.clone());
	});

	map
}

/// Return a list of unique folders based on the posts to convert
fn get_unique_folders(files: &Vec<File>) -> Vec<String> {
	let mut folders: Vec<String> =
		files.iter().map(|file| file.year_month.clone()).collect();

	folders.sort();
	folders.dedup();

	folders
}

/// Create all year and month directories for
fn create_directories(files: &Vec<File>) -> std::io::Result<()> {
	let output_dir = env::get_output_dir();
	let output_path = Path::new(&output_dir);

	fs::create_dir_all(output_path)?;

	for folder in get_unique_folders(files) {
		let path = output_path.join(folder);
		if !path.exists() {
			fs::create_dir_all(path)?;
		}
	}

	Ok(())
}

/// Generate the contents for an archive index file. Either for a yearly archive
/// or for the entire collection for the main index.
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

/// Generate an index file for each unique year where articles have been posted.
/// Also generate a main index file.
fn create_indexes(output: &str, files: &Vec<File>) -> std::io::Result<()> {
	let template = read_to_string("template\\template.html")?;
	let archive = read_to_string("template\\archive.html")?;

	let now = SystemTime::now();
	let year = util::st_to_ndt(now).format("%Y").to_string();
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

/// Generate a single article file based on the contents of a file
fn file_to_template(file: &File) -> std::io::Result<()> {
	// Ignore files that have already been processed
	// TODO: Look at checksum and update the file if it's different
	let path = Path::new(&file.output_file);
	if path.exists() {
		return Ok(());
	}

	let mut template = read_to_string("template\\template.html")?;
	let mut single = read_to_string("template\\single.html")?;

	single = single.replace(r"{{title}}", &file.title);
	single = single.replace(r"{{content}}", &file.contents);
	single = single.replace(r"{{dateiso}}", &file.dateiso);
	single = single.replace(r"{{datehuman}}", &file.datehuman);

	template = template.replace(r"{{content}}", &single);
	template = template.replace(r"{{title}}", &file.title);
	template = template.replace(r"{{dateyear}}", &file.year);

	write(&file.output_file, template)?;

	Ok(())
}

/// Grab any locally linked assets in the contents of a file into the same
/// folder as the article.
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
	// Step 0 - Load environment variables
	dotenv().ok();
	let output_dir = env::get_output_dir();
	let path = env::get_data_dir();

	// Step 1 - Get all public directories
	let dirs = get_all_dirs(&path);

	// Step 2 - Get all articles/notes within each public directory
	let mut files: Vec<File> =
		dirs.into_iter().flat_map(|d| get_files(d)).collect();

	// Step 3 - Sort files by date
	files.sort_by(|a, b| b.datetime.partial_cmp(&a.datetime).unwrap());

	// Step 4 - Create necessary folders and files
	create_directories(&files)?;
	create_indexes(&output_dir, &files)?;
	copy_assets(&output_dir, &files)?;

	// Step 5 - Convert each file into an article
	for file in &files {
		file_to_template(file)?;
	}

	Ok(())
}
