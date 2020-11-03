; rio-ipl
; TAB=4

cyls    equ     0x0a             ; どこまで読み込むか

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

; ディスクを読む

        mov     ax, 0x0820
        mov     es, ax
        mov     ch, 0           ; シリンダ0
        mov     dh, 0           ; ヘッド0
        mov     cl, 2           ; セクタ2
readloop:
        mov     si, 0           ; 失敗回数を数えるレジスタ
retry:
        mov     ah, 0x02        ; AH=0x20 : ディスク読み込み
        mov     al, 1           ; 1セクタ
        mov     bx, 0
        mov     dl, 0x00        ; Aドライブ
        int     0x13            ; ディスクBIOS呼び出し
        jnc     next            ; エラーが起きなければnextへ
        add     si, 1           ; siに1を足す
        cmp     si, 5           ; siと5を比較
        jae     error           ; si >= 5 だったらerrorへ
        mov     ah, 0x00
        mov     dl, 0x00        ; Aドライブ
        int     0x13            ; ドライブのリセット
        jmp     retry
next:
        mov     ax, es          ; アドレスを0x200進める
        add     ax, 0x0020
        mov     es, ax          ; add ex, 0x0020という命令がないのでこうしている
        add     cl, 1           ; clに1を足す
        cmp     cl, 18          ; clと18を比較
        jbe     readloop        ; cl <= 18 だったらreadloopへ
        mov     cl, 1
        add     dh, 1
        cmp     dh, 2
        jb      readloop        ; dh < 2 だったらreadloopへ
        mov     dh, 0
        add     ch, 1
        cmp     ch, cyls
        jb      readloop        ; ch < cyls だったらreadloopへ

; 読み終わったのでプログラムを実行する

        mov     [0x0ff0], ch    ; IPLがどこまで読んだのかをメモ
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
        db      "load error"
        db      0x0a            ; 改行
        db      0

        ; resb    0x7dfe-$      ; 0x7dfe(32254)まで0x00で埋める
        ; resb    32254           ; 0x7dfe(32254)まで0x00で埋める
        times   0x7dfe-0x7c00-($-$$) db 0

        db      0x55, 0xaa

