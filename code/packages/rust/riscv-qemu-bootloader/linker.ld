OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
  RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128K
}

SECTIONS
{
  . = ORIGIN(RAM);

  .text : ALIGN(4) {
    KEEP(*(.text._start))
    *(.text .text.*)
  } > RAM

  .rodata : ALIGN(8) {
    *(.rodata .rodata.*)
  } > RAM

  .data : ALIGN(8) {
    PROVIDE(__global_pointer$ = . + 0x800);
    *(.sdata .sdata.*)
    *(.data .data.*)
  } > RAM

  .bss (NOLOAD) : ALIGN(8) {
    __bss_start = .;
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(8);
    __bss_end = .;
  } > RAM

  . = ALIGN(16);
  __stack_bottom = .;
  . += 16K;
  __stack_top = .;

  /DISCARD/ : {
    *(.eh_frame .eh_frame.*)
  }
}
