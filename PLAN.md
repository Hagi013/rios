# rios 開発ロードマップ

自作OS「rios」を汎用OS(Unix Like)方向に育てるための開発計画。
長期的にはユーザーモード・プロセス・FSを持つ本格的なOSを目指しつつ、
当面はネットワーク機能(TCP → SSH/TLS)を優先して育てる。
xHCI/USBはフェーズ3で再開する(既知バグあり、本文書末尾参照)。

## 現状サマリ(2026-08時点)

- 32bit i686、自作FAT32ブートローダ(IPL → secondboot → プロテクトモード)、フラットバイナリを 0x00280000 にロード
- 実装済み: GDT/IDT、PIC、タイマー、2段ページング、自作アロケータ(slab 7段 + linked list)、DMAヒープ、自作 spin::Mutex/Once、PCI、e1000、Ethernet/ARP/IP/ICMP/UDP/DHCP、ウィンドウ/マウス(PS/2)、割り込み→キュー→メインループ(hlt)
- 未実装: TCP、シリアルログ、ユーザーモード、プロセス/スケジューラ、FS、virtio、64bit
- xHCI: 初期化シーケンス前半のみ(進捗15〜20%)、呼び出しはコメントアウト中
- ツールチェイン: nightly-2023-06-05 固定 + cargo xbuild(非推奨ツール、フェーズ0で更新)

---

## フェーズ0: 足場固め(約10h)

### 0-1. シリアル(COM1)ログ実装 【最優先・2〜4h】

- [x] UART 16550 の初期化(COM1 = 0x3F8、115200bps 8N1)
- [x] `serial_print!` / `serial_println!` マクロ
- [x] 既存の画面print系デバッグと併存させる(置き換えは不要、追加でよい)
- [x] panicハンドラからシリアルにも出力
- [x] QEMU側: `-serial stdio`(または `-serial file:serial.log`)を起動スクリプトに追加

受け入れ基準: カーネル起動ログとpanicメッセージがホストのターミナル/ファイルで読める。

### 0-2. ツールチェイン近代化(5〜10h)

- [ ] cargo xbuild を廃止し `cargo build -Z build-std=core,alloc` へ移行
- [ ] rust-toolchain を現行nightlyへ更新
- [ ] 使用中のunstable featureの棚卸しと修正(コンパイルエラー駆動で潰す)
- [ ] Makefile / ビルドスクリプト / Dockerfile.amd64 の追随
- [ ] 既存機能(ネットワーク、GUI、マウス)のリグレッション確認

注意: 一括更新で壊れた場合、中間バージョンのnightlyを経由する段階更新も検討。
作業前にブランチを切ること。

---

## フェーズ1: TCP(15〜25h)

### 設計原則: sans-I/O

TCP状態機械は純粋ロジックとして実装し、ドライバ/割り込みから分離する。
「入力バイト列+現在状態 → 新状態+出力バイト列」の形にし、ホスト側で
`cargo test` 可能にする。プロトコル部は `no_std` の別クレート(例: `rios-net`)
に切り出すことを推奨(smoltcpと同じ設計思想)。

### 1-1. 最小TCP実装

- [ ] プロトコルクレートの分離(workspace化)
- [ ] TCPヘッダのparse/serialize + チェックサム(擬似ヘッダ含む)
- [ ] 状態機械: LISTEN / SYN_RCVD / ESTABLISHED / FIN_WAIT_1 / FIN_WAIT_2 / CLOSE_WAIT / LAST_ACK / TIME_WAIT(受動オープンのみ、能動オープンは後回し可)
- [ ] 3-wayハンドシェイク(ISN生成は当面タイマーベースで可、SSH実装時にCSPRNG化)
- [ ] 受信: 順序通りセグメントの受理とACK返却(順序入れ替えバッファは後回し可、順序外は破棄+ACKで最低限動く)
- [ ] 送信: 再送タイマー(固定RTOの単純再送でよい)、相手ウィンドウの尊重
- [ ] 切断: FIN/ACK処理、TIME_WAIT
- [ ] 複数コネクション対応((src ip, src port, dst port) でのdemux)
- [ ] 既存メインループ(pollベース)との統合

### 1-2. 中間ゴール①: HTTPを返す

- [ ] TCP上に最小HTTP/1.0応答(固定レスポンスでよい)
- [ ] ホストから `curl http://<riosのIP>:8080/` が成功する

### 1-3. 動作確認(実装と並行して行う)

ユニットテスト(ホスト側):
- 状態遷移、チェックサム、シーケンス番号演算(wrap-around含む)を cargo test で

観測:
- `sudo tcpdump -i tap0 -w rios.pcap` を常時録取、Wiresharkで確認
  (再送・シーケンス異常・チェックサム不正は自動で色付けされる)
