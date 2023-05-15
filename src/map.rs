use std::{fs, ptr, os::fd::AsRawFd, ops::{Deref, DerefMut}};
use anyhow::Result;
use libc::c_void;

pub struct Mapping<'a> {
	buffer: &'a mut [u8]
}

impl<'a> Deref for Mapping<'a> {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		self.buffer
	}
}

impl<'a> DerefMut for Mapping<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.buffer
	}
}

impl<'a> Drop for Mapping<'a> {
	fn drop(&mut self) {
		unsafe { libc::munmap(self.buffer.as_ptr() as *mut c_void, self.buffer.len()); }
	}
}

pub fn map_file(path: &str) -> Result<Mapping> {
	let file = fs::OpenOptions::new()
		.read(true)
		.write(true)
		.open(path)?;
	let filesize = file.metadata()?.len() as usize;

	unsafe {
		let raw_mapping = libc::mmap(
			ptr::null_mut(),
			filesize, 
			libc::PROT_READ | libc::PROT_WRITE,
			libc::MAP_PRIVATE,
			file.as_raw_fd(),
			0);

		if raw_mapping == libc::MAP_FAILED {
			let os_error = std::io::Error::last_os_error();
			return Err(anyhow::format_err!(os_error));
		}

		Ok(Mapping {
			buffer: std::slice::from_raw_parts_mut(raw_mapping as *mut u8, filesize)
		})
	}		
}
