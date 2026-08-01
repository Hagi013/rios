//! util/bit_util.rs のリグレッションテスト。
//! 関数が private のため、include! でこのテストクレート内に取り込んで検証する。
#![allow(dead_code)]

include!("../../../src/util/bit_util.rs");

#[test]
fn shift_left_shifts_bits() {
    assert_eq!(shift_left(0b1, 3), 0b1000);
    assert_eq!(shift_left(0xffff_ffff, 0), 0xffff_ffff);
    assert_eq!(shift_left(0x8000_0000, 1), 0); // 最上位ビットは溢れて消える
}

#[test]
fn shift_right_shifts_bits() {
    assert_eq!(shift_right(0b1000, 3), 0b1);
    assert_eq!(shift_right(0xffff_ffff, 0), 0xffff_ffff);
    assert_eq!(shift_right(0x1, 1), 0); // 最下位ビットは溢れて消える
}
