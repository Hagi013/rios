use alloc::collections::vec_deque::VecDeque;

use super::super::spin::mutex::Mutex;
use alloc::string::{String, ToString};
use core::cmp::PartialEq;
//use alloc::borrow::ToOwned;

// static HEAP: Mutex<Option<Heap>> = LockedHeap::empty();
//pub struct KQueue<T: ?Sized> {
//    k_queue: Mutex<Queue<T>>,
//}
//
//impl<T> KQueue<T> {
//    pub fn new() -> KQueue<T> {
//        Self {
//            k_queue: Mutex::new(Queue::new())
//        }
//        // *HEAP.lock() = unsafe { Some(Heap::new(heap_addr_start, size)) };
//    }
//}

//#[derive(Debug)]
//pub struct KQueue<T> {
//    k_queue: Mutex<VecDeque<T>>,
//}
//
//impl<T> KQueue<T> {
//    pub fn new() -> KQueue<T> {
//        KQueue {
//            k_queue: Mutex::new(VecDeque::new()),
//        }
//    }
//
//    pub fn enqueue(&mut self, data: T) {
//        self.k_queue.lock().push_back(data);
//    }
//
//    pub fn dequeue(&mut self) -> Option<T> {
//        self.k_queue.lock().pop_front()
//    }
//
//    pub fn len(&self) -> usize {
//        self.k_queue.lock().len()
//    }
//
//    pub fn is_empty(&self) -> bool {
//        self.k_queue.lock().is_empty()
//    }
//
//    pub fn is_existing(&self) -> bool {
//        !self.is_empty()
//    }
//
//    pub fn clear(&mut self) {
//        self.k_queue.lock().clear()
//    }
//}

const CAPACITY: usize = 30;
const INITIAL_INDEX: usize = 0;
const LIMIT: usize = CAPACITY - INITIAL_INDEX;

#[derive(Debug, Copy, Clone)]
pub struct SimpleQueue<T>
where
    T: Copy,
    T: PartialEq
{
    head: usize,
    tail: usize,
    count: usize,
    queue: [T; LIMIT],
}

impl<T> SimpleQueue<T>
where
    T: Copy,
    T: PartialEq
{
    pub fn new() -> Self {
        SimpleQueue {
            head: INITIAL_INDEX,
            tail: INITIAL_INDEX,
            count: 0,
            // ヌルポインタ参照(`*(0 as *mut T)`)は未定義動作。opt-level=3 では
            // LLVM がこのパスを到達不能とみなし、new() の呼び出し元(キュー初期化)ごと
            // 最適化で消されてしまい、キーボード/マウスのキューが機能しなくなる。
            // 書き込み前のスロットは読まれないが、健全性のためゼロ初期化する。
            queue: unsafe { core::mem::MaybeUninit::<[T; LIMIT]>::zeroed().assume_init() },
        }
    }

    pub fn enqueue(&mut self, data: T) {
        if self.len() == LIMIT { return; }

        self.queue[self.tail] = data;
        self.tail = (self.tail + 1) % LIMIT;
        self.count += 1;
    }

    pub fn enqueue_front(&mut self, data: T) {
        self.add(data, 0);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        if self.len() <= 0 { return None }

        let data = Some(self.queue[self.head]);
        self.head = (self.head + 1) % LIMIT;
        self.count -= 1;
        data
    }

    pub fn dequeue_front(&mut self, data: T) -> Option<T> {
        return self.remove(0);
    }

    pub fn add(&mut self, data: T, index: usize) {
        if self.len() == LIMIT { return; }

        // 一番最後(かそれ以降)に追加したい場合
        if index >= self.len() {
            self.enqueue(data);
            return;
        }

        // 挿入位置から後ろの要素を一つずつ末尾側へずらす
        let mut i = self.len();
        while i > index {
            let dst = self.get_index_in_queue(i);
            let src = self.get_index_in_queue(i - 1);
            self.queue[dst] = self.queue[src];
            i -= 1;
        }
        // 値のセット
        let target_idx = self.get_index_in_queue(index);
        self.queue[target_idx] = data;
        self.tail = (self.tail + 1) % LIMIT;
        self.count += 1;
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len() { return None }

        // 0番目を取得する場合はdequeueして終わり
        if index == 0 {
            return self.dequeue();
        }

        // 値の取得
        let data = Some(self.queue[self.get_index_in_queue(index)]);

        // 削除位置より後ろの要素を一つずつ先頭側へ詰める
        for i in index..self.len() - 1 {
            let dst = self.get_index_in_queue(i);
            let src = self.get_index_in_queue(i + 1);
            self.queue[dst] = self.queue[src];
        }
        self.tail = (self.tail + LIMIT - 1) % LIMIT;
        self.count -= 1;

        return data;
    }

    pub fn update(&mut self, data: T) -> Result<(), String> {
        let index = self.get_position_in_queue(data).or(Err("Error in update queue.".to_string()))?;
        self.queue[index] = data;
        Ok(())
    }

    pub fn remove_entry(&mut self, data: T) -> Result<(), String> {
        for idx in 0..self.len() {
            let i = self.get_index_in_queue(idx);
            if data == self.queue[i] {
                self.remove(idx);
                return Ok(());
            }
        }
        return Err("Removing Target is not existing.".to_string());
    }

    pub fn get(&self, idx: usize) -> Option<T> {
        if idx < 0 { return None; }
        if idx >= self.len() { return None; }
        let i = self.get_index_in_queue(idx);
        return Some(self.queue[i]);
    }

    pub fn get_position_in_queue(&self, data: T) -> Result<usize, String> {
        if self.len() <= 0 { return Err("queue is empty.".to_string()) }
        for i in 0..self.len() {
            if Some(data) == self.get(i) {
                return Ok(i)
            }
        }
        Err("data is not existing".to_string())
    }

    pub fn len(&self) -> usize {
        self.count
    }

    // 論理インデックス(先頭からの位置)をリングバッファの物理インデックスに変換する
    fn get_index_in_queue(&self, idx: usize) -> usize {
        (self.head + idx) % LIMIT
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_existing(&self) -> bool {
        self.len() > 0
    }

    pub fn clear(&mut self) {
        // new() と同じ理由でヌルポインタ参照を避ける。
        self.queue = unsafe { core::mem::MaybeUninit::<[T; LIMIT]>::zeroed().assume_init() };
        self.head = INITIAL_INDEX;
        self.tail = INITIAL_INDEX;
        self.count = 0;
    }
}
