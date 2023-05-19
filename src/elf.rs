use std::mem::size_of;

use anyhow::{Result, anyhow};
use libc::{Elf64_Ehdr, Elf64_Phdr};

macro_rules! parse_elf {
	($elf:expr, $offset:expr, $r#type: ty) => {
		unsafe { &*($elf.as_ptr().add($offset as usize) as *const $r#type) }
	}
}

pub fn take_elf_header(binary: &[u8]) -> Result<&Elf64_Ehdr> {
	if binary.len() < size_of::<Elf64_Ehdr>() {
		return Err(anyhow!("file is too small to contain ELF data"));
	}

	let ehdr = parse_elf!(binary, 0, Elf64_Ehdr);
	validate_elf_header(ehdr)?;

	Ok(ehdr)
}

fn validate_elf_header(ehdr: &Elf64_Ehdr) -> Result<()> {
	let magic = parse_elf!(ehdr.e_ident, 0, u32);
	if *magic != 0x464c457f { // ELF Magic Number
		return Err(anyhow!("not an ELF file"));
	}

	if ehdr.e_ident[libc::EI_CLASS] != libc::ELFCLASS64 {
		return Err(anyhow!("not a 64-bit executable"));
	}

	if ehdr.e_ident[libc::EI_DATA] != libc::ELFDATA2LSB {
		return Err(anyhow!("not a little-endian executable"));
	}

	if ehdr.e_type == libc::ET_DYN {
		return Err(anyhow!("shared objects are not currently supported"));
	}

	if ehdr.e_type != libc::ET_EXEC {
		return Err(anyhow!("not an executable file"));
	}

	if ehdr.e_machine != libc::EM_X86_64 {
		return Err(anyhow!("x86_64 is the only architecture supported"));
	}

	Ok(())
}

pub fn take_exec_program_header<'a>(ehdr: &Elf64_Ehdr, data: &'a [u8]) -> Result<&'a Elf64_Phdr> {
	let mut offset = ehdr.e_phoff as usize;
	let phdrtab_size = (ehdr.e_phentsize * ehdr.e_phnum) as usize;
	let phdrtab_end = offset + phdrtab_size;
	if phdrtab_end > data.len() {
		return Err(anyhow!("invalid offset: program table header"));
	}

	while offset < phdrtab_end {
		let phdr = parse_elf!(data, ehdr.e_phoff, Elf64_Phdr);
		if validate_exec_phdr(phdr, data.len()) {
			return Ok(phdr);
		}

		offset += ehdr.e_phentsize as usize;
	}

	Err(anyhow!("ELF binary does not contain any executable segment"))
}

fn validate_exec_phdr(phdr: &Elf64_Phdr, datalen: usize) -> bool {
	if phdr.p_type != libc::PT_LOAD {
		return false;
	}		

	if phdr.p_flags & libc::PF_X != 1 {
		return false;
	}

	let segment_end = (phdr.p_offset + phdr.p_filesz) as usize;
	if segment_end > datalen {
		return false;
	}

	true
}

