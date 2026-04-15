.section .text._start,"ax"
.globl _start

_start:
    la sp, __stack_top
    call bootloader_main

1:
    j 1b
