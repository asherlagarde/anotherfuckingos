/* kernel/linker.ld - 64-bit Kernel Linker Script */
ENTRY(_start64_setup)
OUTPUT_FORMAT("elf64-x86-64")

SECTIONS
{
    . = 0x8000;

    .boot : {
        *(.boot)
        *entry64.o(.text)
    }

    . = ALIGN(4096);
    .text : {
        *(.text .text.*)
    }

    . = ALIGN(4096);
    .rodata : {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4096);
    .data : {
        *(.data .data.*)
    }

    . = ALIGN(4096);
    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /DISCARD/ : {
        *(.comment)
        *(.note.*)
        *(.eh_frame)
    }
}
