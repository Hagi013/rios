BUILD_DIR := ./dist
KERNEL_DIR := ./src
TARGET_DIR := ./target
TARGET_ARCH_i686 := i686-unknown-linux-gnu
BUILD_NAME := rios-$(TARGET_ARCH_i686)
QEMU_ARCH_i686 := i386
QEMU_PATH ?= /Users/haaagiii/Personal/qemu-arm2/qemu/build

BUILD_MODE=debug
#BUILD_MODE=release

DEBUG := -S -gdb tcp::9001
DEBUG_MODE := -D ./qemu.log

QEMUNET = -netdev type=tap,id=net0,ifname=tap0,script=./tuntap-up,downscript=./tuntap-down \
		  -device e1000,netdev=net0 \
		  -object filter-dump,id=f1,netdev=net0,file=dump.dat

TRACE=-trace enable=true


asm:	$(BUILD_DIR)/ipl.bin \
 	$(BUILD_DIR)/secondboot.bin

# make image file
$(BUILD_DIR)/$(BUILD_NAME).img: $(BUILD_DIR)/ipl.bin $(BUILD_DIR)/$(BUILD_NAME).sys Makefile
	mformat -f 1440 -C -B $(BUILD_DIR)/ipl.bin -i $(BUILD_DIR)/$(BUILD_NAME).img ::
	mcopy -i $(BUILD_DIR)/$(BUILD_NAME).img $(BUILD_DIR)/$(BUILD_NAME).sys ::

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
#	sudo qemu-system-$(QEMU_ARCH_i686) -m 4096 \
#		-rtc base=localtime \
#		-vga std \
#		-fda $(BUILD_DIR)/$(BUILD_NAME).img \
#		-monitor stdio \
#		-net nic -netdev tap,id=net0,ifname=tap0,script=/Users/hagi/Personal/rios/tuntap-up,downscript=./tuntap-down \
# 		-net nic,model=rtl8139,macaddr=ae:de:48:00:33:01 -netdev tap,id=net0,ifname=tap0,script=no,downscript=no -usb  \
#		-device e1000,netdev=net0 \
#		$(DEBUG)
#	sudo qemu-system-$(QEMU_ARCH_i686) -m 4096 \
# 	sudo /Users/hagi/Personal/qemu-v5.1.0/qemu-system-$(QEMU_ARCH_i686) -m 4096 \
#	sudo /opt/local/bin/qemu-system-$(QEMU_ARCH_i686) -m 4096 \

	sudo $(QEMU_PATH)/qemu-system-$(QEMU_ARCH_i686) -m 4096 \
		-rtc base=localtime \
		-vga std \
		-M smm=off \
		-d trace:ps2_keyboard_event,trace:ps2_kbd_init,trace:ps2_mouse_send_packet \
		-fda $(BUILD_DIR)/$(BUILD_NAME).img \
		-monitor stdio \
		$(DEBUG) \
		$(DEBUG_MODE) \
		-device e1000,netdev=net0 -netdev user,id=net0 \
		-object filter-dump,id=f1,netdev=net0,file=dump.dat \
		$(TRACE)
#		-d trace:pic_set_irq,trace:pic_interrupt,trace:ps2_keyboard_event \
#		-d trace:ps2_keyboard_event,int \
#		$(QEMUNET) \
		# -boot n \
		# -device vmxnet3,netdev=net0 \

qemu2:
	sudo /Users/hagi/Personal/qemu/build/qemu-system-$(QEMU_ARCH_i686) -m 4096 \
		-rtc base=localtime \
		-vga std \
		-fda $(BUILD_DIR)/$(BUILD_NAME).img \
		-monitor stdio \
		$(UBUNTU_QEMUNET2) \
		$(DEBUG) \
		$(DEBUG_MODE)

UBUNTU_QEMUNET = -netdev type=tap,id=net0,ifname=tap0,script=./tuntap-ubuntu-up,downscript=./tuntap-ubuntu-down \
		  -device e1000,netdev=net0 \
		  -object filter-dump,id=f1,netdev=net0,file=dump.dat
ubuntu:
	sudo touch dump.dat && sudo chmod 777 dump.dat && \
	sudo /home/haaagiii/qemu/build/qemu-system-$(QEMU_ARCH_i686) -m 4096 \
		-rtc base=localtime \
		-vga std \
		-fda $(BUILD_DIR)/$(BUILD_NAME).img \
		-monitor stdio \
		$(UBUNTU_QEMUNET) \
		$(DEBUG) \
		$(DEBUG_MODE)

ubuntu2:
	sudo touch dump.dat && sudo chmod 777 dump.dat && \
	sudo /home/haaagiii/qemu/build/qemu-system-$(QEMU_ARCH_i686) -m 4096 \
		-rtc base=localtime \
		-vga std \
		-fda $(BUILD_DIR)/$(BUILD_NAME).img \
		-monitor stdio \
		$(UBUNTU_QEMUNET2) \
		$(DEBUG) \
		$(DEBUG_MODE)

ubuntu3:
	sudo touch dump.dat && sudo chmod 777 dump.dat && \
	sudo /home/haaagiii/qemu/build/qemu-system-$(QEMU_ARCH_i686) -m 4096 \
		-rtc base=localtime \
		-vga std \
		-fda $(BUILD_DIR)/$(BUILD_NAME).img \
		-monitor stdio \
		$(UBUNTU_QEMUNET5) \
		$(DEBUG) \
		$(DEBUG_MODE)


clean:
	rm -rf $(BUILD_DIR)/*
	rm -rf ./target
	cd $(KERNEL_DIR) && cargo clean
	cd $(KERNEL_DIR) && xargo clean

od:
	od $(BUILD_DIR)/$(BUILD_NAME).img -t x1z -A x

test:
	cd ${KERNEL_DIR}; set RUST_BACKTRACE=1; `which cargo` xtest

dump: target2
	# gobjdump -d -S -M intel ./target2/i686-unknown-linux-gnu/debug/librios.a > rios.obj
	objdump -d -S -M intel ./target/i686-unknown-linux-gnu/debug/librios.a > rios.obj