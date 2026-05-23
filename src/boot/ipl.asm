; rio-ipl
; TAB=4

; cyls    equ     0x0a             ; どこまで読み込むか
; 読み込み設定
; SECTORS_TO_READ    equ    4260             ; 全体で読み込むセクタ数 (≒ 2.1MB)
SECTORS_TO_READ    equ    889             ; 全体で読み込むセクタ数。一時的に縮小。
SECTORS_PER_CALL    equ    127              ; 1回のint 0x13で読み込むセクタ数 (127以下にすること)
LOAD_SEG                  equ    0xc20         ; ロード先 segment (-> 物理 0xc200)
START_LBA                 equ    1                  ; 開始 LBA (IPL 直後のセクタ)

        org     0x7c00           ; このプログラムがどこに読み込まれるか

; 以下は標準的なFAT12フォーマットフロッピーディスクのための記述

        jmp     short entry
        db      0x90
        db      "RIO     "      ; ブートセクタの名前を自由に書いて良い(8バイト)
        dw      512             ; 1セクタの大きさ
        db      1               ; クラスタの大きさ
        dw      1               ; FATがどこから始まるか
        db      2               ; FATの個数
        dw      224             ; ルートディレクトリ領域の大きさ
        dw      2880            ; このドライブの大きさ
        db      0xf0            ; メディアのタイプ
        dw      9               ; FAT領域の長さ
        dw      18              ; 1トラックに幾つのセクタがあるか
        dw      2               ; ヘッドの数
        dd      0               ; パーティションを使ってないのでここは必ず0
        dd      2880            ; このドライブの大きさをもう一度かく
        db      0,0,0x29        ; よくわからないけどこの値にしておくといいらしい
        dd      0xffffffff      ; 多分ボリュームのシリアル番号
        db      "RIO-OS     "   ; ディスクの名前(11バイト)
        db      "FAT12   "      ; フォーマットの名前(8バイト)
        ; resb    18            ; とりあえず18バイトあけておく
        times   18 db 0         ; とりあえず18バイトあけておく

; プログラム本体

entry:
        mov     ax, 0           ; レジスタの初期化
        mov     ss, ax
        mov     sp, 0x7c00
        mov     ds, ax
        mov     [boot_drive], dl    ; BIOS から渡されたブートドライブ番号を保存

; ディスクを読む
; DAP の初期固定値（転送先と開始 LBA。読むセクタ数だけループ内で更新）
        mov     word   [dap_buf_off], 0
        mov     word   [dap_buf_seg], LOAD_SEG
        mov     dword [dap_lba_lo], START_LBA
        mov     dword [dap_lba_hi], 0
        mov     word [total_remain], SECTORS_TO_READ

read_loop:
        mov     ax, [total_remain]
        cmp     ax, 0
        je         read_done

        ; 今回読むセクタ数 = min(残り, SECTORS_PER_CALL)
        cmp     ax, SECTORS_PER_CALL
        jbe       .set_count
        mov     ax, SECTORS_PER_CALL
.set_count:
        mov     [dap_sectors], ax

        ; INT 13h AH=42h 呼び出し
        mov     ah, 0x42
        mov     dl, [boot_drive]
        mov     si, dap
        int        0x13
        jc         error         ; エラーが起きたらerrorへ

        ; ポインタ・LBA・残りを進める
        movzx   eax, word [dap_sectors]   ; 実際に読んだセクタ数 (32bit に zero-extend)
        sub       [total_remain], ax             ; 残り -= 読んだ分
        add       [dap_lba_lo], eax             ; LBA += 読んだ分
        shl        ax, 5                                ; ax *= 32 (1 sector = 0x20 segments)
        add       [dap_buf_seg], ax             ; 転送先セグメント += 読んだ分(segment += sectors * 0x20)
        jmp       read_loop

read_done:
        jmp     0xc200

error:
        mov     si, msg
putloop:
        mov     al, [si]
        add     si, 1           ; siに1を足す
        cmp     al, 0
        je      fin
        mov     ah, 0x0e        ; 一文字表示ファンクション
        mov     bx, 15          ; カラーコード
        int     0x10            ; ビデオBIOSの呼び出し
        jmp     putloop
fin:
        hlt                     ; 何かあるまでCPUを停止させる
        jmp     fin
msg:
        db      0x0a, 0x0a      ; 改行を2つ
        db      "LBA load error"
        db      0x0a            ; 改行
        db      0

        ; resb    0x7dfe-$      ; 0x7dfe(32254)まで0x00で埋める
        ; resb    32254           ; 0x7dfe(32254)まで0x00で埋める
; ----- データ領域 -----
        align   4
dap:
        db      0x10               ; +0  DAP size (= 0x10)
        db      0                     ; +1  reserved
dap_sectors:
        dw      0                     ; +2  読むセクタ数 (DAP呼び出し前にここをセットする)
dap_buf_off:
        dw      0                     ; +4  転送先オフセット
dap_buf_seg:
        dw      0                     ; +6  転送先セグメント
dap_lba_lo:
        dd      0                     ; +8  読み込み開始 LBA (下位)
dap_lba_hi:
        dd      0                     ; +12 読み込み開始 LBA (上位

total_remain dw    0          ; 読み込むセクタの残り数
boot_drive    db    0         ; ブートドライブ番号を保存する変数

        times   0x7dfe-0x7c00-($-$$) db 0

        db      0x55, 0xaa

