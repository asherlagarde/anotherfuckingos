; kernel/src/syscall_asm.asm - Low-level x86_64 syscall entry & exit
[BITS 64]
global syscall_entry
extern syscall_dispatcher

; MSR LSTAR targets this entry point upon 'syscall' from Ring 3
; On entry:
;   RCX = user RIP
;   R11 = user RFLAGS
;   RAX = syscall number
;   RDI = arg1
;   RSI = arg2
;   RDX = arg3
;   R10 = arg4 (Linux passes 4th arg in R10 instead of RCX because syscall clobbers RCX)
;   R8  = arg5
;   R9  = arg6
;   RSP = user RSP
align 16
syscall_entry:
    ; Swap to kernel GS / save user stack
    ; For our OS, we preserve user RSP into a known memory location or register
    mov [rel user_saved_rsp], rsp
    mov rsp, [rel kernel_stack_top]

    ; Push preserved user state onto kernel stack
    push qword [rel user_saved_rsp] ; user RSP
    push r11                        ; user RFLAGS
    push rcx                        ; user RIP
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    ; Prepare Rust syscall_dispatcher call:
    ; fn syscall_dispatcher(num: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64) -> i64
    ; System V AMD64 calling convention:
    ;   rdi = num (from rax)
    ;   rsi = a1  (from rdi)
    ;   rdx = a2  (from rsi)
    ;   rcx = a3  (from rdx)
    ;   r8  = a4  (from r10)
    ;   r9  = a5  (from r8)
    ;   push a6   (from r9)

    push r9       ; arg6 (on stack)
    mov r9, r8    ; arg5
    mov r8, r10   ; arg4
    mov rcx, rdx  ; arg3
    mov rdx, rsi  ; arg2
    mov rsi, rdi  ; arg1
    mov rdi, rax  ; syscall number

    call syscall_dispatcher
    add rsp, 8    ; clean up pushed arg6

    ; Return value from syscall_dispatcher is in RAX

    ; Restore registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    pop rcx       ; restore user RIP
    pop r11       ; restore user RFLAGS
    pop rsp       ; restore user RSP

    ; Return to Ring 3
    sysretq

global jump_to_user_mode
; fn jump_to_user_mode(entry_point: u64, user_sp: u64) -> !
; Sets up IRETQ frame or sysret to jump to user space
jump_to_user_mode:
    ; RDI = entry_point
    ; RSI = user_sp
    cli
    ; Push stack frame for iretq:
    ; SS (0x1B = User Data with RPL 3)
    ; RSP (user_sp)
    ; RFLAGS (0x202 = Interrupts enabled, bit 1 always 1)
    ; CS (0x23 = User Code with RPL 3)
    ; RIP (entry_point)
    push qword 0x1B       ; User Data Selector (0x18 | 3)
    push rsi              ; User Stack Pointer
    push qword 0x202      ; User RFLAGS
    push qword 0x23       ; User Code Selector (0x20 | 3)
    push rdi              ; User Entry Point

    ; Clear general registers
    xor rax, rax
    xor rbx, rbx
    xor rcx, rcx
    xor rdx, rdx
    xor rsi, rsi
    xor rdi, rdi
    xor rbp, rbp
    xor r8, r8
    xor r9, r9
    xor r10, r10
    xor r11, r11
    xor r12, r12
    xor r13, r13
    xor r14, r14
    xor r15, r15

    iretq

section .data
align 8
global user_saved_rsp
global kernel_stack_top
user_saved_rsp: dq 0
kernel_stack_top: dq 0x90000
