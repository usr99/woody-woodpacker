use anyhow::{Result, anyhow};
use libc::Elf64_Phdr;
use std::{fs, os::unix::prelude::PermissionsExt};
use std::io::Write;

use woody_woodpacker::{map::map_file, elf};
mod packer;

macro_rules! update_offset {
	($off:expr, $insertion:expr, $add:expr) => {
		if $off >= $insertion {
			$off += $add;
		}
	}
}

fn main() -> Result<()> {
	let args: Vec<_> = std::env::args().collect();
	if args.len() > 2 {
		eprintln!("warning: ignoring options after \"{}\"", args[1]);
	} else if args.len() == 1 {
		return Err(anyhow!("missing path to an ELF file"));
	}

	let mut source = map_file(&args[1])?;
	let mut elf = elf::parse(&mut source)?;
	let xphdr = match elf.phdrtab.iter_mut().find(|phdr| is_exec_segment(*phdr)) {
		Some(exec_segment) => exec_segment,
		None => return Err(anyhow!("no executable segment found"))
	};

	let packer = packer::generate_packer();
	let jmp = packer::generate_jmp(elf.ehdr, xphdr);

	/*
		if code cave -> increase offset by insertion size
		else -> increase offset by page size
	*/
	let insert_off = xphdr.p_offset + xphdr.p_filesz;
	let pagesize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };

	/* Update every offset after insertion */
	elf.ehdr.e_entry += xphdr.p_filesz;
	xphdr.p_filesz += pagesize;
	xphdr.p_memsz += pagesize;
	update_offset!(elf.ehdr.e_phoff, insert_off, pagesize);
	update_offset!(elf.ehdr.e_shoff, insert_off, pagesize);
	elf.phdrtab.iter_mut().for_each(|header| {
		update_offset!(header.p_offset, insert_off, pagesize);
	});
	elf.shdrtab.iter_mut().for_each(|header| {
		update_offset!(header.sh_offset, insert_off, pagesize);
		if header.sh_type == 1 { // progbits
			header.sh_size += pagesize;
		}
	});

	/* Create woody program with same permissions */
	let mut woody = fs::File::create("woody")?;
	let mut perms = woody.metadata()?.permissions();
	perms.set_mode(0o777);
	woody.set_permissions(perms)?;	

	/* Write packed executable */
	let insert = insert_off as usize;
	let padsize = pagesize as usize - packer.len() - jmp.len();
	let padding = vec![0; padsize];

	woody.write_all(&source[..insert])?;
	woody.write_all(&packer)?;
	woody.write_all(&jmp)?;
	woody.write_all(&padding)?;
	woody.write_all(&source[insert..])?;

	Ok(())
}

fn is_exec_segment(phdr: &Elf64_Phdr) -> bool {
	phdr.p_type == libc::PT_LOAD && phdr.p_flags & libc::PF_X == 1
}
