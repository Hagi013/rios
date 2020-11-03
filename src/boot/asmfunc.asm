
bits 32

;global far_jmp
global asm_inthandler02, asm_inthandler04, asm_inthandler05, asm_inthandler06, asm_inthandler07, asm_inthandler08, asm_inthandler0a, asm_inthandler0b, asm_inthandler0c, asm_inthandler0d
global asm_inthandler0e, asm_inthandler10, asm_inthandler11, asm_inthandler12, asm_inthandler13, asm_inthandler14, asm_inthandler20, asm_inthandler21, asm_inthandler27, asm_inthandler2c
extern non_maskable_interrupt_handler, overflow_handler, bounds_check_handler, undefined_operation_code_instruction_handler, no_coprocessor_handler, double_fault_handler, invalid_tss_handler
extern segment_not_present_handler, stack_segment_fault_handler, general_protection_error_handler, page_fault_handler, coprocessor_error_handler, alignment_check_error_handler, machine_check_handler
extern simd_fpu_exception_handler
extern inthandler20, inthandler21, inthandler27, inthandler2c

section .text

;farjmp:     ; farjmp(eip: u32, cs: u32)
;    jmp far [esp+4]     ; eip, cs. JMP FARはfar jmpさせるための命令。CPUは指定番地からまず4バイト読み込んんで、その値をEIPに入れる
;    ret                 ; さらにその隣の2バイトも読み込んでCSに入れる

asm_inthandler02:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call non_maskable_interrupt_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler04:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call overflow_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler05:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call bounds_check_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler06:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call undefined_operation_code_instruction_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler07:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call no_coprocessor_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler08:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call double_fault_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler0a:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call invalid_tss_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler0b:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call segment_not_present_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler0c:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call stack_segment_fault_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler0d:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call general_protection_error_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler0e:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call page_fault_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler10:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call coprocessor_error_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler11:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call alignment_check_error_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler12:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call machine_check_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler13:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call simd_fpu_exception_handler
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler20:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call inthandler20
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler21:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call inthandler21
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler27:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call inthandler27
    pop eax
    popad
    pop ds
    pop es
    iretd

asm_inthandler2c:
    push es
    push ds
    pushad
    mov eax, esp
    push eax
    mov ax, ss
    mov ds, ax
    mov es, ax
    call inthandler2c
    pop eax
    popad
    pop ds
    pop es
    iretd