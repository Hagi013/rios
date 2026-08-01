//! ホスト (std) 環境でカーネルの純粋ロジックをコンパイルし、
//! リグレッションテストを実行するためのクレート。
//!
//! カーネル本体は nightly-2023-06-05 + cargo xbuild + QEMU が必要だが、
//! ここではハードウェアに依存しないモジュール
//! (sync::queue / util::linked_list / drivers::net::net_util) を
//! `#[path]` で直接取り込み、stable Rust の `cargo test` で検証する。
//!
//! カーネル側のソースは一切変更せずに取り込んでいるため、
//! カーネル固有の書き方に起因する警告はクレート単位で抑制する。
#![allow(
    dead_code,
    unused_imports,
    unused_unsafe,
    unused_variables,
    unused_mut,
    unused_comparisons,
    unused_must_use
)]

extern crate alloc;

// sync::queue が `super::super::spin::mutex::Mutex` を import するため、
// カーネルと同じモジュール階層を再現する (spin はスタブ)。
pub mod spin;
pub mod sync;
pub mod util;

#[path = "../../../src/drivers/net/net_util.rs"]
pub mod net_util;
