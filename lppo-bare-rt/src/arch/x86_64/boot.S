.section .text.start, "ax"
.code32
.globl _start

.macro serial_char ch
    movb \ch, %al
    movw $0x3F8, %dx
    outb %al, %dx
.endm

_start:
    cli

    # Set up a temporary stack below the kernel image.
    mov $0x90000, %esp

    # Ensure paging is off before we re-initialise it.
    mov %cr0, %eax
    andl $~0x80000000, %eax
    mov %eax, %cr0

    # Load the GDT (null + 32-bit code + data + 64-bit code descriptors).
    lgdt gdt_desc

    # Enter protected mode.
    mov %cr0, %eax
    orl $0x1, %eax
    mov %eax, %cr0

    # Far jump to the 32-bit code segment to reload CS.
    ljmp $0x08, $protected_mode

.code32
protected_mode:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs

    # Set up the real stack from the linker-provided symbol.
    mov $_stack_top, %esp

    # Clear the direction flag.
    cld

    # ------------------------------------------------------------------
    # Build identity-mapped page tables (4-level, 2 MiB huge pages).
    # Page tables live at fixed physical addresses just below 1 MiB.
    # ------------------------------------------------------------------
    # PML4 at 0x7000, PDPT at 0x8000, PD at 0x9000..0xC000 (4 tables).
    mov $0x0, %eax
    mov $0x7000, %edi
    mov $0x1000, %ecx
    rep stosb                      # zero PML4
    mov $0x8000, %edi
    mov $0x1000, %ecx
    rep stosb                      # zero PDPT
    mov $0x9000, %edi
    mov $0x4000, %ecx
    rep stosb                      # zero four PD pages

    # PML4[0] -> PDPT (0x8000), present + writable.
    movl $0x8007, 0x7000

    # PDPT[0..3] -> PD pages (0x9000..0xC000).
    movl $0x9007, 0x8000
    movl $0xA007, 0x8008
    movl $0xB007, 0x8010
    movl $0xC007, 0x8018

    # PD[gb][i] -> 2 MiB huge page at vaddr == paddr (identity map lower 4 GiB).
    mov $0x9000, %esi              # PD[0] base
    mov $0, %eax                   # gb = 0
map_gb:
    mov $0, %ecx                   # i = 0
map_i:
    mov %eax, %edx
    shl $26, %edx                  # gb * 1 GiB
    mov %ecx, %edi
    shl $21, %edi                  # i * 2 MiB
    add %edi, %edx                 # vaddr = paddr
    orl $0x87, %edx                # present | writable | huge (2 MiB)
    mov %edx, (%esi, %ecx, 8)      # PD[gb][i] = entry
    incl %ecx
    cmp $512, %ecx
    jb map_i
    add $0x1000, %esi              # next PD page
    incl %eax
    cmp $4, %eax
    jb map_gb

    # Enable PAE (CR4.PAE).
    mov %cr4, %eax
    orl $0x20, %eax                # bit 5 = PAE
    mov %eax, %cr4

    # Enable LME (IA32_EFER bit 8).
    movl $0xC0000080, %ecx
    rdmsr
    orl $0x100, %eax               # set LME
    wrmsr

    # Load CR3 with PML4 address.
    movl $0x7000, %eax
    mov %eax, %cr3

    # Enable paging (CR0.PG), clear WP.
    mov %cr0, %eax
    orl $0x80000000, %eax          # PG
    andl $~0x10000, %eax           # clear WP
    mov %eax, %cr0

    # Far jump to 64-bit code segment to enter long mode.
    ljmp $0x18, $long_mode

.code64
long_mode:
    # Segment bases are ignored in 64-bit mode; reload data segments anyway.
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs

    # Jump to the Rust kernel main.
    jmp kmain

    hlt
    jmp long_mode

gdt:
    .quad 0x0000000000000000         # null descriptor
    .quad 0x00CF9A000000FFFF         # 32-bit code segment (read/execute)
    .quad 0x00CF92000000FFFF         # data segment (read/write)
    .quad 0x00A09A000000FFFF         # 64-bit code segment (read/execute)
gdt_end:

gdt_desc:
    .word gdt_end - gdt - 1
    .quad gdt
