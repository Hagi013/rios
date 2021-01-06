/* 入力オブジェクトファイル */

/* 出力オブジェクトファイル */

/* 出力フォーマット */
OUTPUT_FORMAT(binary);
/* OUTPUT_FORMAT("elf32-i386") */
OUTPUT_ARCH(i386);

ENTRY(init_os)

/* セクション */
SECTIONS
{
    /* .textセクションのメモリ開始位置 */
    .text 0x0 :
    /* .textセクションのファイル上の開始位置 */
    AT(0x00280000)
    {
        /* textセクション全部を含む */
        *(.text.init_os)
        *(.text*)
        KEEP(*(.text*))
    }

    /* dataセクションのメモリ開始位置 */
    .data LOADADDR(.text) + SIZEOF(.text) :
    /* .dataセクションのファイル上の開始位置 */
    AT(LOADADDR(.text) + SIZEOF(.text)) SUBALIGN(4)
    {
        *(.data)
        *(.rodata*)
    }

    /* .bssセクションのメモリ開始位置(.dataセクションの終端から) */
    .bss LOADADDR(.data) + SIZEOF(.data) :
    /* .bssセクションのファイル上の開始位置 */
    /* AT(LOADADDR(.data) + SIZEOF(.data)) SUBALIGN(4) */
    {
        *(.bss)
        *(.bss*)
    }

    /* 上記以外のセクションは無視する */
    /DISCARD/ : {*(*)}
}
