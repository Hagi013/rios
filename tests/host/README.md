# rios ホストリグレッションテスト

カーネルのうちハードウェアに依存しない純粋ロジックを、ホスト (std) 環境の
stable Rust でコンパイルして回帰テストするためのクレートです。
nightly-2023-06-05 / cargo xbuild / QEMU は不要です。

## 実行方法

リポジトリルートから:

```sh
make test-host
```

またはこのディレクトリで:

```sh
cargo test
```

(ルートの `.cargo/config.toml` がカーネル用カスタムターゲットを強制するため、
このディレクトリの `.cargo/config.toml` で x86_64 ホストターゲットに戻しています。
x86_64 以外のホストでは `cargo test --target $(rustc -vV | awk '/^host/{print $2}')`
のようにホストターゲットを明示してください。)

## 仕組み

カーネルのソースファイルを **一切変更せず** `#[path]` 属性でこのクレートの
モジュールツリーに取り込み、`tests/` 以下の統合テストから検証します。

| テスト対象 (カーネル側) | テストファイル | 内容 |
| --- | --- | --- |
| `src/sync/queue.rs` (`SimpleQueue`) | `tests/queue.rs` | キーボード/マウス割り込みキューの FIFO 順序・容量制限・リングバッファ折り返し・`add`/`remove`/`clear` |
| `src/util/linked_list.rs` (`LinkedList`) | `tests/linked_list.rs` | ウィンドウマネージャで使う連結リストの追加・削除・並び替え (`change_order`)・検索 |
| `src/drivers/net/net_util.rs` | `tests/net_util.rs` | ネットワークスタック全体で使うエンディアン変換・構造体のバイト列化・Vec 連結 |
| `src/util/bit_util.rs` | `tests/bit_util.rs` | ビットシフトヘルパ (private 関数のため `include!` で取り込み) |

`sync/queue.rs` は `spin::mutex::Mutex` を import しているため、
`src/spin/mutex.rs` に型解決のためだけの最小スタブを置いています
(カーネルの本物の Mutex は画面出力に依存していてホストでは使えないため)。

## 注意

- テストは「現状の正しい挙動」を固定するものです。カーネルのロジックを
  変更してテストが落ちた場合、意図した仕様変更であればテスト側を更新して
  ください。
- エンディアンに依存するテスト (`any_as_u8_slice` など) は、カーネルの
  ターゲット (i686) とホスト (x86_64) がどちらもリトルエンディアンである
  ことを前提にしています。
- ハードウェア依存のモジュール (e1000, xHCI, PIC, paging, graphic など) は
  ここでは対象外です。従来どおり `make qemu` での実機確認が必要です。