- QEMUの `-object filter-dump` も代替として可

段階別合格テスト:
1. `nc <ip> 8080` → pcapでSYN/SYN-ACK/ACKのシーケンス番号が正しい
2. `echo hello | nc <ip> 8080` → シリアルログに受信データ、正しいACK番号
3. エコーサーバ化して往復 → `curl` でHTTP応答
4. 切断時のFIN交換とTIME_WAIT遷移をpcapで確認
5. `nc` 2本同時で混線しない

異常系(正常系完了後):
- `sudo tc qdisc add dev tap0 root netem loss 20% delay 100ms duplicate 5%`
  の状態で `curl` が成功すること(終了後 `tc qdisc del dev tap0 root`)
- Scapyでウィンドウ外シーケンス/突然のRST/half-open放置を注入して落ちないこと
- `ip link set tap0 mtu 576` でセグメント分割動作の確認

---

## フェーズ2: SSH(25〜40h)

### 2-1. 乱数源

- [ ] CPUID で RDRAND 対応確認(QEMU設定によっては要 `-cpu` 調整)
- [ ] 非対応時: タイマージッタ+受信パケットタイミングをエントロピー源に
      ChaCha20ベースのCSPRNGをシード
- [ ] TCPのISN生成もCSPRNGに差し替え

### 2-2. 暗号プリミティブ(ハイブリッド方針)

- 自作する(学習価値が高い、既習領域): x25519 ECDH
- クレートを使う(no_std対応のRustCrypto系): sha2, aes-gcm(または chacha20poly1305), ed25519-dalek
- 注意: タイミング攻撃耐性は趣味実装として過度に追求しない(方針として明記)

### 2-3. SSHプロトコル(RFC 4253系)

- [ ] トランスポート層: バージョン交換、バイナリパケットプロトコル、
      KEX(curve25519-sha256)、ホスト鍵(ssh-ed25519)、暗号化開始(NEWKEYS)
- [ ] userauth: password認証のみで可(publickeyは任意)
- [ ] connection: session channel 1本、shell要求に対して簡易対話(コマンドエコー程度から)
- [ ] 検証: OpenSSHクライアント `ssh -vvv` の出力を突き合わせながら進める

### 中間ゴール②: 手元のターミナルから自作OSへsshして対話できる

ここで一度立ち止まり、フェーズ3の選択を再判断する。

---

## フェーズ3: 選択肢(sshできた後に再判断)

- A. TLS 1.3サーバ(20〜35h): 1-RTT、TLS_AES_128_GCM_SHA256 + x25519のみ、
  自己署名証明書をDERで静的埋め込み(ASN.1パーサは書かない)。TLS 1.2はやらない
- B. xHCI再開(残り40〜55h): 下記の既知バグ修正から着手。
  汎用OS路線ではUSB入力は正当な投資(デスクトップOSとしての完成度に直結)
- C. Unix Like基盤の着工(長期・数百時間規模): ring3ユーザーモード、syscall、
  プロセス/スケジューラ、ELFローダ、FS(FAT read/writeから)。
  着手順の推奨: ring3+syscall1本 → コンテキストスイッチ → 協調→プリエンプティブ
  → ELFロード → FS。xv6のコードリーディングを並走させると効率的
- (参考)64bit化(30〜50h): 汎用OS路線でも長期的には価値があるが、
  32bitのまま B/C を先に進めて問題ない。急がない

---

## xHCI 既知バグ(フェーズ3-Bで最初に修正)

src/drivers/usb/xhci.rs:

1. 【致命的】DCBAAのライフタイム: `device_ctx_base_address_array` と `device_array`
   が関数ローカルの DmaBox のため、`initialize_xhci()` を抜けるとdropされ、
   xHCに設定したDCBAAポインタがdanglingになる。static + Once または意図的leakで恒久化する
2. `calc_page_size` 内のループで `let num_of_least_significant_bit = i;` が
   シャドーイングになっており外側の変数が更新されない(常にbit 0扱い)
3. `initialize_xhci()` が常に `Err(())` を返す(成功パスでも)

現状の到達点: Capabilityレジスタ読み出し、リセットシーケンス、CONFIG設定、
Scratchpad/DCBAA構築試行まで。Command Ring / Event Ring / TRB発行 / ポート処理 /
Descriptor取得 / HID は未着手。

---

## 作業上の約束

- 各フェーズ開始時にブランチを切る
- コミットは動作単位で小さく(受け入れ基準を満たしたらコミット)
- デバッグ出力は原則シリアルへ(フェーズ0-1完了後)
- 大きな設計判断(クレート分割、暗号方針の変更等)はPLAN.mdを更新してから実装
- 週5時間ペース想定。フェーズ0〜2の完了目安は3〜4ヶ月
