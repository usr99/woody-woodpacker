use anyhow::{Result, anyhow};

use woody_woodpacker::map::map_file;

fn main() -> Result<()> {
	let args: Vec<_> = std::env::args().collect();
	if args.len() > 2 {
		eprintln!("warning: ignoring options after \"{}\"", args[1]);
	} else if args.len() == 1 {
		return Err(anyhow!("missing path to an ELF file"));
	}

	let mut source = map_file(&args[1])?;

	unsafe {
		print!("{}", std::str::from_utf8_unchecked(&source));
	}

	Ok(())
}
