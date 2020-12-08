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
