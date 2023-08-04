use libc::{Elf64_Ehdr, Elf64_Phdr};
use std::mem::size_of_val;

pub const WOODY_LEN: usize = 74;
const WOODY_INSTR: [u8; WOODY_LEN] = [
	// Allocate stack frame
	0x48, 0x83, 0xec, 0x0a,			// sub rsp, 10
	// write(1, "woody\n", 6)
	0xbf, 0x01, 0x00, 0x00, 0x00,	// mov rdi, 1
	0xc6, 0x04, 0x24, 0x57,			// mov [rsp+0], 'w'
	0xc6, 0x44, 0x24, 0x01, 0x4f,	// mov [rsp+1], 'o'
	0xc6, 0x44, 0x24, 0x02, 0x4f,	// mov [rsp+2], 'o'
	0xc6, 0x44, 0x24, 0x03, 0x44,	// mov [rsp+3], 'd'
	0xc6, 0x44, 0x24, 0x04, 0x59,	// mov [rsp+4], 'y'
	0xc6, 0x44, 0x24, 0x05, 0x0a,	// mov [rsp+5], '\n'
	0x48, 0x89, 0xe6,				// mov rsi, rsp
	0xba, 0x06, 0x00, 0x00, 0x00,	// mov rdx, 6
	0xb8, 0x01, 0x00, 0x00, 0x00,	// mov rax, 1
	0x0f, 0x05,						// syscall
	// Restore stack frame
	0x48, 0x83, 0xc4, 0x0a,			// add rsp, 10
	// Clear used registers
	0x48, 0x31, 0xff,				// xor rdi, rdi
	0x48, 0x31, 0xf6,				// xor rsi, rsi
	0x48, 0x31, 0xd2,				// xor rdx, rdx
	0x48, 0x31, 0xc0,				// xor rax, rax
	// Execute original program
	0xe9, 0xff, 0xff, 0xff, 0xff	// jmp <entrypoint address> (relative jump 32bit address)
];

pub const NO_PIE_INIT_LEN: usize = 21;
const NO_PIE_INIT_INSTR: [u8; NO_PIE_INIT_LEN] = [
	0x90, 0x48, 0xb8, 			// mov rax <start of exec segment>
	0xff, 0xff, 0xff, 0xff,		// placeholder
	0xff, 0xff, 0xff, 0xff,		// placeholder
	0x48, 0xbf, 				// mov rdi <end of exec segment>
	0xff, 0xff, 0xff, 0xff,		// placeholder
	0xff, 0xff, 0xff, 0xff		// placeholder
];

pub const PIE_INIT_LEN: usize = 14;
const PIE_INIT_INSTR: [u8; PIE_INIT_LEN] = [
	0x48, 0x8d, 0x05, 			// lea rax, [rip + <placeholder>]
	0xff, 0xff, 0xff, 0xff,
	0x48, 0x8d, 0x3d, 			// lea rdi, [rip + <placeholder>]
	0xff, 0xff, 0xff, 0xff,	
];

pub const PACKER_LOOP_LEN: usize = 12;
const PACKER_LOOP_INSTR: [u8; PACKER_LOOP_LEN] = [
	// loop label definition
	0x80, 0x30, 0x61,				// xor byte [rax], 0x61
	0x48, 0x83, 0xc0, 0x01,			// add rax, 1
	0x48, 0x39, 0xf8,				// cmp rax, rdi
	0x75, 0xf4						// jne "loop"
];

fn generate_no_pie(xphdr: &Elf64_Phdr) -> [u8; NO_PIE_INIT_LEN] {
	let mut instructions = NO_PIE_INIT_INSTR;
	
	// Store executable segment bounds
	// it defines the area to "de-obfuscate"
	let start = xphdr.p_vaddr as usize;
	let end = (xphdr.p_vaddr + xphdr.p_memsz) as usize;
	
	let address = &mut instructions[2..][..8];
	address.copy_from_slice(&start.to_le_bytes());

	let address = &mut instructions[12..][..8];
	address.copy_from_slice(&end.to_le_bytes());

	return instructions;
}

fn generate_pie(xphdr: &Elf64_Phdr) -> [u8; PIE_INIT_LEN] {
	let mut instructions = PIE_INIT_INSTR;
	
	// Compute executable segment bounds
	// as offsets to the rip register
	let start = -(7 + xphdr.p_memsz as i32);
	let end = -14 as i32;

	let address = &mut instructions[3..][..4];
	address.copy_from_slice(&start.to_le_bytes());

	let address = &mut instructions[10..][..4];
	address.copy_from_slice(&end.to_le_bytes());

	return instructions;
}

pub fn generate_packer(ehdr: &Elf64_Ehdr, xphdr: &Elf64_Phdr) -> Vec<u8> {	
	let init;
	if ehdr.e_type == libc::ET_EXEC {
		init = generate_no_pie(xphdr).to_vec();
	} else {
		init = generate_pie(xphdr).to_vec();
	}
	
	let packer_loop = PACKER_LOOP_INSTR;

	let mut woody = WOODY_INSTR;
	let insert_size = (init.len() + packer_loop.len() + WOODY_LEN) as u64;
	let reljump32 = -((xphdr.p_vaddr + xphdr.p_memsz + insert_size - ehdr.e_entry) as i32);
	let address = &mut woody[WOODY_LEN - size_of_val(&reljump32)..];
	address.copy_from_slice(&reljump32.to_le_bytes());

	let total_len = init.len() + packer_loop.len() + woody.len();
	let mut packer = Vec::with_capacity(total_len);
	packer.extend_from_slice(&init);
	packer.extend_from_slice(&packer_loop);
	packer.extend_from_slice(&woody);

	return packer;
}
