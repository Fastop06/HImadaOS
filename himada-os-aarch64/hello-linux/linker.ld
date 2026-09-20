ENTRY(_start)
SECTIONS {
    . = 0x400000;
    .text : { *(.text*) }
    . = ALIGN(65536);
    .rodata : { *(.rodata*) }
    . = ALIGN(65536);
    .got : { *(.got*) *(.got.plt*) *(.data.rel.ro*) }
    . = ALIGN(65536);
    .data : { *(.data*) }
    . = ALIGN(65536);
    .bss : { *(.bss*) *(COMMON) }
    /DISCARD/ : { *(.eh_frame*) *(.comment) }
}



