use std::env;

fn main() {
	let outdir = env::var("OUT_DIR").unwrap();
	let obj = &format!("{}/xor.o", outdir);
	let lib = &format!("{}/libxor.a", outdir);

	println!("cargo:rerun-if-changed=asm/xor.s");
	println!("cargo:rustc-link-search={}", outdir);
	println!("cargo:rustc-link-lib=static=xor");

    if !std::process::Command::new("nasm")
		.args(["-felf64", "asm/xor.s"])
		.args(["-o", obj])
        .output()
        .expect("could not compile assembly module")
        .status
        .success()
    {
        panic!("could not compile object file");
    }

    if !std::process::Command::new("ar")
		.arg("-rc")
		.arg(lib)
		.arg(obj)
        .output()
        .expect("could not compile assembly module")
        .status
        .success()
    {
        panic!("could not compile object file");
    }	
}
