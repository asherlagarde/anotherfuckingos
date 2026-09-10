; kernel/src/idt_asm.asm - CPU Exception Entry points
[BITS 64]
global default_exception_handler
global page_fault_handler
global gpf_handler

extern rust_default_exception
extern rust_page_fault
extern rust_general_protection

default_exception_handler:
    push rbp
    mov rbp, rsp
    sub rsp, 8           ; 16-byte stack alignment
    mov rdi, 0xFF        ; Unknown exception
    mov rsi, [rbp + 8]   ; RIP
    call rust_default_exception
    add rsp, 8
    pop rbp
    iretq

gpf_handler:
    ; Stack has: error_code, RIP, CS, RFLAGS, RSP, SS
    push rbp
    mov rbp, rsp

    ; Print early diagnosis to COM1
    mov dx, 0x3F8
    mov al, '#'
    out dx, al
    mov al, 'G'
    out dx, al
    mov al, 'P'
    out dx, al
    mov al, ':'
    out dx, al
    mov rbx, [rbp + 16]  ; RIP
    call print_hex64_com1
    mov dx, 0x3F8
    mov al, 10
    out dx, al

    sub rsp, 8           ; 16-byte stack alignment
    mov rdi, [rbp + 8]   ; error code
    mov rsi, [rbp + 16]  ; RIP
    call rust_general_protection
    add rsp, 8
    pop rbp
    add rsp, 8           ; pop error code
    iretq

page_fault_handler:
    ; Stack has: error_code, RIP, CS, RFLAGS, RSP, SS
    push rbp
    mov rbp, rsp
    sub rsp, 8           ; 16-byte stack alignment
    mov rdi, cr2         ; CR2 contains faulting linear address
    mov rsi, [rbp + 8]   ; error code
    mov rdx, [rbp + 16]  ; RIP
    call rust_page_fault
    add rsp, 8
    pop rbp
    add rsp, 8           ; pop error code
    iretq

print_hex64_com1:
    mov rcx, 16
.loop:
    rol rbx, 4
    mov al, bl
    and al, 0x0F
    add al, '0'
    cmp al, '9'
    jle .out
    add al, 7
.out:
    mov dx, 0x3F8
    out dx, al
    dec rcx
    jnz .loop
    ret
