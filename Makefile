BUILD_DIR := ./dist
KERNEL_DIR := ./src
TARGET_DIR := ./target
TARGET_ARCH_i686 := i686-unknown-linux-gnu
BUILD_NAME := rios-$(TARGET_ARCH_i686)
QEMU_ARCH_i686 := i386

BUILD_MODE=debug
#BUILD_MODE=release

DEBUG := -S -gdb tcp::9001

asm:	$(BUILD_DIR)/ipl.bin \
 	$(BUILD_DIR)/secondboot.bin

# make image file
$(BUILD_DIR)/$(BUILD_NAME).img: $(BUILD_DIR)/ipl.bin $(BUILD_DIR)/$(BUILD_NAME).sys Makefile
	/opt/homebrew-x86_64/bin/mformat -f 1440 -C -B $(BUILD_DIR)/ipl.bin -i $(BUILD_DIR)/$(BUILD_NAME).img ::
	/opt/homebrew-x86_64/bin/mcopy -i $(BUILD_DIR)/$(BUILD_NAME).img $(BUILD_DIR)/$(BUILD_NAME).sys ::

$(BUILD_DIR)/$(BUILD_NAME).sys: $(BUILD_DIR)/kernel.bin $(BUILD_DIR)/secondboot.bin
	cat $(BUILD_DIR)/secondboot.bin $(BUILD_DIR)/kernel.bin > $(BUILD_DIR)/$(BUILD_NAME).sys

$(BUILD_DIR)/kernel.bin: $(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/librios.a $(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/asmfunc.o $(KERNEL_DIR)/boot
#	$(TARGET_ARCH_i686)-ld --print-gc-sections --gc-sections -t -nostdlib -Tdata=0x00310000 -T $(KERNEL_DIR)/boot/kernel.ld -o $(BUILD_DIR)/kernel.bin $(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/asmfunc.o --library-path=$(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE) -lrios -Map $(BUILD_DIR)/kernel.map --verbose
	$(TARGET_ARCH_i686)-ld -nostdlib -T $(KERNEL_DIR)/boot/kernel.ld -o $(BUILD_DIR)/kernel.bin $(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/asmfunc.o --library-path=$(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE) -lrios -Map $(BUILD_DIR)/kernel.map --verbose

$(BUILD_DIR)/ipl.bin: $(KERNEL_DIR)/boot
	nasm -f bin -o $(BUILD_DIR)/ipl.bin $(KERNEL_DIR)/boot/ipl.asm -l $(BUILD_DIR)/ipl.lst

$(BUILD_DIR)/secondboot.bin: $(KERNEL_DIR)/boot
	nasm -f bin -o $(BUILD_DIR)/secondboot.bin $(KERNEL_DIR)/boot/secondboot.asm -l $(BUILD_DIR)/secondboot.lst

#kernel
$(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/librios.a: ./$(KERNEL_DIR)/$(TARGET_ARCH_i686).json Cargo.toml $(KERNEL_DIR)/*.rs
	cd ${KERNEL_DIR}; RUST_TARGET_PATH=$(PWD); set RUST_BACKTRACE=1;rustup run nightly `which cargo` xbuild --target $(TARGET_ARCH_i686).json -v

$(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/%.o: $(KERNEL_DIR)/boot
	nasm -f elf32 $(KERNEL_DIR)/boot/asmfunc.asm -o $(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/$*.o -l $(TARGET_DIR)/$(TARGET_ARCH_i686)/$(BUILD_MODE)/$*.lst

qemu:
	qemu-system-$(QEMU_ARCH_i686) -m 4096 -rtc base=localtime -vga std -fda $(BUILD_DIR)/$(BUILD_NAME).img -monitor stdio $(DEBUG)

clean:
	rm -rf $(BUILD_DIR)/*
	rm -rf ./target
	cd $(KERNEL_DIR) && cargo clean
	cd $(KERNEL_DIR) && xargo clean

od:
	od $(BUILD_DIR)/$(BUILD_NAME).img -t x1z -A x

test:
	cd ${KERNEL_DIR}; set RUST_BACKTRACE=1; `which cargo` xtest
