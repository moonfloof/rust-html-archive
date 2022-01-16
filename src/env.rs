use std::env;

pub fn get_output_dir() -> String {
	let default = String::from("./output");
	env::var("OUTPUT_DIR").unwrap_or(default)
}

pub fn get_extensions() -> Vec<String> {
	let default = String::from("html,md,txt");
	let extensions = env::var("EXTENSIONS").unwrap_or(default);

	extensions.split(",").map(|s| String::from(s)).collect()
}

pub fn get_public_dir() -> String {
	let default = String::from("public_archive");
	env::var("PUBLIC_DIR").unwrap_or(default)
}

pub fn get_data_dir() -> String {
	env::var("DATA_DIR").unwrap()
}

pub fn get_overwrite_existing() -> bool {
	let default = String::from("false");
	let overwrite_existing = env::var("OVERWRITE_EXISTING").unwrap_or(default);

	overwrite_existing == "true"
}

pub struct Site {
	pub title: String,
	pub url_base: String,
	pub description: String,
}

pub fn get_site_config() -> Site {
	let title = env::var("SITE_TITLE").unwrap_or(String::from(""));
	let mut url_base = env::var("SITE_URL_BASE").unwrap_or(String::from(""));
	let description = env::var("SITE_DESCRIPTION").unwrap_or(String::from(""));

	// Replace trailing slashes
	url_base = url_base.trim_end_matches("/").to_owned();

	Site {
		title,
		url_base,
		description,
	}
}
