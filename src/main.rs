use dotenv::dotenv;
use env_logger;
use fs::write;
use log;
use regex::Regex;
use rss::{ChannelBuilder, GuidBuilder, Item, ItemBuilder};
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
			log::debug!("[get_all_dirs] Discovered {}", &name);
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
fn group_by_year(files: &[File]) -> HashMap<String, Vec<File>> {
	let mut map: HashMap<String, Vec<File>> = HashMap::new();

	files.iter().for_each(|file| {
		if !map.contains_key(&file.year) {
			log::debug!("[group_by_year] Adding year {}", &file.year);
			map.insert(file.year.clone(), vec![]);
		}
		let vec = map.get_mut(&file.year).unwrap();
		vec.push(file.clone());
	});

	map
}

/// Return a list of unique folders based on the posts to convert
fn get_unique_folders(files: &[File]) -> Vec<String> {
	let mut folders: Vec<String> =
		files.iter().map(|file| file.year_month.clone()).collect();

	folders.sort();
	folders.dedup();

	log::debug!("[get_unique_folders] {} unique folders", &folders.len());
	log::debug!("[get_unique_folders] {:?}", &folders);

	folders
}

/// Create all year and month directories for
fn create_directories(files: &[File]) -> std::io::Result<()> {
	let output_dir = env::get_output_dir();
	let output_path = Path::new(&output_dir);

	log::debug!("[create_directories] Creating '{}'", &output_dir);
	fs::create_dir_all(output_path)?;

	for folder in get_unique_folders(files) {
		let path = output_path.join(folder);
		if !path.exists() {
			log::debug!(
				"[create_directories] Creating '{}'",
				path.to_str().unwrap()
			);
			fs::create_dir_all(path)?;
		}
	}

	Ok(())
}

fn shorten_text(text: &str, max_length: usize) -> String {
	if text.len() < max_length {
		String::from(text)
	} else {
		let short: String = text.chars().take(max_length).collect();
		format!("{}...", short)
	}
}

fn format_file_template(file: &File, template_name: &str) -> String {
	let tmp_filename = &format!("{}.html", template_name);
	let tmp_path = util::str_to_path(&["template", tmp_filename]).unwrap();
	let mut template = read_to_string(tmp_path).unwrap();

	let title = if file.title == "" {
		shorten_text(&file.raw_contents, 48)
	} else {
		String::from(&file.title)
	};

	let summary = shorten_text(&file.raw_contents, 160);

	template = template.replace(r"{{url}}", &file.url);
	template = template.replace(r"{{title}}", &title);
	template = template.replace(r"{{summary}}", &summary);
	template = template.replace(r"{{dateiso}}", &file.dateiso);
	template = template.replace(r"{{datehuman}}", &file.datehuman);
	template = template.replace(r"{{dateisoshort}}", &file.dateisoshort);
	template = template.replace(r"{{content}}", &file.contents);

	template
}

/// Generate the contents for an archive index file. Either for a yearly archive
/// or for the entire collection for the main index.
fn create_index(
	template_html: &str,
	archive_html: &str,
	title: &str,
	year: &str,
	files: &[File],
	recent_posts: &str,
	site: &env::Site,
) -> String {
	let list = files
		.iter()
		.map(|file| format_file_template(&file, "archive-item"))
		.collect::<Vec<String>>()
		.join("");

	let mut archive = String::from(archive_html);
	archive = archive.replace(r"{{title}}", &title);
	archive = archive.replace(r"{{archive-item}}", &list);

	let mut template = String::from(template_html);
	template = template.replace(r"{{title}}", &title);
	template = template.replace(r"{{content}}", &archive);
	template = template.replace(r"{{dateyear}}", &year);
	template = template.replace(r"{{recent-posts}}", recent_posts);

	// Add global site config variables
	template = template.replace(r"{{site-title}}", &site.title);
	template = template.replace(r"{{site-description}}", &site.description);
	template = template.replace(r"{{site-url}}", &site.url_base);

	template
}

/// Generate an index file for each unique year where articles have been posted.
/// Also generate a main index file.
fn create_indexes(
	output: &str,
	files: &[File],
	recent_posts: &str,
	site: &env::Site,
) -> std::io::Result<()> {
	let tmp_path = util::str_to_path(&["template", "template.html"]).unwrap();
	let arc_path = util::str_to_path(&["template", "archive.html"]).unwrap();

	let template = read_to_string(tmp_path)?;
	let archive = read_to_string(arc_path)?;

	let now = SystemTime::now();
	let year = util::st_to_ndt(now).format("%Y").to_string();
	let index = create_index(
		&template,
		&archive,
		"Blog Posts",
		&year,
		files,
		recent_posts,
		site,
	);

	let write_path = util::str_to_path(&[output, "index.html"]).unwrap();
	log::debug!("[create_indexes] Writing main index");
	write(write_path, index)?;

	let archives = group_by_year(files);
	for (year, files) in archives {
		let title = format!("Posts from {}", &year);
		let contents = create_index(
			&template,
			&archive,
			&title,
			&year,
			&files,
			recent_posts,
			site,
		);

		let write_path =
			util::str_to_path(&[output, &year, "index.html"]).unwrap();
		log::debug!(
			"[create_indexes] Writing '{}' index",
			write_path.to_str().unwrap()
		);
		write(write_path, contents)?;
	}

	Ok(())
}

