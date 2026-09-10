; boot/boot.asm - Stage 1 BIOS MBR Bootloader (512 bytes)
; Loads stage 2 / kernel from disk and jumps to 32-bit protected mode
[BITS 16]
[ORG 0x7C00]

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    mov [BOOT_DRIVE], dl

    ; Print early BIOS message
    mov si, MSG_BOOTING
    call print_string_16

    ; Reset disk system
    xor ax, ax
    mov dl, [BOOT_DRIVE]
    int 0x13

    xor ax, ax
    mov ds, ax
    mov es, ax

    mov word [dap_segment], 0x0800    ; Segment 0x0800 (Physical 0x8000)
    mov word [dap_offset], 0x0000     ; Offset 0x0000
    mov dword [dap_lba_low], 1        ; Start from LBA sector 1 (2nd sector)
    mov dword [dap_lba_high], 0
    mov cx, 28                        ; 28 iterations * 32 sectors = 896 sectors (448 KB)

.read_chunk:
    push cx
    mov si, disk_address_packet
    mov dl, [BOOT_DRIVE]
    mov ah, 0x42                      ; Extended Read Sectors
    int 0x13
    jc disk_error

    ; Advance destination segment by 32 sectors * 512 bytes / 16 = 0x400
    add word [dap_segment], 0x0400
    add dword [dap_lba_low], 32
    pop cx
    loop .read_chunk

    mov si, MSG_LOADED
    call print_string_16

    ; Enable A20 Line (Fast A20 Gate)
    in al, 0x92
    or al, 2
    out 0x92, al

    ; Switch to 32-bit Protected Mode
    cli
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1          ; Enable Protection (PE) bit
    mov cr0, eax

    ; Far jump to 32-bit code segment
    jmp 0x08:protected_mode_entry

disk_error:
    push ax
    mov si, MSG_DISK_ERROR
    call print_string_16
    pop ax
    ; Print AH error code in hex
    mov al, ah
    shr al, 4
    add al, '0'
    cmp al, '9'
    jle .p1
    add al, 7
.p1:
    mov ah, 0x0E
    int 0x10
    cli
    hlt
    jmp $

print_string_16:
    lodsb
    or al, al
    jz .done
    mov ah, 0x0E
    mov bh, 0
    int 0x10
    jmp print_string_16
.done:
    ret

BOOT_DRIVE db 0
MSG_BOOTING db "[BOOT] Starting Rust OS...", 0x0D, 0x0A, 0
MSG_LOADED  db "[BOOT] Loaded kernel image.", 0x0D, 0x0A, 0
MSG_DISK_ERROR db "[BOOT] Disk read error!", 0x0D, 0x0A, 0

align 4
disk_address_packet:
    db 0x10                   ; Packet size = 16 bytes
    db 0                      ; Reserved (0)
    dw 32                     ; Read 32 sectors per chunk (16 KB)
dap_offset:
    dw 0x0000                 ; Destination offset
dap_segment:
    dw 0x0800                 ; Destination segment (Physical 0x8000)
dap_lba_low:
    dd 1                      ; LBA starting sector
dap_lba_high:
    dd 0                      ; High 32 bits of LBA

; 32-bit Temporary GDT
align 8
gdt_start:
    dq 0x0000000000000000 ; Null descriptor
    ; 32-bit Code Segment: Base=0, Limit=0xFFFFF, Granularity=4K, 32-bit, DPL=0
    dw 0xFFFF, 0x0000, 0x9A00, 0x00CF
    ; 32-bit Data Segment: Base=0, Limit=0xFFFFF, Granularity=4K, 32-bit, DPL=0
    dw 0xFFFF, 0x0000, 0x9200, 0x00CF
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

[BITS 32]
protected_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x1FF000

    ; Jump to Stage 2 / 64-bit setup at 0x8000
    jmp 0x8000

; Boot signature
times 510 - ($ - $$) db 0
dw 0xAA55
