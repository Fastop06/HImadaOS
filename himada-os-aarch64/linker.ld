OUTPUT_FORMAT(elf64-littleaarch64)
ENTRY(_start)

PHDRS
{
    text    PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    rodata  PT_LOAD    FLAGS((1 << 2)) ;            /* Read only */
    data    PT_LOAD    FLAGS((1 << 1) | (1 << 2)) ; /* Write + Read */
}

SECTIONS
{
    /* Link in the higher half. Limine requires 64KiB (65536) alignment on AArch64. */
    . = 0xffffffff80000000;

    .text ALIGN(65536) : {
        *(.text .text.*)
    } :text

    .rodata ALIGN(65536) : {
        *(.rodata .rodata.*)
    } :rodata

    .data ALIGN(65536) : {
        *(.data .data.*)
    } :data

    .got ALIGN(65536) : {
        *(.got)
        *(.got.plt)
        *(.data.rel.ro .data.rel.ro.*)
    } :data

    .requests ALIGN(65536) : {
        *(.requests)
    } :data

    .bss ALIGN(65536) : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
