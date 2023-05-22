use anyhow::{Result, anyhow};

use woody_woodpacker::{map::map_file, elf};

fn main() -> Result<()> {
	let args: Vec<_> = std::env::args().collect();
	if args.len() > 2 {
		eprintln!("warning: ignoring options after \"{}\"", args[1]);
	} else if args.len() == 1 {
		return Err(anyhow!("missing path to an ELF file"));
	}

	let mut source = map_file(&args[1])?;
	let elf = elf::parse(&source)?;

	let xphdr = match elf.phdrtab.iter().find(|phdr| {
		phdr.p_type == libc::PT_LOAD && phdr.p_flags & libc::PF_X == 1
	}) {
		Some(exec_segment) => exec_segment,
		None => return Err(anyhow!("no executable segment found"))
	};

	unsafe {
		print!("{}", std::str::from_utf8_unchecked(&source));
	}

	Ok(())
}
