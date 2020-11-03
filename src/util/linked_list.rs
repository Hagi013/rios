use alloc::string::{String, ToString};
use alloc::boxed;
use core::mem::replace;
use core::fmt::Debug;
use alloc::borrow::ToOwned;

#[derive(Copy, Clone, Debug)]
struct LinkedListNode<T>
    where
        T: Copy,
        T: PartialEq,
        T: Debug,
{
    prev: Option<*mut LinkedListNode<T>>,
    next: Option<*mut LinkedListNode<T>>,
    data: Option<*mut T>,
}

impl<T> LinkedListNode<T>
    where
        T: Copy,
        T: PartialEq,
        T: Debug,
{
    pub fn new(data: *mut T) -> Self {
        LinkedListNode {
            prev: None,
            next: None,
            data: Some(data),
        }
    }
}

#[derive(Debug)]
pub struct LinkedList<T>
    where
        T: Copy,
        T: PartialEq,
        T: Debug,
{
    head: Option<*mut LinkedListNode<T>>,
    tail: Option<*mut LinkedListNode<T>>,
    count: usize,
}

impl<T> LinkedList<T>
    where
        T: Copy,
        T: PartialEq,
        T: Debug,
{
    pub fn new() -> Self {
        LinkedList {
            head: None,
            tail: None,
            count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn push_front(&mut self, data: *mut T) -> Result<(), String> {
        unsafe {
            let node: *mut LinkedListNode<T> = boxed::Box::into_raw(boxed::Box::new(LinkedListNode::new(data)));
            if self.len() == 0 {
                self.head = Some(node);
                self.tail = Some(node);
                (*node).prev = None;
                (*node).next = None;
            } else {
                let head = self.head.ok_or("LinkedList's head is none.".to_owned())?;
                self.head = Some(node);
                (*head).prev = Some(node);
                (*node).next = Some(head);
                (*node).prev = None;
            }
            self.count += 1;
            return Ok(());
        }
    }

    // addするときは一番最後に入れる
    pub fn add(&mut self, data: *mut T) -> Result<(), String> {
        unsafe {
            let node: *mut LinkedListNode<T> = boxed::Box::into_raw(boxed::Box::new(LinkedListNode::new(data)));
            if self.len() == 0 {
                self.head = Some(node);
                self.tail = Some(node);
                (*node).prev = None;
                (*node).next = None;
            } else {
                let tail = self.tail.ok_or("LinkedList's tail is none.".to_owned())?;
                self.tail = Some(node);
                (*tail).next = Some(node);
                (*node).prev = Some(tail);
                (*node).next = None;
            }
            self.count += 1;
            return Ok(());
        }
    }

    pub fn remove(&mut self, data: *mut T) -> Result<(), String> {
        if self.len() == 0 {
            return Err("LinkedList length is 0.".to_owned());
        }

        if self.get_position_from_data(data).is_none() {
            return Err("Data is not existing.".to_owned());
        }

        self.count -= 1;
        // 残った最後の一つの要素だった場合
        if self.count == 0 {
            self.head = None;
            self.tail = None;
            return Ok(());
        }

        unsafe {
            let pointer: *mut LinkedListNode<T> = self.get_pointer_from_data(data).ok_or("data is not existing in LinkedList.".to_owned())?;

            if self.head.is_some() && self.tail.is_some() { // 基本的に要素が存在する場合は、headとtailは存在するはず
                let head: *mut LinkedListNode<T> = self.head.unwrap();
                let tail: *mut LinkedListNode<T> = self.tail.unwrap();

                if pointer != head && pointer != tail { // headとtailの要素が今回の削除対象ではない場合
                    if let Some(prev) = (*pointer).prev {
                        replace(&mut (*prev).next, (*pointer).next);
                    }
                    if let Some(next) = (*pointer).next {
                        replace(&mut (*next).prev, (*pointer).prev);
                    }
                    (*pointer).prev = None;
                    (*pointer).next = None;
                } else if pointer == head { // headが削除対象の場合
                    replace(&mut self.head, replace(&mut (*pointer).next, None));
                    replace(&mut (*self.head.unwrap()).prev, None);
                } else if pointer == tail { // tailが削除対象の場合
                    replace(&mut self.tail, replace(&mut (*pointer).prev, None));
                    replace(&mut (*self.tail.unwrap()).next, None);
                } else {
                    self.head = None;
                    self.tail = None;
                }
                return Ok(());
            } else {
                return Err("Element in LinkedList is null.".to_owned());
            }
        }
    }

    pub fn update(&mut self, data: *mut T) -> Result<(), String> {
        let index = self.get_position_from_data(data).ok_or("Data is not existing in linked_list, when invoke get_position_from_data in update.".to_owned())?;
        let target_node = self.get_pointer_from_index(index).ok_or("Data is not existing in linked_list, when invoke get_pointer_from_index in update.")?;
//        unsafe { (*target_node).data = Some(data); }
        unsafe { replace(&mut (*target_node).data, Some(data)); }
        Ok(())
    }

    pub fn change_order(&mut self, data: *mut T, idx: usize) -> Result<(), String> {
        let src_node_ptr: *mut LinkedListNode<T> = self.get_pointer_from_data(data).ok_or("data is not existing in LinkedList".to_owned())?;
        let position = self.get_position_from_data(data).ok_or("In chane_order, data's position is not found.".to_owned())?;
        // println!("{:?}", unsafe { (&mut *src_node_ptr) });
        let order: usize =
            if idx >= self.len() { self.len() - 1 }
            else { idx };

        unsafe {
            // はじめに対象のNodeの前後の紐付きを更新する
            if let Some(prev) = (*src_node_ptr).prev {
                replace(&mut (*prev).next,(*src_node_ptr).next);
            }
            if let Some(next) = (*src_node_ptr).next {
                replace(&mut (*next).prev, (*src_node_ptr).prev);
            }
            return if order == 0 { // headになる場合
                if let Some(head_node_ptr) = self.head {
                    if position == self.len() - 1 { // ポジションを変更したいNodeの元の位置がtailだった場合
                        self.tail = (*src_node_ptr).prev;
                    }
                    (*head_node_ptr).prev = Some(src_node_ptr);
                    (*src_node_ptr).next = Some(head_node_ptr);
                    (*src_node_ptr).prev = None;
                    self.head = Some(src_node_ptr);
                    Ok(())
                } else {
                    // もしheadが存在しない場合は、追加する(エラーとして終了でも良いが、、)
                    self.add(data)
                }
            } else if order == self.len() - 1 { // tailになる場合
                if position == 0 { // ポジションを変更したいNodeの元の位置がheadだった場合
                    self.head = (*src_node_ptr).next;
                }
                let tail_node_ptr: *mut LinkedListNode<T> = self.tail.ok_or("LinkedList is broken.".to_owned())?;
                (*tail_node_ptr).next = Some(src_node_ptr);
                (*src_node_ptr).prev = Some(tail_node_ptr);
                (*src_node_ptr).next = None;
                self.tail = Some(src_node_ptr);
                Ok(())
            } else {
                // 挿入したい場所における入れ替え操作
                // 1. 入れ替えたい先のNodeとその前の順番のNodeを取得
                let dest_order_node_ptr: *mut LinkedListNode<T> = self.get_pointer_from_index(order)
                    .ok_or("idx argument to change_order may be out of bound.".to_owned())?;
                let dest_order_prev_node_ptr = (*dest_order_node_ptr).prev
                    .ok_or("change_order target order node's state is broken in LinkedList.".to_owned())?;

                // 2. 入れ替えたいNodeの紐付けを行う
                (*dest_order_node_ptr).prev = Some(src_node_ptr);
                (*dest_order_prev_node_ptr).next = Some(src_node_ptr);

                // 3. 入れ替えたいNodeの元の位置がheadもしくはtailだった場合の処置
                if position == self.len() - 1 { // ポジションを変更したいNodeの元の位置がtailだった場合
                    self.tail = (*src_node_ptr).prev;
                } else if position == 0 { // ポジションを変更したいNodeの元の位置がheadだった場合
                    self.head = (*src_node_ptr).next;
                }

                // 4. 入れ替えたいNodeの前後の紐付けを行う
                (*src_node_ptr).next = Some(dest_order_node_ptr);
                (*src_node_ptr).prev = Some(dest_order_prev_node_ptr);
                Ok(())
            }
        }
    }

    pub fn get_position_from_data(&self, data: *mut T) -> Option<usize> {
        if self.len() == 0 { return None; }
        let mut idx: usize = 0;
        let mut node: *mut LinkedListNode<T> = self.head.or(None)?;
        loop {
            unsafe {
                if (*node).data.is_none() { break; }
                if *data == (*((*node).data.unwrap())) {
                    return Some(idx);
                }
                if let Some(next) = (*node).next {
                    idx += 1;
                    node = next;
                } else {
                    break;
                }
            }
        }
        return None;
    }

    pub fn get_data_from_position(&self, idx: usize) -> Option<*mut T> {
        if idx >= self.count {
            return None;
        }
        let data_ptr: *mut LinkedListNode<T> = self.get_pointer_from_index(idx).or(None)?;
        return unsafe { (*data_ptr).data };
    }

    pub fn get_next_data(&self, data: *mut T) -> Option<*mut T> {
        match self.get_position_from_data(data) {
            Some(idx) => {
                if idx + 1 >= self.len() {
                    return None;
                }
                let next_data_ptr: *mut LinkedListNode<T> = self.get_pointer_from_index(idx + 1).or(None)?;
                return unsafe { (*next_data_ptr).data };
            },
            None => None,
        }
    }

    pub fn get_prev_data(&self, data: *mut T) -> Option<*mut T> {
        match self.get_position_from_data(data) {
            Some(idx) => {
                if idx <= 0 { return None; }
                let prev_data_ptr: *mut LinkedListNode<T> = self.get_pointer_from_index(idx - 1).or(None)?;
                return unsafe { (*prev_data_ptr).data };
            },
            None => None,
        }
    }

    fn get_pointer_from_data(&self, data: *mut T) -> Option<*mut LinkedListNode<T>> {
        if self.len() == 0 { return None }
        let mut node: *mut LinkedListNode<T>  = self.head.or(None)?;
        loop {
            unsafe {
                if (*node).data.is_none() { break; }
                if *data == (*(*node).data.unwrap()) {
                    return Some(node);
                }
                if let Some(next) = (*node).next {
                    node = next;
                } else {
                    break;
                }
            }
        }
        return None;
    }

    fn get_pointer_from_index(&self, idx: usize) -> Option<*mut LinkedListNode<T>> {
        if idx >= self.len() { return None; }
        let mut node: *mut LinkedListNode<T> = self.head.or(None)?;

        for _i in 0..idx {
            unsafe {
                if let Some(next) = (*node).next {
                    node = next;
                } else {
                    return None;
                }
            }
        }
        return Some(node)
    }
}

//#[cfg(test)]
//mod test {
//    use self::util::linked_list::LinkedList;
//
//    #[test]
//    fn new() {
//        let ll: LinkedList<i32> = LinkedList::new();
//        assert_eq!(ll.len(), 0);
//    }
//}
