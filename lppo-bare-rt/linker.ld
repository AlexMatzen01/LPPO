ENTRY(_start)

SECTIONS
{
    /* Bare metal kernel is loaded at the 1 MiB mark. */
    . = 0x100000;

    .text : {
        *(.multiboot)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    _end = .;

    /* A fixed 16 KiB stack grows downward from _stack_top. */
    . = . + 0x4000;
    _stack_top = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
        *(.note.gnu.build-id)
    }
}
