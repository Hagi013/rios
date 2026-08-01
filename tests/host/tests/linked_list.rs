//! util/linked_list.rs のリグレッションテスト。
//! LinkedList はウィンドウマネージャ等で使われる侵入的リスト。
//! データは *mut T で保持され、検索は値比較 (*data == *node.data) で行われる。

use rios_host_tests::util::linked_list::LinkedList;

/// テスト用のデータ領域。Box でヒープに置いてポインタを渡す。
struct Data {
    vals: Vec<Box<i32>>,
}

impl Data {
    fn new(vals: &[i32]) -> Self {
        Data {
            vals: vals.iter().map(|v| Box::new(*v)).collect(),
        }
    }
    fn ptr(&mut self, idx: usize) -> *mut i32 {
        &mut *self.vals[idx] as *mut i32
    }
}

fn collect(ll: &LinkedList<i32>) -> Vec<i32> {
    (0..ll.len())
        .map(|i| unsafe { *ll.get_data_from_position(i).unwrap() })
        .collect()
}

#[test]
fn new_list_is_empty() {
    let ll: LinkedList<i32> = LinkedList::new();
    assert_eq!(ll.len(), 0);
    assert_eq!(ll.get_data_from_position(0), None);
}

#[test]
fn add_appends_to_tail() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    assert_eq!(ll.len(), 3);
    assert_eq!(collect(&ll), vec![1, 2, 3]);
}

#[test]
fn push_front_prepends_to_head() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.push_front(d.ptr(i)).unwrap();
    }
    assert_eq!(collect(&ll), vec![3, 2, 1]);
}

#[test]
fn get_position_from_data_finds_by_value() {
    let mut d = Data::new(&[10, 20, 30]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    assert_eq!(ll.get_position_from_data(d.ptr(0)), Some(0));
    assert_eq!(ll.get_position_from_data(d.ptr(2)), Some(2));
    let mut other = Box::new(99);
    assert_eq!(ll.get_position_from_data(&mut *other as *mut i32), None);
}

#[test]
fn get_next_and_prev_data() {
    let mut d = Data::new(&[10, 20, 30]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    unsafe {
        assert_eq!(*ll.get_next_data(d.ptr(0)).unwrap(), 20);
        assert_eq!(*ll.get_prev_data(d.ptr(2)).unwrap(), 20);
    }
    assert_eq!(ll.get_next_data(d.ptr(2)), None); // tail の次は無い
    assert_eq!(ll.get_prev_data(d.ptr(0)), None); // head の前は無い
}

#[test]
fn remove_middle_element() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    ll.remove(d.ptr(1)).unwrap();
    assert_eq!(ll.len(), 2);
    assert_eq!(collect(&ll), vec![1, 3]);
}

#[test]
fn remove_head_element() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    ll.remove(d.ptr(0)).unwrap();
    assert_eq!(collect(&ll), vec![2, 3]);
}

#[test]
fn remove_tail_element() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    ll.remove(d.ptr(2)).unwrap();
    assert_eq!(collect(&ll), vec![1, 2]);
}

#[test]
fn remove_last_remaining_element_empties_list() {
    let mut d = Data::new(&[1]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    ll.add(d.ptr(0)).unwrap();
    ll.remove(d.ptr(0)).unwrap();
    assert_eq!(ll.len(), 0);
    assert_eq!(ll.get_data_from_position(0), None);
}

#[test]
fn remove_from_empty_list_is_error() {
    let mut other = Box::new(1);
    let mut ll: LinkedList<i32> = LinkedList::new();
    assert!(ll.remove(&mut *other as *mut i32).is_err());
}

#[test]
fn remove_nonexistent_data_is_error() {
    let mut d = Data::new(&[1]);
    let mut other = Box::new(99);
    let mut ll: LinkedList<i32> = LinkedList::new();
    ll.add(d.ptr(0)).unwrap();
    assert!(ll.remove(&mut *other as *mut i32).is_err());
    assert_eq!(ll.len(), 1);
}

#[test]
fn change_order_moves_element_to_head() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    ll.change_order(d.ptr(2), 0).unwrap();
    assert_eq!(collect(&ll), vec![3, 1, 2]);
}

#[test]
fn change_order_moves_element_to_tail() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    ll.change_order(d.ptr(0), 2).unwrap();
    assert_eq!(collect(&ll), vec![2, 3, 1]);
}

#[test]
fn change_order_with_out_of_bound_index_moves_to_tail() {
    // idx >= len の場合は末尾扱い (change_order 内で clamp される)
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    ll.change_order(d.ptr(0), 100).unwrap();
    assert_eq!(collect(&ll), vec![2, 3, 1]);
}

#[test]
fn update_replaces_data_pointer() {
    let mut d = Data::new(&[1, 2, 3]);
    let mut ll: LinkedList<i32> = LinkedList::new();
    for i in 0..3 {
        ll.add(d.ptr(i)).unwrap();
    }
    // 値 2 のノードを、同じ値 2 を持つ別のポインタに差し替える
    let mut replacement = Box::new(2);
    ll.update(&mut *replacement as *mut i32).unwrap();
    assert_eq!(
        ll.get_data_from_position(1),
        Some(&mut *replacement as *mut i32)
    );
    // 存在しない値の update はエラー
    let mut other = Box::new(99);
    assert!(ll.update(&mut *other as *mut i32).is_err());
}
