; boot/entry64.asm - Stage 2: Long Mode setup (64-bit), Paging, and Rust Entry
[BITS 32]
section .boot
global _start64_setup
_start64_setup:
    ; Set up 4-level paging (PML4 at 0x1000, PDPT at 0x2000, PD at 0x3000, PT at 0x4000)
    ; Identity map the first 16 MB of physical memory using 2MB or 4KB pages
    ; Zero out paging tables from 0x1000 to 0x5000 (16KB)
    mov edi, 0x1000
    xor eax, eax
    mov ecx, 4096
    rep stosd

    ; PML4[0] -> points to PDPT at 0x2000 (Present | Writable | User)
    mov dword [0x1000], 0x2000 | 0x07
    ; PDPT[0] -> points to PD at 0x3000 (Present | Writable | User)
    mov dword [0x2000], 0x3000 | 0x07
    ; PDPT[1] -> points to second PD at 0x5000 (for higher identity mapping)
    mov dword [0x2008], 0x5000 | 0x07

    ; Populate first PD (0x3000) with 2MB huge pages (mapping 0MB to 1024MB = 1GB)
    ; 512 entries * 2MB = 1GB
    mov edi, 0x3000
    mov eax, 0x00000087   ; Present | Writable | User | Huge Page (2MB)
    mov ecx, 512
.map_pd_entries:
    mov [edi], eax
    mov dword [edi + 4], 0
    add eax, 0x200000     ; 2MB next page
    add edi, 8
    loop .map_pd_entries

    ; Load CR3 with PML4 address
    mov eax, 0x1000
    mov cr3, eax

    ; Enable Physical Address Extension (PAE) and SSE (OSFXSR | OSXMMEXCPT) in CR4
    mov eax, cr4
    or eax, (1 << 5) | (1 << 9) | (1 << 10)
    mov cr4, eax

    ; Enable Long Mode in EFER MSR (0xC0000080)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8        ; LME (Long Mode Enable)
    wrmsr

    ; Enable Paging in CR0 and configure Coprocessor (clear EM, set MP)
    mov eax, cr0
    and eax, ~(1 << 2)    ; Clear CR0.EM (Emulation)
    or eax, (1 << 31) | (1 << 1) | 1 ; PG (Paging), MP (Monitor Coprocessor), and PE (Protection)
    mov cr0, eax

    ; Load 64-bit GDT
    lgdt [gdt64_descriptor]

    ; Far jump to 64-bit Long Mode code segment
    jmp 0x08:long_mode_start

[BITS 64]
long_mode_start:
    ; Reset data segments to 0x10 (Kernel Data)
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Send 'K' to serial COM1 (0x3F8) for early stage-2 diagnosis
    mov dx, 0x3F8
    mov al, 'K'
    out dx, al
    mov al, 10
    out dx, al

    ; Setup 64-bit Kernel Stack at 0x1FF000 (1.5 MB free stack room)
    mov rsp, 0x1FF000

    ; Jump to Rust kernel entry point
    extern kernel_main
    call kernel_main

    ; If kernel_main returns, halt
.halt:
    cli
    hlt
    jmp .halt

; 64-bit GDT
align 16
gdt64_start:
    dq 0x0000000000000000 ; Null descriptor (0x00)
    ; Kernel Code 64-bit: DPL=0, L=1, P=1, S=1, Type=10 (Exec/Read) (0x08)
    dq 0x00AF9A000000FFFF
    ; Kernel Data 64-bit: DPL=0, P=1, S=1, Type=2 (Read/Write) (0x10)
    dq 0x00CF92000000FFFF
    ; User Data 64-bit: DPL=3, P=1, S=1, Type=2 (Read/Write) (0x18)
    dq 0x00CFF2000000FFFF
    ; User Code 64-bit: DPL=3, L=1, P=1, S=1, Type=10 (Exec/Read) (0x20)
    dq 0x00AFFB000000FFFF
gdt64_end:

gdt64_descriptor:
    dw gdt64_end - gdt64_start - 1
    dd gdt64_start
