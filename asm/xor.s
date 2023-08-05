SECTION .text
	GLOBAL xor_cipher

xor_cipher:
	mov rax, rdi
	add rdi, rsi
loop:
	xor byte [rax], 0x61
	add	rax, 1
	cmp	rax, rdi
	jne	loop
