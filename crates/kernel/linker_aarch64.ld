ENTRY(_start)

SECTIONS {
  . = 0x40000000;
  __kernel_start = .;

  .text : ALIGN(16) {
    *(.text._start)
    *(.text*)
  }

  .rodata : ALIGN(16) {
    *(.rodata*)
  }

  .data : ALIGN(16) {
    *(.data*)
  }

  .bss : ALIGN(16) {
    __bss_start = .;
    *(.bss*)
    *(COMMON)
    __bss_end = .;
  }

  . = ALIGN(16);
  __kernel_end = .;
}
