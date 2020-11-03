; rio-os boot
; TAB=4

; [INSTRSET "i486p"]              ; 486の命令まで使いたいという記述

VBEMODE equ     0x105             ; 1024 x 768 x 8bitカラー
; (画面モード一覧)
; 0x100 :   640  x  400 x 8bitカラー
; 0x100 :   640  x  480 x 8bitカラー
; 0x100 :   800  x  600 x 8bitカラー
; 0x100 :   1014 x  768 x 8bitカラー
; 0x105 :   1024 x 768 x 8bitカラー
; 0x107 :   1200 x 1024 x 8bitカラー

INITOS  equ     0x00280000      ; OS本体部分のロード先
DSKCAC  equ     0x00100000      ; ディスクキャッシュの場所
DSKCAC0 equ     0x00008000      ; ディスクキャッシュの場所(リアルモード)

; BOOT_INFO関係
CYLS    equ     0x0ff0          ; ブートセクタが設定する
LEDS    equ     0x0ff1
VMODE   equ     0x0ff2          ; 色すうに関する情報。何ビットカラーか?
SCRNX   equ     0x0ff4          ; 解像度のX
SCRNY   equ     0x0ff6          ; 解像度のY
VRAM    equ     0x0ff8          ; グラフィックバッファの開始番地

        org     0xc200          ; このプログラムがどこに読み込まれるのか

; VBE存在確認

        mov     ax, 0x9000
        mov     es, ax          ; es:diからの512バイトにこのビデオカードで利用できる
        mov     di, 0           ; VBEの情報が書き込まれるので、書き込み場所を指定するために値を入れている
        mov     ax, 0x4f00
        int     0x10
        cmp     ax, 0x004f      ; 上の処理を行うと、axが0x004fに変化することになっている
        jne     scrn320

; VBEのバージョンチェック
        ; jmp     scrn320
        mov     ax, [es:di+4]
        cmp     ax, 0x0200      ; VBEのバージョンが2に満たなかった場合
        jb      scrn320         ; if (ax < 0x200) goto scrn320

; 画面モード情報を得る

        mov     cx, VBEMODE
        mov     ax, 0x4f01
        int     0x10
        cmp     ax, 0x004f
        jne     scrn320

; 画面モード情報の確認

        cmp     BYTE [es:di+0x19], 8    ; 色数
        jne     scrn320
        cmp     BYTE [es:di+0x1b], 4    ; 色の指定方法
        jne     scrn320
        mov     ax, [es:di+0x00]        ; モード属性
        and     ax, 0x0080
        jz      scrn320                 ; モード属性のbit7が0だったので諦める

; 画面モードの切り替え

        mov     bx, VBEMODE+0x4000
        mov     ax, 0x4f02
        int     0x10
        mov     BYTE [VMODE], 8         ; 画面モードをメモする(Rustが参照する)
        mov     ax, [es:di+0x12]        ; Xの解像度
        mov     [SCRNX], ax
        mov     ax, [es:di+0x14]        ; Yの解像度
        mov     [SCRNY], ax
        mov     eax, [es:di+0x28]       ; VRAMの番地
        mov     [VRAM], eax
        jmp     keystatus

scrn320:
        mov     al, 0x13            ; VGAグラフィックス、320x200x8bitカラー
        mov     ah, 0x00
        int     0x10
        mov     BYTE [VMODE], 8     ; 画面モードをメモする(Rustが参照する)
        mov     WORD [SCRNX], 320
        mov     WORD [SCRNY], 200
        mov     DWORD [VRAM], 0x000a0000

; キーボードのLED状態をBIOSに教えてもらう

keystatus:
        mov     ah, 0x02
        int     0x16
        mov     [LEDS], al

