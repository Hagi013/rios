//! sync/queue.rs (SimpleQueue) のリグレッションテスト。
//! SimpleQueue はキーボード/マウス割り込みハンドラと main loop の間の
//! データ受け渡しに使われている。特に new()/clear() のゼロ初期化は
//! opt-level=3 で未定義動作により初期化ごと消えた実績があるため
//! (queue.rs のコメント参照)、空キューとして機能することを固定する。

use rios_host_tests::sync::queue::SimpleQueue;

// queue.rs の CAPACITY (private) と同じ値
const CAPACITY: usize = 30;

#[test]
fn new_queue_is_empty_and_functional() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    assert_eq!(q.len(), 0);
    assert!(q.is_empty());
    assert!(!q.is_existing());
    assert_eq!(q.dequeue(), None);
}

#[test]
fn enqueue_dequeue_is_fifo() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in 1..=5u32 {
        q.enqueue(v);
    }
    assert_eq!(q.len(), 5);
    assert!(q.is_existing());
    for v in 1..=5u32 {
        assert_eq!(q.dequeue(), Some(v));
    }
    assert_eq!(q.dequeue(), None);
    assert!(q.is_empty());
}

#[test]
fn enqueue_beyond_capacity_is_dropped() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in 0..(CAPACITY as u32 + 10) {
        q.enqueue(v);
    }
    // 容量 (30) を超えた分は捨てられる
    assert_eq!(q.len(), CAPACITY);
    for v in 0..CAPACITY as u32 {
        assert_eq!(q.dequeue(), Some(v));
    }
    assert_eq!(q.dequeue(), None);
}

#[test]
fn fifo_order_survives_ring_wraparound() {
    // 割り込みキューは長時間動作でリングバッファが何周もするので、
    // インデックスの折り返しをまたいでも FIFO 順が保たれることを確認する
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    let mut next_in: u32 = 0;
    let mut next_out: u32 = 0;

    for _round in 0..20 {
        // 7個入れて7個出す、を繰り返して折り返し位置をずらす
        for _ in 0..7 {
            q.enqueue(next_in);
            next_in += 1;
        }
        for _ in 0..7 {
            assert_eq!(q.dequeue(), Some(next_out));
            next_out += 1;
        }
        assert!(q.is_empty());
    }
}

#[test]
fn interleaved_enqueue_dequeue_keeps_order() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    let mut next_in: u32 = 0;
    let mut next_out: u32 = 0;

    // 常に数個の要素を残したまま出し入れする
    for _ in 0..5 {
        q.enqueue(next_in);
        next_in += 1;
    }
    for _ in 0..100 {
        q.enqueue(next_in);
        next_in += 1;
        assert_eq!(q.dequeue(), Some(next_out));
        next_out += 1;
        assert_eq!(q.len(), 5);
    }
}

#[test]
fn get_returns_logical_index_before_wraparound() {
    // head が先頭 (0) のままの状態での get() の挙動を固定する
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [10u32, 20, 30] {
        q.enqueue(v);
    }
    assert_eq!(q.get(0), Some(10));
    assert_eq!(q.get(1), Some(20));
    assert_eq!(q.get(2), Some(30));
    assert_eq!(q.get(3), None);
}

#[test]
fn get_position_in_queue_finds_data() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    assert!(q.get_position_in_queue(1).is_err());
    for v in [10u32, 20, 30] {
        q.enqueue(v);
    }
    assert_eq!(q.get_position_in_queue(10), Ok(0));
    assert_eq!(q.get_position_in_queue(30), Ok(2));
    assert!(q.get_position_in_queue(99).is_err());
}

#[test]
fn add_inserts_at_logical_index() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3] {
        q.enqueue(v);
    }
    q.add(9, 1);
    assert_eq!(q.len(), 4);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(9));
    assert_eq!(q.dequeue(), Some(2));
    assert_eq!(q.dequeue(), Some(3));
}

#[test]
fn add_at_len_behaves_like_enqueue() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    q.enqueue(1);
    q.add(2, 1);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
}

#[test]
fn remove_middle_element_shifts_rest() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3, 4] {
        q.enqueue(v);
    }
    assert_eq!(q.remove(1), Some(2));
    assert_eq!(q.len(), 3);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(3));
    assert_eq!(q.dequeue(), Some(4));
}

#[test]
fn remove_first_element_behaves_like_dequeue() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3] {
        q.enqueue(v);
    }
    assert_eq!(q.remove(0), Some(1));
    assert_eq!(q.dequeue(), Some(2));
}

#[test]
fn remove_last_element() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3] {
        q.enqueue(v);
    }
    assert_eq!(q.remove(2), Some(3));
    assert_eq!(q.len(), 2);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
    assert_eq!(q.dequeue(), None);
}

#[test]
fn remove_from_empty_returns_none() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    assert_eq!(q.remove(0), None);
}

#[test]
fn remove_entry_removes_matching_data() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3] {
        q.enqueue(v);
    }
    assert!(q.remove_entry(2).is_ok());
    assert_eq!(q.len(), 2);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(3));
    // 存在しないデータの削除はエラー
    let mut q2: SimpleQueue<u32> = SimpleQueue::new();
    q2.enqueue(1);
    assert!(q2.remove_entry(99).is_err());
}

#[test]
fn update_replaces_matching_data() {
    // T が値比較のため update は同値要素の置き換え (実質 no-op) だが、
    // 対象が存在するときに Ok が返ることを固定する
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3] {
        q.enqueue(v);
    }
    assert!(q.update(2).is_ok());
    assert!(q.update(99).is_err());
}

#[test]
fn clear_resets_queue() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    for v in [1u32, 2, 3] {
        q.enqueue(v);
    }
    q.clear();
    assert_eq!(q.len(), 0);
    assert!(q.is_empty());
    assert_eq!(q.dequeue(), None);
    // clear 後も普通に使える
    q.enqueue(42);
    assert_eq!(q.dequeue(), Some(42));
}

#[test]
fn enqueue_front_on_empty_queue() {
    let mut q: SimpleQueue<u32> = SimpleQueue::new();
    q.enqueue_front(7);
    assert_eq!(q.len(), 1);
    assert_eq!(q.dequeue(), Some(7));
}
