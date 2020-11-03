/* 入力オブジェクトファイル */

/* 出力オブジェクトファイル */

ENTRY(init_os)

/* 出力フォーマット */
OUTPUT_FORMAT("binary")
/* OUTPUT_FORMAT("elf32-i386") */

/* セクション */
SECTIONS{
    /* .hrbオブジェクトファイルのフォーマットに従い、ヘッダを定義 */
    .head 0x0 :
    {
        LONG((ADDR(.bss) + SIZEOF(.bss) + 0xfff) & ~ 0xfff)
        BYTE(0x48) BYTE(0x61) BYTE(0x72) BYTE(0x69)
        LONG(0x0)
        LONG(ADDR(.data))
        LONG(SIZEOF(.data))
        LONG(LOADADDR(.data))
        LONG(0xe9000000)
        LONG(init_os - 0x20)
        LONG((ADDR(.bss) + SIZEOF(.bss) + 0xf) & ~ 0xf)
    }

    /* .textセクションのメモリ開始位置 */
    .text ADDR(.head) + SIZEOF(.head) : SUBALIGN(4)
    {
        /* textセクション全部を含む */
        *(.text.init_os)
        *(.text*)
        KEEP(*(.text*))
    }

    /* dataセクションのメモリ開始位置 */
    /* .data 0x00000400: */
    .data 0x310000:
    /* .data */

    /* .dataセクションのファイル上の開始位置 */
    AT(LOADADDR(.text) + SIZEOF(.text)) SUBALIGN(4)
    {
        *(.data)
        *(.rodata*)
    }

    /* .bssセクションのメモリ開始位置(.dataセクションの終端から) */
    .bss :

    /* .bssセクションのファイル上の開始位置 */
    AT(LOADADDR(.data) + SIZEOF(.data)) SUBALIGN(4)
    {
        *(.bss)
        *(.bss*)
    }

    /* 上記以外のセクションは無視する */
    /DISCARD/ : {*(*)}
}
