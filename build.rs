fn main() {
	println!("cargo:rustc-link-search=/home/mamartin/Projects/woody-rs/misc");
	println!("cargo:rustc-link-lib=static=xor");
}
