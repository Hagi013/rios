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
            queue: [unsafe { *(0 as *mut T) }; LIMIT],
        }
    }

    pub fn enqueue(&mut self, data: T) {
        if self.len() == LIMIT { return; }

        self.count += 1;
        self.queue[self.tail] = data;

        if SimpleQueue::<T>::check_index_limit(self.tail) {
            self.tail += 1;
        } else {
            self.tail = INITIAL_INDEX;
        }
    }

    pub fn enqueue_front(&mut self, data: T) {
        self.add(data, 0);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        if self.len() <= 0 { return None }

        self.count -= 1;
        let data = Some(self.queue[self.head]);

        if SimpleQueue::<T>::check_index_limit(self.head) {
            self.head += 1;
        } else {
            self.head = INITIAL_INDEX;
        }
        data
    }

    pub fn dequeue_front(&mut self, data: T) -> Option<T> {
        return self.remove(0);
    }

    pub fn add(&mut self, data: T, index: usize) {
        if self.len() == LIMIT { return; }

        // 一番最後に追加したい場合
        if index == self.len() {
            self.enqueue(data);
            return;
        }

        self.count += 1;
        let rest_count = self.len() - index;
        for i in 0..rest_count {
            let mut real_idx = (self.len() - i) - 1;
            // 配列が循環している状態でも正しく値を取得できるように
            if self.head + real_idx >= LIMIT - 1 {
                real_idx = real_idx - (LIMIT - 1 - self.head) + INITIAL_INDEX;
            }

            let mut real_idx_minus_one = real_idx - 1;
            if self.head + real_idx_minus_one >= LIMIT - 1 {
                real_idx_minus_one = real_idx_minus_one - (LIMIT - 1 - self.head) + INITIAL_INDEX;
            }

            self.queue[real_idx] = self.queue[real_idx_minus_one];
        }
        // 値のセット
        let mut target_idx = index;
        if self.head + target_idx >= LIMIT - 1 {
            target_idx = target_idx - (LIMIT - 1 - self.head) + INITIAL_INDEX;
        }
        self.queue[target_idx] = data;
    }

    pub fn remove(&mut self, mut index: usize) -> Option<T> {
        if self.len() <= 0 { return None }

        self.count -= 1;

        // 0番目を取得する場合はdequeueして終わり
        if index == 0 {
            return self.dequeue();
        }

        // 1番最後の要素を取得する場合も取得して、tailから1引けば終わり
        // すでにcountがマイナスされているので、条件を `index == self.len()` としている
        if index == self.len() {
            self.tail -= 1;
            // 配列が循環している状態でも正しく値を取得できるように
            if self.head + index >= LIMIT - 1 {
                index = index - (LIMIT - 1 - self.head) + INITIAL_INDEX;
            }
            return Some(self.queue[index]);
        }

        let mut real_idx: usize = index;
        if self.head + index >= LIMIT - 1 {
            real_idx = index - (LIMIT - 1 - self.head) + INITIAL_INDEX;
        }
        // 値の取得
        let data = Some(self.queue[real_idx]);

        // 端じゃない要素の場合は並び替えが必要になる
        for i in index..self.len() {
            let mut target_idx = self.head + i;
            if (self.head + target_idx) >= (LIMIT - 1) {
                target_idx = index - (LIMIT - 1 - self.head) + INITIAL_INDEX
            };

            let mut target_idx_plus_one = target_idx + 1;
            if (self.head + target_idx_plus_one) >= (LIMIT - 1) {
                target_idx_plus_one = index - (LIMIT - 1 - self.head) + INITIAL_INDEX
            };
            self.queue[target_idx] = self.queue[target_idx_plus_one];
        }
        self.tail -= 1;

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

    fn get_index_in_queue(&self, idx: usize) -> usize {
        let mut i = idx;
        if idx + self.head > LIMIT - 1 {
            i = i - (LIMIT - 1 - self.head) + INITIAL_INDEX;
        }
        return i;
    }

    fn check_index_limit(index: usize) -> bool {
        index < LIMIT - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_existing(&self) -> bool {
        self.len() > 0
    }

    pub fn clear(&mut self) {
        self.queue = [unsafe { *(0 as *mut T) }; LIMIT];
        self.head = INITIAL_INDEX;
        self.tail = INITIAL_INDEX;
        self.count = 0;
    }
}
