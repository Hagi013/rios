pub const ADR_BOOTINFO: u32 = 0x00000ff0;

// public interface
pub struct BootInfo {
    pub cyls: u32,      /* ブートセクタはどこまでディスクを読み込んだのか */
    pub leds: u32,      /* ブートの時のキーボードのLEDの状態 */
    pub vmode: u32,     /* ビデオモード 何ビットカラーか */
    pub scrnx: u16,     /* 画像解像度 */
    pub scrny: u16,     /* 画像解像度 */
    pub vram: u32,      /* vram */
}

impl BootInfo {
    pub fn new() -> Self {
        BootInfo {
            cyls:   unsafe   { *(ADR_BOOTINFO as *const u32) },
            leds:   unsafe   { *((ADR_BOOTINFO + 0x01) as *const u32) },
            vmode:  unsafe   { *((ADR_BOOTINFO + 0x02) as *const u32) },
            scrnx:  unsafe   { *((ADR_BOOTINFO + 0x04) as *const u16) },
            scrny:  unsafe   { *((ADR_BOOTINFO + 0x06) as *const u16) },
            vram:   unsafe   { *((ADR_BOOTINFO + 0x08) as *mut   u32) },
        }
    }

    pub fn set_addr_vram(&mut self, vir_addr: u32) {
        unsafe { *((ADR_BOOTINFO + 0x08) as *mut   u32) = vir_addr };
    }
}
