OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

PHDRS
{
  text   PT_LOAD FLAGS(0x5); /* R E */
  rodata PT_LOAD FLAGS(0x4); /* R */
  data   PT_LOAD FLAGS(0x6); /* R W */
}

SECTIONS
{
  . = 0xffffffff80000000;

  .text : ALIGN(4096) {
    *(.text .text.*)
  } :text

  . = ALIGN(4096);
  .rodata : ALIGN(4096) {
    *(.rodata .rodata.*)
  } :rodata

  . = ALIGN(4096);
  .limine_requests : ALIGN(4096) {
    KEEP(*(.limine_requests.start))
    KEEP(*(.limine_requests))
    KEEP(*(.limine_requests.end))
  } :data

  .data : ALIGN(4096) {
    *(.data .data.*)
    *(.got .got.*)
  } :data

  .bss : ALIGN(4096) {
    *(.bss .bss.*)
    *(COMMON)
  } :data
}