/// Generate a single article file based on the contents of a file
fn article_to_file(
	file: &File,
	recent_posts: &str,
	site: &env::Site,
) -> std::io::Result<()> {
	// Ignore files that have already been processed
	// TODO: Look at checksum and update the file if it's different
	let path = Path::new(&file.output_file);
	if path.exists() && !env::get_overwrite_existing() {
		log::debug!(
			"[article_to_file] '{}' already exists. Skipping.",
			&path.to_str().unwrap()
		);
		return Ok(());
	}

	let tmp_path = util::str_to_path(&["template", "template.html"]).unwrap();
	let mut template = read_to_string(tmp_path)?;

	let single = format_file_template(&file, "single");

	template = template.replace(r"{{content}}", &single);
	template = template.replace(r"{{title}}", &file.title);
	template = template.replace(r"{{dateyear}}", &file.year);
	template = template.replace(r"{{recent-posts}}", recent_posts);

	// Add global site config variables
	template = template.replace(r"{{site-title}}", &site.title);
	template = template.replace(r"{{site-description}}", &site.description);
	template = template.replace(r"{{site-url}}", &site.url_base);

	log::debug!(
		"[article_to_file] Writing article '{}'",
		&path.to_str().unwrap()
	);
	write(&file.output_file, template)?;

	Ok(())
}

/// Use the "recent-post" template to take the five most recent posts and
/// create a string to add them to each blog post and index
fn get_recent_posts(files: &[File]) -> String {
	let path = util::str_to_path(&["template", "recent-post.html"]).unwrap();

	if !path.exists() {
		log::debug!(
			"[get_recent_posts] Template '{}' doesn't exist. Skipping.",
			&path.to_str().unwrap()
		);
		return String::from("");
	}

	files
		.iter()
		.take(5)
		.map(|file| format_file_template(&file, "recent-post"))
		.collect::<Vec<String>>()
		.join("")
}

/// Grab any locally linked assets in the contents of a file into the same
/// folder as the article.
fn copy_assets(output: &str, files: &[File]) -> std::io::Result<()> {
	let search = Regex::new(r#"["']\./([^"']*)["']"#).unwrap();
	for file in files {
		let assets = search.captures_iter(&file.contents);
		for asset in assets {
			let asset_path = file.path.with_file_name(&asset[1]);
			let destination =
				util::str_to_path(&[output, &file.year_month, &asset[1]])
					.unwrap();

			if destination.exists() {
				log::debug!(
					"[copy_assets] '{}' already exists. Skipping.",
					&asset[1]
				);
				continue;
			}

			log::debug!("[copy_assets] Copying file '{}'", &asset[1]);
			fs::copy(asset_path, destination)?;
		}
	}

	Ok(())
}

fn files_to_rss(files: &[File], site: &env::Site) {
	let items: Vec<Item> = files
		.iter()
		.map(|file| {
			let post_url = format!("{}{}", &site.url_base, file.url);

			let guid = GuidBuilder::default()
				.value(&post_url.clone())
				.permalink(true)
				.build();

			let pub_date =
				file.datetime.format("%a, %d %b %Y %X GMT").to_string();

			ItemBuilder::default()
				.title(Some(file.title.clone()))
				.description(Some(shorten_text(&file.raw_contents, 160)))
				.link(Some(post_url))
				.guid(Some(guid))
				.pub_date(Some(pub_date))
				.build()
		})
		.collect();

	let rss_feed = ChannelBuilder::default()
		.title(&site.title)
		.description(&site.description)
		.link(&site.url_base)
		.generator(Some(String::from(
			"Tombo Archive (https://git.tombo.sh/tom/rust-tombo-archive)",
		)))
		.items(items)
		.build()
		.to_string();

	let write_path =
		util::str_to_path(&[&env::get_output_dir(), "rss.xml"]).unwrap();

	write(write_path, rss_feed).unwrap();
}

fn main() -> std::io::Result<()> {
	// Step 0 - Set up environment
	dotenv().ok();
	env_logger::init();
	let output_dir = env::get_output_dir();
	let path = env::get_data_dir();
	let site_config = env::get_site_config();

	// Step 1 - Get all public directories
	let dirs = get_all_dirs(&path);

	// Step 2 - Get all articles/notes within each public directory
	let mut files: Vec<File> =
		dirs.into_iter().flat_map(|d| get_files(d)).collect();

	log::debug!("[main] Found {} articles to generate", files.len());

	// Step 3 - Sort files by date
	files.sort_by(|a, b| b.datetime.partial_cmp(&a.datetime).unwrap());

	// Step 3.5 - Get recent posts to use on sidebar
	let recent_posts = get_recent_posts(&files);

	// Step 4 - Create necessary folders and files
	create_directories(&files)?;
	create_indexes(&output_dir, &files, &recent_posts, &site_config)?;
	copy_assets(&output_dir, &files)?;

	// Step 5 - Convert each file into an article
	for file in &files {
		article_to_file(file, &recent_posts, &site_config)?;
	}

	// Step 6 - Output RSS feed
	files_to_rss(&files, &site_config);

	Ok(())
}
