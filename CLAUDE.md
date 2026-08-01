# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`rios` is a hobby operating system written in Rust for x86 (32-bit, i686). It boots via a custom FAT32 IPL, a second-stage boot loader, and a Rust kernel crate compiled as a `staticlib` and linked with assembly glue into a flat binary.

## Toolchain

- Rust: nightly **2023-06-05** pinned in `src/rust-toolchain`. Many unstable features are required (`allocator_api`, `asm`, `const_mut_refs`, `alloc_error_handler`, `nonnull_slice_from_raw_parts`, …) — do not bump the toolchain casually.
- Builds use **`cargo xbuild`** (not plain `cargo build`) against the custom target `src/i686-unknown-linux-gnu.json` (soft-float, no SSE/MMX, `code-model = kernel`, `relocation-model = static`, `disable-redzone`, `ld.lld`). Tests use **`cargo xtest`**.
- External tools required on the host: `nasm`, `mtools` (`mcopy`), `dosfstools` (`mkfs.fat`), `qemu-system-i386`, a cross linker invoked as `i686-unknown-linux-gnu-ld`, and (for networking) `brew install tuntap` on macOS.
- `QEMU_PATH` in the Makefile points at a locally-built QEMU (`/Users/haaagiii/Personal/qemu-arm2/qemu/build2`); override it when running elsewhere.

## Common commands

All driven from the Makefile:

- `make asm` — assemble `ipl.bin` and `secondboot.bin` only.
- `make dist/rios-i686-unknown-linux-gnu.img` — full build: assemble boot code, run `cargo xbuild` to produce `librios.a`, link with `src/boot/kernel.ld` + `asmfunc.o` into `kernel.bin`, concatenate with `secondboot.bin` into `.sys`, then `dd`/`mcopy` into a FAT32 image.
- `make qemu` — boot the image under the locally-built QEMU. Uses `sudo` because of tuntap + privileged trace flags. Opens a gdbserver on `tcp::9001` (`-S -gdb tcp::9001` from the `DEBUG` macro), so QEMU starts paused — connect gdb or remove `$(DEBUG)` to run immediately.
- `make qemu-with-usb` — same as `qemu` but attaches the image as IDE instead of `-fda` (needed when exercising the xHCI/USB path).
- `make test` — runs `cargo xtest` inside `src/`.
- `make clean` — wipes `dist/`, `target/`, and also runs `cargo clean`/`xargo clean` in `src/`.
- `make dump` / `make elf` — disassemble / dump program headers of the debug `librios.a` via `objdump` / `greadelf`.

`BUILD_MODE` at the top of the Makefile switches between `debug` and `release` profile outputs under `target/i686-unknown-linux-gnu/`.

## Architecture

The kernel is a single `no_std`, `no_main` static library (`[lib] crate-type = ["staticlib"]`) whose entry point `init_os` (`src/lib.rs`) is called by `secondboot.asm`. The linker script `src/boot/kernel.ld` sets `ENTRY(init_os)` and places `.text` at `0x00280000`, producing a raw binary.

Boot flow:

1. `src/boot/ipl.asm` — FAT32 boot sector; loads `secondboot.bin` off the FAT32 image built by the Makefile.
2. `src/boot/secondboot.asm` — switches to protected mode and jumps to the Rust kernel. Also populates the `BootInfo` consumed by `arch::boot_info`.
3. `src/boot/asmfunc.asm` — assembled separately into `asmfunc.o` and linked alongside `librios.a`; exposes the `io_*`, interrupt-handler trampolines, and other routines imported via `arch::asmfunc`.
4. `init_os` (`src/lib.rs`) initializes PIC → GDT/IDT → paging → graphics → heap → DMA heap → window manager → timer → input (PS/2 kbd+mouse) → PCI scan → NIC (`e1000`) → xHCI, then enters a poll loop that services received Ethernet frames and keyboard/mouse events.

Module layout (top-level `src/`):

- `arch/` — x86-specific: `dsctbl` (GDT/IDT), `paging`, `pic`, `timer`, `keyboard`, `mouse`, `graphic` (VGA framebuffer + `Printer` used everywhere for on-screen debug), `boot_info`, `hankaku` (bitmap font), `asmfunc` (Rust side of asm glue).
- `allocator/` — custom `#[global_allocator]`. `LockedHeap` wraps a `Heap` composed of **seven fixed-size slab allocators** (64 B … 4096 B) plus a linked-list fallback for `>4 KiB`. `Heap::layout_to_allocator` picks the slab by size/align. Initialized in `init_heap` (`src/lib.rs`) with the hard-coded range `0x00800000`–`0x3e970000`; also exposed to the paging layer via `set_kernel_table_allocator`.
- `memory/` — `dma` sets up a separate DMA-capable heap (`DMA_ALLOCATOR`) used by drivers that need physically-contiguous buffers; `volatile` provides volatile MMIO helpers.
- `spin/`, `sync/` — `Mutex`/`MutexGuard`/`Once` replacements (no OS deps) and a simple FIFO `queue` used by interrupt handlers to hand data to the main loop.
- `util/` — `lazy_static` reimplementation, intrusive `linked_list`, bit helpers.
- `drivers/bus/pci.rs` — PCI config-space scan, BAR dump, bus-master enable, NIC bring-up helpers called from `init_os`.
- `drivers/net/` — in-tree TCP/IP-ish stack built on top of the e1000 driver: `ethernet` framing, `arp`, `ip`, `icmp`, `udp`, `dhcp`, plus `net_util`. The main loop dispatches parsed frames to these modules based on EtherType/IP protocol.
- `drivers/usb/xhci.rs` — xHCI host-controller init invoked by `xhci::initialize_xhci()`.
- `window/` — simple compositor (`Window`, `WindowsManager`) used for the mouse cursor and text output.

Heap invariants enforced at runtime (see `Heap::new`): the heap start must be 4 KiB-aligned and the total size must be a multiple of `NUM_OF_SLABS * MIN_SLAB_SIZE` (= 8 × 4096 = 32 KiB). If you change the heap range in `init_heap`, keep both invariants.

Keyboard/mouse interrupts push into queues via PIC handlers; the main loop drains them with `keyboard::is_existing()`/`get_data()` and `mouse::is_existing()`/`get_data()`. Certain keycodes in `init_os` are bound to network actions (ARP probe, ICMP, DHCP discover, test-frame send) — this is debug plumbing, not a user-facing feature.

## Notes / gotchas

- The `i686-unknown-linux-gnu.json` target disables SSE and enables soft-float; do not introduce `f32`/`f64` math without checking what that implies, and do not assume SIMD intrinsics are available.
- Panics print via `Graphic::putfont_asc` / `Printer` to fixed screen coordinates — there is no serial log by default.
- `asmfunc.o` is rebuilt from `src/boot/asmfunc.asm` by a Makefile rule into `target/.../$(BUILD_MODE)/asmfunc.o`; if you add new asm symbols, make sure they are referenced from Rust so LLD keeps them.
- The `qemu` target runs with `-S` (CPU stopped at entry) and a gdbserver on port 9001. Connect with `gdb` using the included `.gdbinit`, or remove `$(DEBUG)` from the recipe to auto-start.
- `cargo xbuild`/`xtest` come from `cargo-xbuild` (installed in `Dockerfile.arm`). Plain `cargo build` will not work because the sysroot needs rebuilding for this target.
