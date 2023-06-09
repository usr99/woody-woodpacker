use std::mem::size_of;
use thiserror::Error;
use libc::{Elf64_Ehdr, Elf64_Phdr, Elf64_Shdr};

macro_rules! parse_elf {
	($elf:expr, $offset:expr, $r#type: ty) => {
		unsafe { &mut *($elf.as_ptr().add($offset as usize) as *mut $r#type) }
	}
}

pub struct Elf<'a> {
	pub ehdr: &'a mut Elf64_Ehdr,
	pub phdrtab: &'a mut [Elf64_Phdr],
	pub shdrtab: &'a mut [Elf64_Shdr]
}

#[derive(Error, Debug)]
pub enum Error {
	#[error("ELF magic number doesn't match")]
	NotAnElf,
	#[error("only 64-bit executables are supported")]
	InvalidClass,
	#[error("only little-endian executables are supported")]
	InvalidEndianness,
	#[error("not an executable")]
	InvalidType,
	#[error("architecture is not x86_64")]
	InvalidArchitecture,
	#[error("corrupted file: offset {0} is out of bounds")]
	InvalidOffset(usize),
	#[error("requested entity was not found")]
	NotFound()
}
use Error::*;

pub type Result<T> = std::result::Result<T, Error>;

pub fn parse(file: &mut [u8]) -> Result<Elf> {
	if file.len() < size_of::<Elf64_Ehdr>() {
		return Err(NotAnElf);
	}

	let ehdr = parse_elf!(file, 0, Elf64_Ehdr);
	validate_elf_header(ehdr)?;

	let offset = ehdr.e_phoff as usize;
	bound_check(offset + (ehdr.e_phentsize * ehdr.e_phnum) as usize, file.len())?;
	let phdrtab = unsafe {
		std::slice::from_raw_parts_mut(
			file.as_ptr().add(offset) as *mut Elf64_Phdr,
			ehdr.e_phnum as usize
		)
	};

	let offset = ehdr.e_shoff as usize;
	bound_check(offset + (ehdr.e_shentsize * ehdr.e_shnum) as usize, file.len())?;
	let shdrtab = unsafe {
		std::slice::from_raw_parts_mut(
			file.as_ptr().add(offset) as *mut Elf64_Shdr,
			ehdr.e_shnum as usize
		)
	};

	Ok(Elf { ehdr, phdrtab, shdrtab })
}

fn validate_elf_header(ehdr: &Elf64_Ehdr) -> Result<()> {
	let magic = parse_elf!(ehdr.e_ident, 0, u32);
	if *magic != 0x464c457f { // ELF Magic Number
		return Err(NotAnElf);
	}

	if ehdr.e_ident[libc::EI_CLASS] != libc::ELFCLASS64 {
		return Err(InvalidClass);
	}

	if ehdr.e_ident[libc::EI_DATA] != libc::ELFDATA2LSB {
		return Err(InvalidEndianness);
	}

	if ehdr.e_type != libc::ET_EXEC && ehdr.e_type != libc::ET_DYN || ehdr.e_entry == 0 {
		return Err(InvalidType);
	}

	if ehdr.e_machine != libc::EM_X86_64 {
		return Err(InvalidArchitecture);
	}

	Ok(())
}

fn bound_check(offset: usize, max: usize) -> Result<()> {
	if offset > max {
		Err(InvalidOffset(offset))
	} else {
		Ok(())
	}
}
