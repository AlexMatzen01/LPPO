ENTRY(_start)

PHDRS
{
    text PT_LOAD ;
    rodata PT_LOAD ;
    data PT_LOAD ;
    note PT_NOTE ;
}

SECTIONS
{
    /* Bare metal kernel is loaded at the 1 MiB mark. */
    . = 0x100000;

    /* _start must be at 0x100000 so the PVH note references it as the
       kernel entry point. */
    .text.start 0x100000 : {
        KEEP(*(.text.start))
        *(.text.start.*)
    } :text

    .text : {
        *(.multiboot)
        *(.text .text.*)
    } :text

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    _end = .;

    /* PVH note section (XEN_ELFNOTE_PHYS32_ENTRY, type 18) so QEMU's
       -kernel loader recognizes this ELF as a bootable PVH kernel. */
    .note.xen : {
        *(.note.xen)
    } :note

    /* A fixed 16 KiB stack grows downward from _stack_top. */
    . = . + 0x4000;
    _stack_top = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
        *(.note.gnu.build-id)
    }
}
