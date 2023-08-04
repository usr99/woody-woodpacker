use anyhow::{Result, anyhow};
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

extern "C" {
	fn xor_cipher(buf: *mut u8, len: usize);
}

fn main() -> Result<()> {
	let args: Vec<_> = std::env::args().collect();
	if args.len() > 2 {
		eprintln!("warning: ignoring options after \"{}\"", args[1]);
	} else if args.len() == 1 {
		return Err(anyhow!("missing path to an ELF file"));
	}

	let mut source = map_file(&args[1])?;
	let (ehdr, phdrtab, shdrtab) = elf::fetch_headers(source.as_mut())?;
	let xphdr = match phdrtab.iter().position(elf::is_exec_segment) {
		Some(idx) => &mut phdrtab[idx], 
		None => return Err(anyhow!("no executable segment found"))
	};

	let packer = packer::generate_packer(ehdr, xphdr);

	/*
		if code cave -> increase offset by insertion size
		else -> increase offset by page size
	*/
	
	let insert_off = xphdr.p_offset + xphdr.p_filesz;
	let pagesize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };

	/* Save xphdr values before dropping all references to subslices of source */
	let cipher_off = xphdr.p_offset as usize;
	let cipher_len = xphdr.p_filesz as usize;

	/* Update every offset after insertion */
	ehdr.e_entry = xphdr.p_vaddr + xphdr.p_filesz;
	xphdr.p_filesz += pagesize;
	xphdr.p_memsz += pagesize;
	xphdr.p_flags |= libc::PF_W;
	update_offset!(ehdr.e_phoff, insert_off, pagesize);
	update_offset!(ehdr.e_shoff, insert_off, pagesize);
	phdrtab.iter_mut().for_each(|header| {
		update_offset!(header.p_offset, insert_off, pagesize);
	});
	shdrtab.iter_mut().for_each(|header| {
		update_offset!(header.sh_offset, insert_off, pagesize);
		if header.sh_type == 1 { // progbits
			header.sh_size += pagesize;
		}
	});

	/* Obfuscate executable segment */
	let exec_segment_data = &mut source[cipher_off..][..cipher_len];
	unsafe { xor_cipher(exec_segment_data.as_mut_ptr(), exec_segment_data.len()); }

	/* Create woody program with same permissions */
	let mut woody = fs::File::create("woody")?;
	let mut perms = woody.metadata()?.permissions();
	perms.set_mode(0o777);
	woody.set_permissions(perms)?;	

	/* Write packed executable */
	let insert = insert_off as usize;
	let padsize = pagesize as usize - packer.len();
	let padding = vec![0; padsize];

	woody.write_all(&source[..insert])?;
	woody.write_all(&packer)?;
	woody.write_all(&padding)?;
	woody.write_all(&source[insert..])?;

	Ok(())
}
