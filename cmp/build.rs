//! CMP build script.
//!
//! This build script performs automatic asset compilation if the required
//! programs are available:
//! - libresprite, to compile all *.ase files into corresponding *.png files
//! - qoi, to compile all *.png files into corresponding *.qoi files
//!
//! The QOI files are also committed to the repository, so this script's success is not a prerequisite for compiling and
//! running the game. However, it helps tremendously when working on assets. The process can be done manually in any
//! case.
//!
//! This script further embeds an EXE icon into the compiled binary for Windows.

extern crate embed_resource;

use std::env;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use anyhow::{anyhow, Result};
use image::ImageFormat;

const ASSET_DIRECTORY: &str = "../assets";
const PNG_TEMP_SUBDIRECTORY: &str = "png";
const ASE_EXTENSION: &str = "ase";
const PNG_EXTENSION: &str = "png";
const QOI_EXTENSION: &str = "qoi";

fn main() {
	println!("cargo:rerun-if-changed={}", ASSET_DIRECTORY);

	let target = std::env::var("TARGET").unwrap();
	if target.contains("windows") {
		embed_windows_icon();
	}

	let ase_files = find_all_ase_inputs();
	println!("Converting ase files: {:?}", ase_files);

	if FULL_ASSET_DIRECTORY.exists() {
		std::fs::remove_dir_all(FULL_ASSET_DIRECTORY.as_path()).unwrap();
	}
	std::fs::create_dir(FULL_ASSET_DIRECTORY.as_path()).unwrap();

	let png_files = convert_all_ase_to_png(&ase_files);
	convert_all_png_to_qoi(&png_files);
}

fn embed_windows_icon() {
	embed_resource::compile(PathBuf::from(ASSET_DIRECTORY).join("icon.rc"));
}

fn find_all_ase_inputs() -> Vec<PathBuf> {
	let base_path = PathBuf::from(ASSET_DIRECTORY);
	base_path
		.read_dir()
		.into_iter()
		.flatten()
		.filter_map(|maybe_entry| maybe_entry.map(|entry| entry.path()).ok())
		.filter(|entry| entry.extension() == Some(&OsString::from(ASE_EXTENSION)))
		.collect()
}

fn convert_all_ase_to_png(ase_files: &[impl AsRef<Path> + Debug]) -> Vec<PathBuf> {
	let mut resulting_pngs = Vec::new();
	for ase_file in ase_files {
		match convert_ase_to_png(ase_file) {
			Ok(png_path) => resulting_pngs.push(png_path),
			Err(why) => println!("cargo:warning=File {:?} could not be converted to PNG: {}", ase_file, why),
		}
	}
	resulting_pngs
}

fn convert_ase_to_png(ase: impl AsRef<Path>) -> Result<PathBuf> {
	let output_path = to_png_temp_output(&ase)?;
	let command =
		Command::new("libresprite").args(["--batch", "--sheet"]).arg(&output_path).arg(ase.as_ref()).output()?;

	if !command.status.success() {
		Err(anyhow!(format!(
			"libresprite exited with code {}. Stdout: {}\nStderr: {}",
			command.status.code().unwrap_or(-1),
			String::from_utf8_lossy(&command.stdout),
			String::from_utf8_lossy(&command.stderr)
		)))
	} else {
		Ok(output_path)
	}
}

fn convert_all_png_to_qoi(png_files: &[impl AsRef<Path> + Debug]) {
	for png_file in png_files {
		if let Err(why) = convert_png_to_qoi(png_file) {
			println!("cargo:warning=File {:?} could not be converted to QOI: {}", png_file, why);
		}
	}
}

fn convert_png_to_qoi(png_file: impl AsRef<Path>) -> Result<()> {
	let output_path = to_qoi_output(&png_file)?;
	let image = image::load(std::io::BufReader::new(std::fs::File::open(png_file.as_ref())?), ImageFormat::Png)?;
	image.write_to(
		&mut std::io::BufWriter::new(
			std::fs::File::options().write(true).truncate(true).create(true).open(output_path)?,
		),
		ImageFormat::Qoi,
	)?;
	Ok(())
}

static FULL_ASSET_DIRECTORY: LazyLock<PathBuf> =
	LazyLock::new(|| Path::new(&env::var_os("OUT_DIR").unwrap_or(".".into())).join(PNG_TEMP_SUBDIRECTORY));

fn to_png_temp_output(ase: impl AsRef<Path>) -> Result<PathBuf> {
	Ok(FULL_ASSET_DIRECTORY
		.join(ase.as_ref().with_extension(PNG_EXTENSION).file_name().ok_or(anyhow!("ase file path is invalid"))?))
}

fn to_qoi_output(png_file: impl AsRef<Path>) -> Result<PathBuf> {
	Ok(Path::new(ASSET_DIRECTORY)
		.join(png_file.as_ref().with_extension(QOI_EXTENSION).file_name().ok_or(anyhow!("png file path is invalid"))?))
}