; PICが一切の割り込みを受け付けないようにする
;   AT互換機の使用では、PICの初期化をするなら、
;   こいつをCLI前にやっておかないと、たまにハングアップする
;   PICの初期化は後でやる

        mov     al, 0xff
        out     0x21, al
        nop                         ; out命令を連続させるとうまくいかない機種があるらしいので
        out     0xa1, al

        cli                         ; さらにCPUレベルでも割り込み禁止

; CPUから1MB以上のメモリにアクセスできるように、A20GATEを設定

        call    waitkbdout
        mov     al, 0xd1
        out     0x64, al
        call    waitkbdout
        mov     al, 0xdf            ; enable A20
        out     0x60, al
        call    waitkbdout

; プロテクトモードへの行こう

        LGDT    [GDTR0]             ; 暫定のGDTを設定
        mov     eax, cr0
        and     eax, 0x7fffffff     ; bit31を0にする(ページング禁止のため)
        or      eax, 0x00000001     ; bit0を1にする(プロテクトモード移行のため)
        mov     cr0, eax
        jmp     pipelineflush
pipelineflush:
        mov     ax, 1*8             ; 読み書き可能セグメント32bit
        mov     ds, ax
        mov     es, ax
        mov     fs, ax
        mov     gs, ax
        mov     ss, ax

; OS本体の転送

        mov     esi, initos         ; 転送元
        mov     edi, INITOS         ; 転送先
;        mov     ecx, 512*1024*4
        mov     ecx, 2048*1024*4    ; 2MB
        call    memcpy

; ついでにディスクデータも本来の位置へ転送

; 先ずはブートセクタから

        mov     esi, 0x7c00         ; 転送元
        mov     edi, DSKCAC         ; 転送先
        mov     ecx, 512/4
        call    memcpy

; 残り全部

        mov     esi, DSKCAC0+512    ; 転送元
        mov     edi, DSKCAC+512     ; 転送先
        mov     ecx, 0
        mov     cl, BYTE [CYLS]
        imul    ecx, 512*18*2/4     ; シリンダ数からバイト数/4に変換
        sub     ecx, 512/4          ; IPLの分だけ差し引く
        call    memcpy

; asmheadでしなければいけないことは全部出し終わったので
;   後はinitosに任せる

; initosの起動

        mov     ebx, INITOS
        mov     ecx, [ebx+16]
        add     ecx, 3              ; ecx += 3
        shr     ecx, 2              ; ecx /= 4;
        jz      skip                ; 転送すべきものがない
        mov     esi, [ebx+20]       ; 転送元
        add     esi, ebx
        mov     edi, [ebx+12]       ; 転送先
        call    memcpy
skip:
        mov     esp, [ebx+12]       ; スタック初期値
        jmp     DWORD 2*8:0x0000001b    ; .sysのヘッダのjmp命令のある場所へジャンプ(2番目のセグメントの0x001bへジャンプ)

waitkbdout:
        in      al, 0x64
        and     al, 0x02
        jnz     waitkbdout          ; andの結果が0でなければwaitkbdoutへ
        ret

initpaging:
        mov     eax, 0xd7f000
        mov     cr3, eax
        mov     eax, cr0
        or      eax, 0x80000000
        mov     cr0, eax
        ret

memcpy:
        mov     eax, [esi]
        add     esi, 4
        mov     [edi], eax
        add     edi, 4
        sub     ecx, 1
        jnz     memcpy              ; 引き算した結果が0でなければmemcpyへ
        ret
; memcpyはアドレスサイズプリフィクスを入れ忘れなければ、ストリング命令でもかける

        alignb  16, db 0
GDT0:
        ; resb    8                   ; フルセレクタ
        times   8 db 0
        dw      0xffff, 0x0000, 0x9200, 0x00cf  ; 読み書き可能セグメント32bit
        dw      0xffff, 0x0000, 0x9a28, 0x0047  ; 実行可能セグメント32bit(initos用)

        dw      0
GDTR0:
        dw      8*3-1
        dd      GDT0

        alignb  16, db 0
initos:
