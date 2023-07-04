use std::mem::size_of;
use thiserror::Error;
use libc::{Elf64_Ehdr, Elf64_Phdr, Elf64_Shdr};

macro_rules! parse_elf {
	($elf:expr, $offset:expr, $r#type: ty) => {
		unsafe { &mut *($elf.as_ptr().add($offset as usize) as *mut $r#type) }
	}
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

pub fn fetch_headers(elf: &mut [u8]) -> Result<(&mut Elf64_Ehdr, &mut [Elf64_Phdr], &mut [Elf64_Shdr])> {
	if elf.len() < size_of::<Elf64_Ehdr>() {
		return Err(NotAnElf);
	}
	
	let ehdr = unsafe { &mut *(elf.as_mut_ptr() as *mut Elf64_Ehdr) };
	validate_elf_header(ehdr)?;
	
	bound_check(ehdr.e_shoff as usize + (ehdr.e_phentsize * ehdr.e_phnum) as usize, elf.len())?;
	let phdr = unsafe { std::slice::from_raw_parts_mut(
		elf.as_mut_ptr().add(ehdr.e_phoff as usize) as *mut Elf64_Phdr,
		ehdr.e_phnum as usize) };
	
	bound_check(ehdr.e_shoff as usize + (ehdr.e_shentsize * ehdr.e_shnum) as usize, elf.len())?;
	let shdr = unsafe { std::slice::from_raw_parts_mut(
		elf.as_mut_ptr().add(ehdr.e_shoff as usize) as *mut Elf64_Shdr,
		ehdr.e_shnum as usize) };

	Ok((ehdr, phdr, shdr))
}

pub fn is_exec_segment(phdr: &Elf64_Phdr) -> bool {
	phdr.p_type == libc::PT_LOAD && phdr.p_flags & libc::PF_X == 1
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
