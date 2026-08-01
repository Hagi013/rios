//! drivers/net/net_util.rs のリグレッションテスト。
//! エンディアン変換はネットワークスタック全体 (ARP/IP/ICMP/UDP/DHCP) の
//! ヘッダ組み立てで使われているため、挙動が変わるとパケットが壊れる。

use rios_host_tests::net_util::{
    any_as_u8_slice, any_as_u8_vec, push_to_vec, switch_endian16, switch_endian32,
};

#[test]
fn switch_endian32_swaps_byte_order() {
    // net_util.rs のコメントに書かれている仕様そのもの
    assert_eq!(switch_endian32(0xee11ff22), 0x22ff11ee);
    assert_eq!(switch_endian32(0x12345678), 0x78563412);
    assert_eq!(switch_endian32(0x00000000), 0x00000000);
    assert_eq!(switch_endian32(0xffffffff), 0xffffffff);
    assert_eq!(switch_endian32(0x000000ff), 0xff000000);
}

#[test]
fn switch_endian32_is_involution() {
    // 2回適用すると元に戻る
    for v in [0u32, 1, 0xdeadbeef, 0x80000001, u32::MAX] {
        assert_eq!(switch_endian32(switch_endian32(v)), v);
    }
}

#[test]
fn switch_endian32_matches_swap_bytes() {
    for v in [0u32, 0xee11ff22, 0x0a00020f, u32::MAX] {
        assert_eq!(switch_endian32(v), v.swap_bytes());
    }
}

#[test]
fn switch_endian16_swaps_byte_order() {
    assert_eq!(switch_endian16(0xff22), 0x22ff);
    assert_eq!(switch_endian16(0x1234), 0x3412);
    assert_eq!(switch_endian16(0x0000), 0x0000);
    assert_eq!(switch_endian16(0x00ff), 0xff00);
}

#[test]
fn switch_endian16_is_involution() {
    for v in [0u16, 1, 0x0806, 0x0800, u16::MAX] {
        assert_eq!(switch_endian16(switch_endian16(v)), v);
        assert_eq!(switch_endian16(v), v.swap_bytes());
    }
}

#[test]
fn any_as_u8_slice_reads_native_layout() {
    // x86 (LE) 前提でヘッダをそのままバイト列化している。
    // カーネルのターゲット (i686) もホスト (x86_64) もリトルエンディアン。
    let v: u32 = 0x11223344;
    assert_eq!(any_as_u8_slice(&v), &v.to_ne_bytes());

    #[repr(C)]
    struct Hdr {
        a: u8,
        b: u8,
        c: u16,
    }
    let h = Hdr { a: 0xaa, b: 0xbb, c: 0x1234 };
    let bytes = any_as_u8_slice(&h);
    assert_eq!(bytes.len(), core::mem::size_of::<Hdr>());
    assert_eq!(bytes[0], 0xaa);
    assert_eq!(bytes[1], 0xbb);
    assert_eq!(&bytes[2..4], &0x1234u16.to_ne_bytes());
}

#[test]
fn any_as_u8_vec_equals_slice() {
    let v: u64 = 0x0102030405060708;
    assert_eq!(any_as_u8_vec(&v).as_slice(), any_as_u8_slice(&v));
}

#[test]
fn push_to_vec_appends_src_after_dst() {
    // 引数順に注意: push_to_vec(src, dst) は dst の末尾に src を足して返す
    let src = vec![1u8, 2, 3];
    let dst = vec![9u8, 8];
    assert_eq!(push_to_vec(src, dst), vec![9, 8, 1, 2, 3]);
}

#[test]
fn push_to_vec_with_empty_operands() {
    assert_eq!(push_to_vec(vec![], vec![1u8, 2]), vec![1, 2]);
    assert_eq!(push_to_vec(vec![1u8, 2], vec![]), vec![1, 2]);
    assert_eq!(push_to_vec(vec![], Vec::<u8>::new()), Vec::<u8>::new());
}
